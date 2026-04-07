"""
Walk-Forward Analysis with FIXED parameters (no per-fold optimization).

Tests whether the wide_tp config found on full 365d data generalizes
to individual 30-day windows without re-optimization.

This is the more realistic test — in production you'd deploy fixed params,
not re-optimize every month.
"""
import asyncio
import os
import sys
import json
import numpy as np
import pandas as pd
from datetime import datetime, timedelta
from typing import Dict, List

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from backtesting.future_blind_simulator import FutureBlindSimulator
from agents.historical_data_collector import DataWindow
from scripts.run_backtest_r2 import MultiTFStrategy

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")
SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT", "XRP/USDT"]

# Fixed params from the winning R6 wide_tp config
FIXED_PARAMS = {
    "signal_threshold": 0.40,
    "min_alignment": 3,
    "take_profit_atr": 6.0,
    "stop_loss_atr": 2.5,
    "max_hold_hours": 96,
    "time_decay_hours": 48,
}


def compute_round_trip_metrics_list(trips: list) -> Dict:
    if not trips:
        return {"round_trips": 0, "win_rate": 0, "total_pnl_pct": 0, "avg_pnl_pct": 0,
                "sharpe": 0, "max_drawdown_pct": 0, "best_trade": 0, "worst_trade": 0,
                "avg_win": 0, "avg_loss": 0, "profit_factor": 0, "avg_hold_hrs": 0,
                "exit_reasons": {}}
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
        "best_trade": max(pnls), "worst_trade": min(pnls),
        "avg_win": avg_win, "avg_loss": avg_loss,
        "profit_factor": pf, "exit_reasons": exit_reasons,
    }


async def backtest_window(symbol: str, df: pd.DataFrame, params: Dict) -> Dict:
    if len(df) < 250:
        return {"symbol": symbol, "error": "insufficient_data"}
    strategy = MultiTFStrategy(f"fw_{symbol}", {**{"symbol": symbol}, **params})
    sim = FutureBlindSimulator(initial_capital=10000)
    sim.add_strategy(strategy)
    window = DataWindow(
        symbol=symbol, exchange="binance",
        start_time=df.index[0].to_pydatetime(), end_time=df.index[-1].to_pydatetime(),
        current_time=df.index[0].to_pydatetime(), data=df,
    )
    result = await sim.run_simulation(window, time_step_minutes=60)
    trips = strategy.completed_round_trips
    metrics = compute_round_trip_metrics_list(trips)
    return {"symbol": symbol, "candles": len(df),
            "start": str(df.index[0].date()), "end": str(df.index[-1].date()), **metrics}


