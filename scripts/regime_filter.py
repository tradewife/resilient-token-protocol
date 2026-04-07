"""
Regime Filter: ADX-based trend detection to skip ranging markets.

The WFA showed the strategy makes +30-59% in trending periods and -10-33%
in ranging periods. ADX measures trend strength regardless of direction:
- ADX > 25 = trending (trade)
- ADX < 20 = ranging (skip)
- 20-25 = transition zone (use score threshold)

This script:
1. Adds ADX computation to the strategy
2. Runs a before/after comparison on all 47 WFA windows
3. Reports which losing periods get filtered out
"""
import asyncio
import os
import sys
import json
import numpy as np
import pandas as pd
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from backtesting.future_blind_simulator import FutureBlindSimulator
from agents.historical_data_collector import DataWindow

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")
SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT", "XRP/USDT"]
WIDE_TP_PARAMS = {
    "signal_threshold": 0.40, "min_alignment": 3,
    "take_profit_atr": 6.0, "stop_loss_atr": 2.5,
    "max_hold_hours": 96, "time_decay_hours": 48,
}


def compute_adx(high: pd.Series, low: pd.Series, close: pd.Series, period: int = 14) -> pd.Series:
    """Compute ADX using Wilder's smoothing method."""
    # Directional movement
    up_move = high.diff()
    down_move = -low.diff()
    plus_dm = up_move.where((up_move > down_move) & (up_move > 0), 0.0)
    minus_dm = down_move.where((down_move > up_move) & (down_move > 0), 0.0)

    # True Range
    tr1 = high - low
    tr2 = (high - close.shift(1)).abs()
    tr3 = (low - close.shift(1)).abs()
    tr = pd.concat([tr1, tr2, tr3], axis=1).max(axis=1)

    # Wilder's smoothing (equivalent to EMA with alpha=1/period)
    alpha = 1.0 / period
    atr = tr.ewm(alpha=alpha, min_periods=period).mean()
    plus_di = 100 * plus_dm.ewm(alpha=alpha, min_periods=period).mean() / atr
    minus_di = 100 * minus_dm.ewm(alpha=alpha, min_periods=period).mean() / atr

    dx = 100 * (plus_di - minus_di).abs() / (plus_di + minus_di)
    adx = dx.ewm(alpha=alpha, min_periods=period).mean()

    return adx


def compute_round_trip_metrics_list(trips: list) -> Dict:
    if not trips:
        return {"round_trips": 0, "win_rate": 0, "total_pnl_pct": 0, "avg_pnl_pct": 0,
                "sharpe": 0, "max_drawdown_pct": 0, "profit_factor": 0, "avg_hold_hrs": 0, "exit_reasons": {}}
    pnls = [r["pnl_pct"] for r in trips]
    wins = [p for p in pnls if p > 0]
    losses = [p for p in pnls if p <= 0]
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
        "max_drawdown_pct": 0, "sharpe": 0, "best_trade": max(pnls), "worst_trade": min(pnls),
        "avg_win": avg_win, "avg_loss": avg_loss, "profit_factor": pf, "exit_reasons": exit_reasons,
    }


async def backtest_window_with_adx(symbol: str, df: pd.DataFrame, params: Dict,
                                    adx_threshold: float = 25.0, adx_period: int = 14) -> Dict:
    """Backtest with ADX regime filter integrated."""
    from scripts.run_backtest_r2 import MultiTFStrategy

    # Compute ADX on the full dataframe
    adx = compute_adx(df["high"], df["low"], df["close"], adx_period)

    strategy = MultiTFStrategy(f"adx_{symbol}", {**{"symbol": symbol}, **params})

    sim = FutureBlindSimulator(initial_capital=10000)
    sim.add_strategy(strategy)

    window = DataWindow(
        symbol=symbol, exchange="binance",
        start_time=df.index[0].to_pydatetime(), end_time=df.index[-1].to_pydatetime(),
        current_time=df.index[0].to_pydatetime(), data=df,
    )

    # Wrap the strategy's analyze to include ADX filter
    original_analyze = strategy.analyze
    regime_filtered = 0
    total_checks = 0

    async def analyze_with_adx(data: pd.DataFrame, current_time: datetime):
        nonlocal regime_filtered, total_checks
        total_checks += 1

        # Get current ADX value
        if current_time in adx.index:
            current_adx = adx.loc[current_time]
            if pd.notna(current_adx) and current_adx < adx_threshold:
                # Still check exits for open positions even in ranging market
                if symbol in strategy.open_positions:
                    return await original_analyze(data, current_time)
                regime_filtered += 1
                return None

        return await original_analyze(data, current_time)

    strategy.analyze = analyze_with_adx

    result = await sim.run_simulation(window, time_step_minutes=60)
    trips = strategy.completed_round_trips
    metrics = compute_round_trip_metrics_list(trips)

    return {
        "symbol": symbol,
        "candles": len(df),
        "regime_filtered": regime_filtered,
        "total_checks": total_checks,
        "filter_rate": regime_filtered / max(total_checks, 1),
        **metrics,
    }


