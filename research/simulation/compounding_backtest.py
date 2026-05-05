"""
Compounding backtest — walk through trades sequentially, sizing each position
as a percentage of current capital (not fixed).

Usage:
    python -m research.simulation.compounding_backtest
    python -m research.simulation.compounding_backtest --leverage 3 --position-pct 0.2
"""
import argparse
import os
import sys

import numpy as np
import pandas as pd

sys.stdout.reconfigure(line_buffering=True)
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))

from research.optimization.per_symbol_optimizer import compute_indicators, simulate_trades

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "data", "ohlcv")

BASE_PARAMS = {
    "signal_threshold": 0.3,
    "min_alignment": 3,
    "take_profit_atr": 3.0,
    "stop_loss_atr": 2.5,
    "max_hold_hours": 36,
    "time_decay_hours": 12,
    "trailing_stop_atr": 0.3,
    "score_flip_delay_hrs": 0,
}


def run_compounding(symbol, leverage, position_pct, initial_capital):
    safe = symbol.replace("/", "_")
    path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
    df = pd.read_parquet(path)
    df = compute_indicators(df)
    total_days = len(df) / 24

    params = {**BASE_PARAMS, "leverage": leverage}
    trips = simulate_trades(df, params)

    capital = initial_capital
    peak = capital
    max_dd = 0.0
    equity_high_water = capital
    trade_log = []
    monthly_pnl = {}

    for t in trips:
        position_size = capital * position_pct
        pnl = position_size * (t["pnl_pct"] / 100.0)
        capital += pnl

        if capital > peak:
            peak = capital
        dd = (peak - capital) / peak * 100
        max_dd = max(max_dd, dd)

        trade_log.append({
            "pnl_pct": t["pnl_pct"],
            "capital": round(capital, 4),
            "pnl_sol": round(pnl, 4),
            "exit": t["exit"],
            "hold_hrs": t["hold_hrs"],
            "liquidated": t.get("liquidated", False),
        })

    pnls = [t["pnl_pct"] for t in trade_log]
    wins = [p for p in pnls if p > 0]
    losses = [p for p in pnls if p <= 0]
    liqs = [t for t in trade_log if t["liquidated"]]
    total_return = (capital - initial_capital) / initial_capital * 100
    annual_return = ((capital / initial_capital) ** (365 / total_days) - 1) * 100
    sharpe = np.mean(pnls) / np.std(pnls) * np.sqrt(len(pnls) / total_days * 365) if np.std(pnls) > 0 else 0

    print(f"")
    print(f"{'='*62}")
    print(f"COMPOUNDING BACKTEST — {leverage}x Leverage, {position_pct:.0%} position sizing")
    print(f"{'='*62}")
    print(f"Symbol:             {symbol}")
    print(f"Period:             {df.index[250].date()} -> {df.index[-1].date()} ({total_days:.0f} days)")
    print(f"Initial capital:    {initial_capital:.1f} SOL")
    print(f"Final capital:      {capital:.2f} SOL")
    print(f"Total return:       {total_return:+.1f}%")
    print(f"Annualized return:  {annual_return:+.1f}%")
    print(f"Max drawdown:       {max_dd:.1f}%")
    print(f"Sharpe (annual):    {sharpe:.2f}")
    print(f"Total trades:       {len(trade_log)}")
    print(f"Win rate:           {len(wins)/len(trade_log):.0%}")
    print(f"Avg win:            {np.mean(wins):+.1f}%" if wins else "")
    print(f"Avg loss:           {np.mean(losses):+.1f}%" if losses else "")
    print(f"Best trade:         {max(pnls):+.1f}%")
    print(f"Worst trade:        {min(pnls):+.1f}%")
    print(f"Liquidations:       {len(liqs)}")
    print(f"Avg hold:           {np.mean([t['hold_hrs'] for t in trade_log]):.1f}h")

    return capital, total_return, annual_return, max_dd


def main():
    parser = argparse.ArgumentParser(description="Compounding backtest")
    parser.add_argument("--symbol", default="SOL/USDT")
    parser.add_argument("--leverage", type=float, nargs="+", default=[1.0, 2.0, 3.0, 5.0])
    parser.add_argument("--position-pct", type=float, default=0.20)
    parser.add_argument("--initial", type=float, default=100.0)
    args = parser.parse_args()

    results = []
    for lev in args.leverage:
        cap, ret, ann, dd = run_compounding(args.symbol, lev, args.position_pct, args.initial)
        results.append((lev, cap, ret, ann, dd))

    if len(results) > 1:
        print(f"\n{'='*62}")
        print(f"COMPARISON — {args.symbol}, {args.position_pct:.0%} position, {args.initial:.0f} SOL start")
        print(f"{'='*62}")
        print(f"{'Lev':<6} {'Final SOL':<12} {'Total Ret':<12} {'Annual Ret':<12} {'Max DD':<10}")
        print(f"{'─'*52}")
        for lev, cap, ret, ann, dd in results:
            print(f"{lev:<6.0f}x {cap:<12.2f} {ret:<+12.1f}% {ann:<+12.1f}% {dd:<10.1f}%")


if __name__ == "__main__":
    main()