async def run_fixed_wfa(test_days: int = 30, step_days: int = 7):
    """Run WFA with fixed params — no optimization, just validation."""
    sample = pd.read_parquet(os.path.join(DATA_DIR, "BTC_USDT_1h.parquet"))
    data_start = sample.index[0].to_pydatetime()
    data_end = sample.index[-1].to_pydatetime()

    # Need 250h (~10d) warmup before first test window
    warmup = timedelta(hours=250)
    first_test = data_start + warmup

    print(f"\n{'='*90}")
    print(f"FIXED-PARAM WALK-FORWARD (no optimization)")
    print(f"Data: {data_start.date()} to {data_end.date()}")
    print(f"Test windows: {test_days}d, step: {step_days}d")
    print(f"Params: wide_tp (threshold=0.40, alignment=3, TP=6xATR, SL=2.5xATR)")
    print(f"{'='*90}\n")

    # Generate non-overlapping test windows
    folds = []
    current = first_test
    fold_num = 0
    while True:
        test_start = current
        test_end = current + timedelta(days=test_days)
        if test_end > data_end:
            break
        folds.append((fold_num, test_start, test_end))
        current += timedelta(days=step_days)
        fold_num += 1

    print(f"{'Fold':>4s} {'Test Period':>24s} {'PnL':>8s} {'WR':>6s} {'Sharpe':>8s} {'PF':>6s} {'DD':>7s} {'Trades':>6s} "
          f"{'BTC':>7s} {'ETH':>7s} {'SOL':>7s} {'BNB':>7s} {'XRP':>7s}")
    print(f"{'─'*4} {'─'*24} {'─'*8} {'─'*6} {'─'*8} {'─'*6} {'─'*7} {'─'*6} "
          f"{'─'*7} {'─'*7} {'─'*7} {'─'*7} {'─'*7}")

    all_folds = []
    for fold_num, test_start, test_end in folds:
        print(f"  Fold {fold_num}...", end="", flush=True)
        results = []
        for symbol in SYMBOLS:
            df = pd.read_parquet(os.path.join(DATA_DIR, f"{symbol.replace('/', '_')}_1h.parquet"))
            warmup_start = test_start - timedelta(hours=250)
            window_df = df[warmup_start:test_end]
            r = await backtest_window(symbol, window_df, FIXED_PARAMS)
            if "error" not in r:
                results.append(r)

        if results:
            total_pnl = sum(r.get("total_pnl_pct", 0) for r in results)
            avg_wr = np.mean([r.get("win_rate", 0) for r in results]) * 100
            avg_sharpe = np.mean([r.get("sharpe", 0) for r in results])
            avg_pf = np.mean([r.get("profit_factor", 0) for r in results
                              if r.get("profit_factor", 0) != float("inf")])
            avg_dd = np.mean([r.get("max_drawdown_pct", 0) for r in results])
            total_trades = sum(r.get("round_trips", 0) for r in results)

            per_sym = {}
            for r in results:
                per_sym[r["symbol"]] = r.get("total_pnl_pct", 0)

            fold_data = {
                "fold": fold_num,
                "test_period": f"{test_start.date()} to {test_end.date()}",
                "total_pnl": round(total_pnl, 2),
                "avg_wr": round(avg_wr, 1),
                "avg_sharpe": round(avg_sharpe, 2),
                "avg_pf": round(avg_pf, 2),
                "avg_dd": round(avg_dd, 2),
                "trades": total_trades,
                "per_symbol": {k: round(v, 2) for k, v in per_sym.items()},
            }
            all_folds.append(fold_data)
            syms = per_sym
            print(f"\r  {fold_num:4d} {fold_data['test_period']:>24s} {total_pnl:7.1f}% {avg_wr:5.1f}% "
                  f"{avg_sharpe:7.2f} {avg_pf:5.2f} {avg_dd:6.1f}% {total_trades:6d} "
                  f"{syms.get('BTC/USDT',0):6.1f}% {syms.get('ETH/USDT',0):6.1f}% "
                  f"{syms.get('SOL/USDT',0):6.1f}% {syms.get('BNB/USDT',0):6.1f}% "
                  f"{syms.get('XRP/USDT',0):6.1f}%", flush=True)

    # Summary
    if not all_folds:
        print("\nNo valid folds!")
        return

    pnls = [f["total_pnl"] for f in all_folds]
    profitable = sum(1 for p in pnls if p > 0)

    print(f"\n{'='*90}")
    print(f"FIXED-PARAM WFA SUMMARY ({len(all_folds)} folds)")
    print(f"{'='*90}")
    print(f"  Profitable windows: {profitable}/{len(all_folds)} ({profitable/len(all_folds)*100:.0f}%)")
    print(f"  Mean PnL per window: {np.mean(pnls):.2f}% (±{np.std(pnls):.2f})")
    print(f"  Median PnL:          {np.median(pnls):.2f}%")
    print(f"  Total PnL:           {sum(pnls):.2f}%")
    print(f"  Best window:         {max(pnls):.2f}%")
    print(f"  Worst window:        {min(pnls):.2f}%")

    # Per-symbol consistency
    sym_data = {s: [] for s in SYMBOLS}
    for f in all_folds:
        for sym, pnl in f["per_symbol"].items():
            if sym in sym_data:
                sym_data[sym].append(pnl)

    print(f"\n  Per-symbol consistency:")
    for sym in SYMBOLS:
        vals = sym_data[sym]
        pos = sum(1 for v in vals if v > 0)
        print(f"    {sym:12s}  {pos}/{len(vals)} windows positive  "
              f"avg: {np.mean(vals):+.2f}%  total: {sum(vals):+.1f}%")

    # Verdict
    consistency = profitable / len(all_folds)
    if consistency >= 0.6 and np.mean(pnls) > 0:
        verdict = "✅ ROBUST — Fixed params generalize across most market conditions"
    elif consistency >= 0.5 and np.median(pnls) > 0:
        verdict = "⚠️ MODERATE — Edge exists but regime-dependent"
    elif consistency >= 0.4:
        verdict = "⚠️ MARGINAL — Slight edge, needs regime filter or symbol selection"
    else:
        verdict = "❌ FAILED — Fixed params do not generalize"

    print(f"\n  VERDICT: {verdict}")

    # Save
    out_path = os.path.join(DATA_DIR, "..", "wfa_fixed_params_results.json")
    with open(out_path, "w") as f:
        json.dump({
            "run_at": datetime.now().isoformat(),
            "params": FIXED_PARAMS,
            "test_days": test_days,
            "step_days": step_days,
            "folds": all_folds,
            "summary": {
                "total_folds": len(all_folds),
                "profitable_folds": profitable,
                "consistency_pct": round(consistency * 100, 1),
                "mean_pnl": round(np.mean(pnls), 2),
                "median_pnl": round(np.median(pnls), 2),
                "total_pnl": round(sum(pnls), 2),
                "verdict": verdict,
            },
            "per_symbol": {
                sym: {"positive": sum(1 for v in vals if v > 0),
                       "total": len(vals),
                       "avg_pnl": round(np.mean(vals), 2),
                       "total_pnl": round(sum(vals), 2)}
                for sym, vals in sym_data.items()
            },
        }, f, indent=2, default=str)
    print(f"\n  Results saved to {out_path}")


if __name__ == "__main__":
    asyncio.run(run_fixed_wfa())