async def run_regime_validation(test_days: int = 30, step_days: int = 7,
                                 adx_threshold: float = 25.0):
    """Compare WFA with and without regime filter."""
    sample = pd.read_parquet(os.path.join(DATA_DIR, "BTC_USDT_1h.parquet"))
    data_start = sample.index[0].to_pydatetime()
    data_end = sample.index[-1].to_pydatetime()
    warmup = timedelta(hours=300)  # extra warmup for ADX (needs 14 periods + buffer)
    first_test = data_start + warmup

    # Generate windows
    folds = []
    current = first_test
    while True:
        test_end = current + timedelta(days=test_days)
        if test_end > data_end:
            break
        folds.append((current, test_end))
        current += timedelta(days=step_days)

    print(f"\n{'='*90}")
    print(f"REGIME FILTER VALIDATION — ADX threshold={adx_threshold}")
    print(f"Windows: {len(folds)} × {test_days}d, step={step_days}d")
    print(f"{'='*90}\n")

    print(f"{'Window':>24s} {'NoFilter':>9s} {'ADX_Filt':>9s} {'Delta':>8s} "
          f"{'NF_WR':>6s} {'ADX_WR':>7s} {'NF_Tr':>5s} {'ADX_Tr':>6s} {'Filt%':>6s}")
    print(f"{'─'*24} {'─'*9} {'─'*9} {'─'*8} {'─'*6} {'─'*7} {'─'*5} {'─'*6} {'─'*6}")

    # Run both versions on each window
    results = []
    for test_start, test_end in folds:
        print(f"  {test_start.date()}...", end="", flush=True)

        nf_results = []
        adx_results = []

        for symbol in SYMBOLS:
            safe = symbol.replace("/", "_")
            df = pd.read_parquet(os.path.join(DATA_DIR, f"{safe}_1h.parquet"))
            warmup_start = test_start - timedelta(hours=300)
            window_df = df[warmup_start:test_end]

            if len(window_df) < 300:
                continue

            # No-filter version
            from scripts.run_backtest_r2 import MultiTFStrategy
            strat_nf = MultiTFStrategy(f"nf_{symbol}", {**{"symbol": symbol}, **WIDE_TP_PARAMS})
            sim_nf = FutureBlindSimulator(initial_capital=10000)
            sim_nf.add_strategy(strat_nf)
            win_nf = DataWindow(symbol=symbol, exchange="binance",
                                start_time=window_df.index[0].to_pydatetime(),
                                end_time=window_df.index[-1].to_pydatetime(),
                                current_time=window_df.index[0].to_pydatetime(),
                                data=window_df)
            res_nf = await sim_nf.run_simulation(win_nf, time_step_minutes=60)
            trips_nf = strat_nf.completed_round_trips
            m_nf = compute_round_trip_metrics_list(trips_nf)
            nf_results.append({"symbol": symbol, **m_nf})

            # ADX-filtered version
            m_adx = await backtest_window_with_adx(symbol, window_df, WIDE_TP_PARAMS,
                                                    adx_threshold=adx_threshold)
            adx_results.append(m_adx)

        if nf_results and adx_results:
            nf_pnl = sum(r.get("total_pnl_pct", 0) for r in nf_results)
            adx_pnl = sum(r.get("total_pnl_pct", 0) for r in adx_results)
            nf_wr = np.mean([r.get("win_rate", 0) for r in nf_results]) * 100
            adx_wr = np.mean([r.get("win_rate", 0) for r in adx_results]) * 100
            nf_trades = sum(r.get("round_trips", 0) for r in nf_results)
            adx_trades = sum(r.get("round_trips", 0) for r in adx_results)
            avg_filter = np.mean([r.get("filter_rate", 0) for r in adx_results]) * 100
            delta = adx_pnl - nf_pnl

            row = {
                "period": f"{test_start.date()} to {test_end.date()}",
                "nf_pnl": round(nf_pnl, 1),
                "adx_pnl": round(adx_pnl, 1),
                "delta": round(delta, 1),
                "nf_wr": round(nf_wr, 1),
                "adx_wr": round(adx_wr, 1),
                "nf_trades": nf_trades,
                "adx_trades": adx_trades,
                "filter_pct": round(avg_filter, 1),
            }
            results.append(row)
            print(f"\r  {row['period']:>24s} {row['nf_pnl']:+8.1f}% {row['adx_pnl']:+8.1f}% "
                  f"{row['delta']:+7.1f}% {row['nf_wr']:5.1f}% {row['adx_wr']:6.1f}% "
                  f"{row['nf_trades']:5d} {row['adx_trades']:6d} {row['filter_pct']:5.1f}%", flush=True)

    # Summary
    nf_pnls = [r["nf_pnl"] for r in results]
    adx_pnls = [r["adx_pnl"] for r in results]

    nf_profitable = sum(1 for p in nf_pnls if p > 0)
    adx_profitable = sum(1 for p in adx_pnls if p > 0)
    nf_worst = min(nf_pnls)
    adx_worst = min(adx_pnls)

    print(f"\n{'='*90}")
    print(f"REGIME FILTER SUMMARY (ADX > {adx_threshold})")
    print(f"{'='*90}")
    print(f"  {'':20s} {'No Filter':>12s} {'ADX Filter':>12s} {'Improvement':>12s}")
    print(f"  {'─'*20} {'─'*12} {'─'*12} {'─'*12}")
    print(f"  {'Profitable windows':20s} {nf_profitable:>4d}/{len(results):>5d}   "
          f"{adx_profitable:>4d}/{len(results):>5d}   "
          f"{adx_profitable - nf_profitable:>+4d} windows")
    print(f"  {'Mean PnL/window':20s} {np.mean(nf_pnls):>+11.1f}%  "
          f"{np.mean(adx_pnls):>+11.1f}%  {np.mean(adx_pnls) - np.mean(nf_pnls):>+11.1f}%")
    print(f"  {'Median PnL/window':20s} {np.median(nf_pnls):>+11.1f}%  "
          f"{np.median(adx_pnls):>+11.1f}%  {np.median(adx_pnls) - np.median(nf_pnls):>+11.1f}%")
    print(f"  {'Total PnL':20s} {sum(nf_pnls):>+11.1f}%  "
          f"{sum(adx_pnls):>+11.1f}%  {sum(adx_pnls) - sum(nf_pnls):>+11.1f}%")
    print(f"  {'Worst window':20s} {nf_worst:>+11.1f}%  "
          f"{adx_worst:>+11.1f}%  {adx_worst - nf_worst:>+11.1f}%")
    print(f"  {'Std Dev':20s} {np.std(nf_pnls):>11.1f}%  "
          f"{np.std(adx_pnls):>11.1f}%  {np.std(adx_pnls) - np.std(nf_pnls):>+11.1f}%")

    avg_trades_nf = np.mean([r["nf_trades"] for r in results])
    avg_trades_adx = np.mean([r["adx_trades"] for r in results])
    avg_filter = np.mean([r["filter_pct"] for r in results])
    print(f"\n  Avg trades/window:  {avg_trades_nf:.0f} → {avg_trades_adx:.0f} "
          f"({avg_filter:.0f}% filtered out)")

    # Save
    out_path = os.path.join(DATA_DIR, "..", "regime_filter_results.json")
    with open(out_path, "w") as f:
        json.dump({
            "run_at": datetime.now().isoformat(),
            "adx_threshold": adx_threshold,
            "folds": results,
            "summary": {
                "nf_profitable": nf_profitable, "adx_profitable": adx_profitable,
                "nf_mean": round(np.mean(nf_pnls), 2), "adx_mean": round(np.mean(adx_pnls), 2),
                "nf_total": round(sum(nf_pnls), 2), "adx_total": round(sum(adx_pnls), 2),
                "nf_worst": round(nf_worst, 2), "adx_worst": round(adx_worst, 2),
                "nf_std": round(np.std(nf_pnls), 2), "adx_std": round(np.std(adx_pnls), 2),
            },
        }, f, indent=2, default=str)
    print(f"\n  Results saved to {out_path}")


if __name__ == "__main__":
    asyncio.run(run_regime_validation(adx_threshold=25.0))
