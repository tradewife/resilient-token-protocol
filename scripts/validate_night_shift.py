"""
Validate night shift candidates through FutureBlindSimulator (with fees + slippage).

Takes the top candidates from the night shift report and runs them through the
full simulator on the same WFA fold structure. This is the "does the edge survive
fees and slippage?" check before paper trading.

Usage:
    python3 scripts/validate_night_shift.py
    python3 scripts/validate_night_shift.py --symbol SOL/USDT --top 3
"""
import asyncio
import argparse
import json
import os
import sys
from datetime import datetime, timedelta, timezone

import numpy as np
import pandas as pd

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from backtesting.future_blind_simulator import FutureBlindSimulator
from agents.historical_data_collector import DataWindow
from scripts.run_backtest_r2 import MultiTFStrategy


DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")
RESULTS_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "night_results")

# Production baseline
PRODUCTION_CONFIG = {
    "signal_threshold": 0.40, "min_alignment": 3, "take_profit_atr": 6.0,
    "stop_loss_atr": 2.5, "max_hold_hours": 96, "time_decay_hours": 48,
    "trailing_stop_atr": 1.0, "score_flip_delay_hrs": 2,
}

# Top candidates from night shift report 2026-04-05
NIGHT_SHIFT_CANDIDATES = {
    "SOL/USDT": [
        {"signal_threshold": 0.35, "take_profit_atr": 4.0, "stop_loss_atr": 1.25,
         "max_hold_hours": 36, "time_decay_hours": 41, "min_alignment": 3,
         "trailing_stop_atr": 0.7036, "score_flip_delay_hrs": 1, "label": "night_shift_2026-04-05_1"},
        {"signal_threshold": 0.35, "take_profit_atr": 3.9107, "stop_loss_atr": 1.25,
         "max_hold_hours": 48, "time_decay_hours": 36, "min_alignment": 3,
         "trailing_stop_atr": 0.7033, "score_flip_delay_hrs": 0, "label": "night_shift_2026-04-05_2"},
        {"signal_threshold": 0.35, "take_profit_atr": 2.9537, "stop_loss_atr": 1.25,
         "max_hold_hours": 48, "time_decay_hours": 28, "min_alignment": 3,
         "trailing_stop_atr": 0.7012, "score_flip_delay_hrs": 4, "label": "night_shift_prev"},
    ],
    "BTC/USDT": [
        {"signal_threshold": 0.45, "take_profit_atr": 3.5, "stop_loss_atr": 1.5,
         "max_hold_hours": 36, "time_decay_hours": 12, "min_alignment": 3,
         "trailing_stop_atr": 1.0, "score_flip_delay_hrs": 0, "label": "night_shift_btc_1"},
    ],
    "BNB/USDT": [
        {"signal_threshold": 0.35, "take_profit_atr": 3.0, "stop_loss_atr": 1.0,
         "max_hold_hours": 36, "time_decay_hours": 12, "min_alignment": 3,
         "trailing_stop_atr": 0.0, "score_flip_delay_hrs": 0, "label": "night_shift_bnb_1"},
    ],
}

# WFA fold structure matching night_shift.py (36-day test windows)
WFA_CONFIG = {
    "num_folds": 9,
    "test_fold_days": 36,
    "warmup_hours": 250,
}


