"""
Validate per-symbol optimized configs through the actual FutureBlindSimulator.
This gives accurate numbers including fees, slippage, and position sizing.
"""
import asyncio
import os
import sys
import json
import numpy as np

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from scripts.run_backtest_r2 import MultiTFStrategy, compute_round_trip_metrics_list
from backtesting.future_blind_simulator import FutureBlindSimulator
from agents.historical_data_collector import DataWindow

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")
SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT"]

GLOBAL_CONFIG = {
    "signal_threshold": 0.40,
    "min_alignment": 3,
    "take_profit_atr": 6.0,
    "stop_loss_atr": 2.5,
    "max_hold_hours": 96,
    "time_decay_hours": 48,
}

OPTIMIZED_CONFIGS = {
    "BTC/USDT": {
        "signal_threshold": 0.35,
        "min_alignment": 3,
        "take_profit_atr": 5.0,
        "stop_loss_atr": 1.5,
        "max_hold_hours": 72,
        "time_decay_hours": 24,
    },
    "ETH/USDT": GLOBAL_CONFIG,  # keep global for ETH
    "SOL/USDT": {
        "signal_threshold": 0.35,
        "min_alignment": 3,
        "take_profit_atr": 4.0,
        "stop_loss_atr": 1.5,
        "max_hold_hours": 48,
        "time_decay_hours": 24,
    },
    "BNB/USDT": {
        "signal_threshold": 0.45,
        "min_alignment": 3,
        "take_profit_atr": 5.0,
        "stop_loss_atr": 1.5,
        "max_hold_hours": 96,
        "time_decay_hours": 48,
    },
}


async def backtest(symbol, params, label):
    """Full simulator backtest."""
    safe = symbol.replace("/", "_")
    path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
    df = pd.read_parquet(path)

    strategy = MultiTFStrategy(f"val_{label}_{symbol}", {**{"symbol": symbol}, **params})
    sim = FutureBlindSimulator(initial_capital=10000)
    sim.add_strategy(strategy)

    window = DataWindow(
        symbol=symbol, exchange="binance",
        start_time=df.index[0].to_pydatetime(),
        end_time=df.index[-1].to_pydatetime(),
        current_time=df.index[0].to_pydatetime(),
        data=df,
    )

    result = await sim.run_simulation(window, time_step_minutes=60)
    trips = strategy.completed_round_trips
    m = compute_round_trip_metrics_list(trips)
    return {**m, "final_balance": result.final_balance}


import pandas as pd


async def main():
    print(f"\n{'='*90}")
    print(f"VALIDATION — Per-Symbol Optimized vs Global (via FutureBlindSimulator)")
    print(f"{'='*90}\n")

    print(f"  {'Symbol':12s} {'Config':12s} {'Rounds':>7s} {'WR':>6s} {'AvgPnL':>8s} {'Total':>8s} {'MaxDD':>7s} {'Sharpe':>7s} {'PF':>5s} {'Bal':>9s}")
    print(f"  {'─'*12} {'─'*12} {'─'*7} {'─'*6} {'─'*8} {'─'*8} {'─'*7} {'─'*7} {'─'*5} {'─'*9}")

    for symbol in SYMBOLS:
        # Global
        print(f"  [{symbol}] Running global config...", flush=True)
        g = await backtest(symbol, GLOBAL_CONFIG, "global")
        pf_s = f"{g['profit_factor']:.1f}" if g['profit_factor'] != float('inf') else "INF"
        print(f"  {symbol:12s} {'GLOBAL':12s} {g['round_trips']:7d} {g['win_rate']*100:5.1f}% {g['avg_pnl_pct']:7.3f}% {g['total_pnl_pct']:7.3f}% {g['max_drawdown_pct']:6.2f}% {g['sharpe']:7.2f} {pf_s:>5s} ${g['final_balance']:>8,.2f}")

        # Optimized
        opt = OPTIMIZED_CONFIGS[symbol]
        label = "OPTIMIZED" if opt != GLOBAL_CONFIG else "SAME"
        print(f"  [{symbol}] Running optimized config...", flush=True)
        o = await backtest(symbol, opt, "optimized")
        pf_s = f"{o['profit_factor']:.1f}" if o['profit_factor'] != float('inf') else "INF"
        print(f"  {symbol:12s} {label:12s} {o['round_trips']:7d} {o['win_rate']*100:5.1f}% {o['avg_pnl_pct']:7.3f}% {o['total_pnl_pct']:7.3f}% {o['max_drawdown_pct']:6.2f}% {o['sharpe']:7.2f} {pf_s:>5s} ${o['final_balance']:>8,.2f}")

        # Improvement
        diff = o['total_pnl_pct'] - g['total_pnl_pct']
        print(f"  {'':12s} {'Δ IMPROV':12s} {'':>7s} {'':>6s} {'':>8s} {diff:+7.3f}%")
        print()


if __name__ == "__main__":
    asyncio.run(main())