def make_folds(total_bars: int) -> list:
    """Create expanding-window test folds matching night_shift.py."""
    test_bars = WFA_CONFIG["test_fold_days"] * 24
    warmup = WFA_CONFIG["warmup_hours"]
    num_folds = WFA_CONFIG["num_folds"]
    usable = total_bars - warmup
    actual = min(num_folds, usable // test_bars)
    bars_per = test_bars if actual < (usable // test_bars) else usable // actual
    folds = []
    start = warmup
    for i in range(actual):
        end = start + bars_per if i < actual - 1 else total_bars
        folds.append((i, start, end))
        start = end
    return folds


def compute_metrics(trips: list) -> dict:
    if not trips:
        return {"round_trips": 0, "win_rate": 0, "total_pnl_pct": 0, "avg_pnl_pct": 0,
                "sharpe": 0, "max_drawdown_pct": 0, "profit_factor": 0, "avg_hold_hrs": 0,
                "exit_reasons": {}, "final_balance": 10000}
    pnls = [r["pnl_pct"] for r in trips]
    wins = [p for p in pnls if p > 0]
    losses = [p for p in pnls if p <= 0]
    cum = np.cumsum(pnls)
    running_max = np.maximum.accumulate(cum)
    max_dd = abs(min(cum - running_max)) if len(cum) > 0 else 0
    sharpe = np.mean(pnls) / np.std(pnls) * np.sqrt(24 * 365) if np.std(pnls) > 0 else 0
    avg_win = np.mean(wins) if wins else 0
    avg_loss = abs(np.mean(losses)) if losses else 0
    pf = avg_win / avg_loss if avg_loss > 0 else float("inf")
    exit_reasons = {}
    for r in trips:
        reason = r.get("exit_reason", "signal")
        exit_reasons[reason] = exit_reasons.get(reason, 0) + 1
    return {
        "round_trips": len(trips), "win_rate": len(wins) / len(pnls),
        "total_pnl_pct": sum(pnls), "avg_pnl_pct": np.mean(pnls),
        "avg_hold_hrs": np.mean([r["hold_hrs"] for r in trips]),
        "max_drawdown_pct": max_dd, "sharpe": sharpe,
        "profit_factor": pf, "exit_reasons": exit_reasons,
        "final_balance": 10000 * (1 + sum(pnls) / 100),
    }


async def backtest_fold(symbol: str, df: pd.DataFrame, params: dict,
                         train_end: int, test_end: int) -> dict:
    """Run full simulator on a single fold window."""
    window_df = df.iloc[max(0, train_end - 250):test_end]
    if len(window_df) < 300:
        return None

    strategy = MultiTFStrategy(f"val_{symbol}", {**{"symbol": symbol}, **params})
    sim = FutureBlindSimulator(initial_capital=10000)
    sim.add_strategy(strategy)
    window = DataWindow(
        symbol=symbol, exchange="binance",
        start_time=window_df.index[0].to_pydatetime(),
        end_time=window_df.index[-1].to_pydatetime(),
        current_time=window_df.index[0].to_pydatetime(),
        data=window_df,
    )
    result = await sim.run_simulation(window, time_step_minutes=60)
    trips = strategy.completed_round_trips
    return compute_metrics(trips)


async def validate_candidate(symbol: str, params: dict, df: pd.DataFrame,
                             folds: list, label: str) -> dict:
    """Run a candidate through all WFA folds with the full simulator."""
    fold_results = []
    for fold_num, train_end, test_end in folds:
        print(f"      fold {fold_num}...", end="", flush=True)
        m = await backtest_fold(symbol, df, params, train_end, test_end)
        if m:
            fold_results.append({"fold": fold_num, **m})
            ok = "+" if m["total_pnl_pct"] > 0 else "-"
            print(f"\r      fold {fold_num}: {m['total_pnl_pct']:+7.2f}% ({m['round_trips']} trades) {ok}", flush=True)
        else:
            print(f"\r      fold {fold_num}: skipped (insufficient data)", flush=True)

    if not fold_results:
        return {"symbol": symbol, "label": label, "error": "no_valid_folds"}

    oos_pnls = [f["total_pnl_pct"] for f in fold_results]
    oos_sharpes = [f["sharpe"] for f in fold_results]
    total_trades = sum(f["round_trips"] for f in fold_results)

    # Cap Sharpe for aggregation (same as night_shift.py)
    SHARPE_CAP = 100.0
    capped = [max(-SHARPE_CAP, min(SHARPE_CAP, s)) for s in oos_sharpes]
    med_sharpe = float(np.median(capped)) if capped else 0
    consistency = sum(1 for p in oos_pnls if p > 0) / len(oos_pnls) if oos_pnls else 0

    # Aggregate
    total_pnl = sum(oos_pnls)
    avg_wr = np.mean([f["win_rate"] for f in fold_results])
    avg_pf = np.mean([f["profit_factor"] for f in fold_results
                      if f["profit_factor"] != float("inf")])
    avg_dd = np.mean([f["max_drawdown_pct"] for f in fold_results])

    # Compare to fast simulator estimate
    return {
        "symbol": symbol, "label": label, "params": {k: v for k, v in params.items() if k != "label"},
        "total_pnl_pct": round(total_pnl, 2),
        "median_sharpe": round(med_sharpe, 2),
        "consistency": round(consistency, 3),
        "avg_win_rate": round(avg_wr, 3),
        "avg_pf": round(avg_pf, 2),
        "avg_max_dd": round(avg_dd, 2),
        "total_trades": total_trades,
        "avg_trades_per_fold": round(total_trades / len(fold_results), 1),
        "folds": fold_results,
    }


async def main():
    parser = argparse.ArgumentParser(description="Validate night shift candidates with full simulator (fees + slippage)")
    parser.add_argument("--symbol", default=None, help="Validate single symbol only")
    parser.add_argument("--top", type=int, default=3, help="Number of top candidates to test per symbol")
    parser.add_argument("--production", action="store_true", help="Also test production baseline")
    args = parser.parse_args()

    print(f"\n{'='*90}")
    print(f"VALIDATION — Night Shift Candidates via FutureBlindSimulator")
    print(f"  Includes: fees, slippage, position sizing")
    print(f"{'='*90}\n")

    # Determine which symbols to test
    if args.symbol:
        symbols = [args.symbol]
    else:
        symbols = [s for s in NIGHT_SHIFT_CANDIDATES if NIGHT_SHIFT_CANDIDATES[s]]

    all_results = []

    for symbol in symbols:
        safe = symbol.replace("/", "_")
        path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
        if not os.path.exists(path):
            print(f"  {symbol}: no data, skipping")
            continue
        df = pd.read_parquet(path)
        folds = make_folds(len(df))

        print(f"\n{'─'*70}")
        print(f"  {symbol}: {len(df)} candles, {len(folds)} WFA folds")
        print(f"{'─'*70}")

        # Production baseline
        if args.production:
            print(f"\n  [PRODUCTION]")
            r = await validate_candidate(symbol, PRODUCTION_CONFIG, df, folds, "production")
            if "error" not in r:
                all_results.append(r)
                print(f"  → PnL: {r['total_pnl_pct']:+.2f}%  Sharpe: {r['median_sharpe']:+.2f}  "
                      f"Consistency: {r['consistency']:.0%}  Trades: {r['total_trades']}")
            else:
                print(f"  → {r['error']}")

        # Night shift candidates
        candidates = NIGHT_SHIFT_CANDIDATES.get(symbol, [])[:args.top]
        for i, cand in enumerate(candidates):
            label = cand.get("label", f"candidate_{i+1}")
            print(f"\n  [{label.upper()}]")
            r = await validate_candidate(symbol, cand, df, folds, label)
            if "error" not in r:
                all_results.append(r)
                print(f"\n  → PnL: {r['total_pnl_pct']:+.2f}%  Sharpe: {r['median_sharpe']:+.2f}  "
                      f"Consistency: {r['consistency']:.0%}  Trades: {r['total_trades']}  "
                      f"DD: {r['avg_max_dd']:.2f}%  WR: {r['avg_win_rate']:.0%}  PF: {r['avg_pf']:.2f}")

    # Summary
    print(f"\n\n{'='*90}")
    print(f"VALIDATION SUMMARY — Full Simulator (fees + slippage)")
    print(f"{'='*90}\n")

    print(f"  {'Symbol':12s} {'Config':20s} {'PnL':>8s} {'Sharpe':>8s} {'Cons':>5s} "
          f"{'DD':>6s} {'WR':>5s} {'PF':>5s} {'Trades':>7s} {'Avg/Fold':>8s}")
    print(f"  {'─'*12} {'─'*20} {'─'*8} {'─'*8} {'─'*5} {'─'*6} {'─'*5} {'─'*5} {'─'*7} {'─'*8}")

    for r in all_results:
        pf_s = f"{r['avg_pf']:.1f}" if r['avg_pf'] < 999 else "INF"
        print(f"  {r['symbol']:12s} {r['label']:20s} {r['total_pnl_pct']:+7.2f}% "
              f"{r['median_sharpe']:+7.2f} {r['consistency']:4.0%} "
              f"{r['avg_max_dd']:5.1f}% {r['avg_win_rate']:4.0%} {pf_s:>5s} "
              f"{r['total_trades']:6d}  {r['avg_trades_per_fold']:7.1f}")

    # Verdict
    print(f"\n  Verdicts:")
    for r in all_results:
        if r["consistency"] >= 0.7 and r["total_pnl_pct"] > 0:
            v = "STRONG"
        elif r["consistency"] >= 0.5 and r["total_pnl_pct"] > 0:
            v = "MODERATE"
        elif r["consistency"] >= 0.4:
            v = "MARGINAL"
        else:
            v = "FAILED"
        print(f"    [{v:8s}] {r['symbol']} {r['label']} — "
              f"PnL={r['total_pnl_pct']:+.2f}% cons={r['consistency']:.0%}")

    # Save
    out_dir = os.path.join(RESULTS_DIR, datetime.now(timezone.utc).strftime("%Y-%m-%d"))
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "full_sim_validation.json")
    with open(out_path, "w") as f:
        json.dump({
            "run_at": datetime.now(timezone.utc).isoformat(),
            "simulator": "FutureBlindSimulator (fees + slippage)",
            "results": all_results,
        }, f, indent=2, default=str)
    print(f"\n  Saved: {out_path}")


if __name__ == "__main__":
    asyncio.run(main())
