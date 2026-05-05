"""
Leverage Night Shift — Full WFA + Compounding with Flash Trade Fee Model.

Runs the night shift pipeline (expanding-window WFA, 9 folds) for leverage
levels 3x through 10x, using REALISTIC Flash Trade fees:

  - Open fee:  0.06% of position notional
  - Close fee: 0.06% of position notional
  - Hourly borrow rate: 0.0042% per hour (accumulates over hold time)
  - No swap fee (SOL long with SOL collateral)

Each trade's PnL is adjusted for fees BEFORE compounding at 20% position sizing.
Results include compounded final capital, max drawdown, Sharpe, win rate, and
liquidation count across all WFA folds.

Usage:
    python -m research.simulation.leverage_night_shift
    python -m research.simulation.leverage_night_shift --leverage 3,4,5
    python -m research.simulation.leverage_night_shift --no-compound
"""
import argparse
import json
import os
import sys
from dataclasses import dataclass

import numpy as np
import pandas as pd

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))

from research.optimization.per_symbol_optimizer import (
    compute_indicators,
    simulate_trades,
)
from research.orchestration.night_shift import create_folds

ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
DATA_DIR = os.path.join(ROOT, "data", "ohlcv")
OUTPUT_DIR = os.path.join(ROOT, "data", "leverage_night_shift")

# Survivor 2.69 base config
BASE_CONFIG = {
    "signal_threshold": 0.3,
    "min_alignment": 3,
    "take_profit_atr": 3.0,
    "stop_loss_atr": 1.5,
    "max_hold_hours": 36,
    "time_decay_hours": 12,
    "trailing_stop_atr": 0.5,
    "score_flip_delay_hrs": 0,
}

# Flash Trade fee model (from SKILL.md / TransactionFlow.md)
# Open fee: 0.06% of position notional (openPositionFeePercent)
# Close fee: 0.06% of position notional (same structure for close)
# Hourly borrow: 0.0042% per hour (marginFeePercentage) — accumulates over hold time
# No swap fee: SOL collateral → SOL long position (same asset)
FLASH_OPEN_FEE_PCT = 0.06     # % of notional
FLASH_CLOSE_FEE_PCT = 0.06    # % of notional
FLASH_HOURLY_BORROW_PCT = 0.0042  # % of notional per hour

POSITION_PCT = 0.20   # 20% of capital per trade
INITIAL_CAPITAL = 100.0  # SOL
WFA_FOLDS = 10
TEST_FOLD_DAYS = 36


@dataclass
class FoldCompoundResult:
    fold_num: int
    final_capital: float
    total_return_pct: float
    max_drawdown_pct: float
    trades: int
    wins: int
    liquidations: int
    avg_hold_hrs: float
    total_fees_sol: float


def flash_trade_round_trip_cost(leverage: float, hold_hrs: float) -> float:
    """Calculate the total Flash Trade fee as a % of MARGIN for one round trip.

    Position notional = margin * leverage
    Open fee = notional * open_fee_pct = margin * leverage * 0.06%
    Close fee = notional * close_fee_pct = margin * leverage * 0.06%
    Borrow = notional * hourly_rate * hold_hrs = margin * leverage * 0.0042% * hold_hrs

    As a % of margin: leverage * (open_fee + close_fee + hourly_rate * hold_hrs)
    """
    notional_fee_pct = FLASH_OPEN_FEE_PCT + FLASH_CLOSE_FEE_PCT + FLASH_HOURLY_BORROW_PCT * hold_hrs
    return leverage * notional_fee_pct


def run_fold_compound(df_fold, params, leverage, position_pct, initial_capital):
    """Run the fast simulator on one fold's data, compound with Flash Trade fees."""
    trips = simulate_trades(df_fold, params)

    if not trips:
        return FoldCompoundResult(
            fold_num=0, final_capital=initial_capital, total_return_pct=0,
            max_drawdown_pct=0, trades=0, wins=0, liquidations=0,
            avg_hold_hrs=0, total_fees_sol=0,
        )

    capital = initial_capital
    peak = capital
    max_dd = 0.0
    wins = 0
    liqs = 0
    total_fees = 0.0
    hold_times = []

    for t in trips:
        hold_hrs = t["hold_hrs"]
        hold_times.append(hold_hrs)

        if t.get("liquidated", False):
            # Liquidation: lose entire margin on this trade
            position_size = capital * position_pct
            capital -= position_size
            liqs += 1
        else:
            # Raw price PnL as % of margin (fast sim already applied leverage)
            raw_pnl_pct = t["pnl_pct"]

            # Flash Trade fee as % of margin
            fee_pct = flash_trade_round_trip_cost(leverage, hold_hrs)

            # Net PnL after fees
            net_pnl_pct = raw_pnl_pct - fee_pct

            # Position sizing
            position_size = capital * position_pct
            pnl_sol = position_size * (net_pnl_pct / 100.0)
            fee_sol = position_size * (fee_pct / 100.0)

            capital += pnl_sol
            total_fees += fee_sol

            if net_pnl_pct > 0:
                wins += 1

        if capital > peak:
            peak = capital
        if peak > 0:
            dd = (peak - capital) / peak * 100
            max_dd = max(max_dd, dd)

    total_trades = len(trips)
    return FoldCompoundResult(
        fold_num=0,
        final_capital=round(capital, 4),
        total_return_pct=round((capital - initial_capital) / initial_capital * 100, 2),
        max_drawdown_pct=round(max_dd, 2),
        trades=total_trades,
        wins=wins,
        liquidations=liqs,
        avg_hold_hrs=round(np.mean(hold_times), 1) if hold_times else 0,
        total_fees_sol=round(total_fees, 4),
    )


def run_wfa_leverage(df, leverage, params, sl_atr=None, trail_atr=None):
    """Run full WFA with compounding + Flash Trade fees for one leverage level.

    Returns per-fold results plus aggregate metrics.
    """
    actual_params = dict(params)
    actual_params["leverage"] = leverage
    if sl_atr is not None:
        actual_params["stop_loss_atr"] = sl_atr
    if trail_atr is not None:
        actual_params["trailing_stop_atr"] = trail_atr

    folds = create_folds(len(df), WFA_FOLDS, TEST_FOLD_DAYS)
    if not folds:
        return None

    # We need to compound ACROSS folds (capital carries forward)
    capital = INITIAL_CAPITAL
    peak = capital
    max_dd = 0.0
    total_trades = 0
    total_wins = 0
    total_liqs = 0
    total_fees = 0.0
    fold_results = []
    all_hold_hrs = []

    for fold in folds:
        test_df = df.iloc[fold.test_start_idx:fold.test_end_idx]
        if len(test_df) < 10:
            continue

        # Run sim on this fold's test data
        fold_result = run_fold_compound(
            test_df, actual_params, leverage, POSITION_PCT, capital
        )
        fold_result.fold_num = fold.fold_num

        # Track from fold's starting capital
        fold_start_capital = capital
        capital = fold_result.final_capital
        total_trades += fold_result.trades
        total_wins += fold_result.wins
        total_liqs += fold_result.liquidations
        total_fees += fold_result.total_fees_sol
        if fold_result.avg_hold_hrs > 0:
            all_hold_hrs.append(fold_result.avg_hold_hrs)

        if capital > peak:
            peak = capital
        if peak > 0:
            dd = (peak - capital) / peak * 100
            max_dd = max(max_dd, dd)

        fold_results.append({
            "fold": fold.fold_num,
            "start_capital": round(fold_start_capital, 2),
            "end_capital": round(capital, 2),
            "return_pct": round((capital - fold_start_capital) / fold_start_capital * 100, 2),
            "trades": fold_result.trades,
            "wins": fold_result.wins,
            "liquidations": fold_result.liquidations,
            "max_dd_pct": fold_result.max_drawdown_pct,
            "fees_sol": fold_result.total_fees_sol,
        })

    if not fold_results:
        return None

    # Aggregate
    total_return = (capital - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100
    total_days = len(df) / 24
    annual_return = ((capital / INITIAL_CAPITAL) ** (365 / total_days) - 1) * 100 if capital > 0 else -100
    win_rate = total_wins / total_trades if total_trades > 0 else 0
    consistency = sum(1 for f in fold_results if f["return_pct"] > 0) / len(fold_results)
    avg_hold = np.mean(all_hold_hrs) if all_hold_hrs else 0

    # Per-trade PnLs for Sharpe (approximate from fold returns)
    fold_returns = [f["return_pct"] for f in fold_results]
    if len(fold_returns) > 1 and np.std(fold_returns) > 0:
        # Use fold-level returns as Sharpe input (conservative)
        sharpe = np.mean(fold_returns) / np.std(fold_returns) * np.sqrt(len(fold_returns))
    else:
        sharpe = 0.0

    return {
        "leverage": leverage,
        "sl_atr": actual_params["stop_loss_atr"],
        "trail_atr": actual_params["trailing_stop_atr"],
        "initial_capital": INITIAL_CAPITAL,
        "final_capital": round(capital, 2),
        "total_return_pct": round(total_return, 2),
        "annualized_return_pct": round(annual_return, 2),
        "max_drawdown_pct": round(max_dd, 2),
        "sharpe": round(sharpe, 2),
        "total_trades": total_trades,
        "win_rate": round(win_rate, 3),
        "consistency": round(consistency, 3),
        "liquidations": total_liqs,
        "total_fees_sol": round(total_fees, 2),
        "avg_hold_hrs": round(avg_hold, 1),
        "position_pct": POSITION_PCT,
        "flash_fees": {
            "open_pct": FLASH_OPEN_FEE_PCT,
            "close_pct": FLASH_CLOSE_FEE_PCT,
            "hourly_borrow_pct": FLASH_HOURLY_BORROW_PCT,
        },
        "folds": fold_results,
        "params": {k: v for k, v in actual_params.items()},
    }


def main():
    parser = argparse.ArgumentParser(description="Leverage Night Shift with Flash Trade fees + compounding")
    parser.add_argument("--symbol", default="SOL/USDT")
    parser.add_argument("--leverage", default="3,4,5,6,7,8,9,10", help="Comma-separated leverage levels")
    parser.add_argument("--sl-atr", default=None, help="Comma-separated stop-loss ATR multipliers (default: auto per leverage)")
    parser.add_argument("--trail-atr", default=None, help="Comma-separated trailing stop ATR (default: auto per leverage)")
    parser.add_argument("--position-pct", type=float, default=0.20, help="Position size as fraction of capital")
    args = parser.parse_args()

    leverage_levels = [float(x) for x in args.leverage.split(",")]
    sl_levels = [float(x) for x in args.sl_atr.split(",")] if args.sl_atr else None
    trail_levels = [float(x) for x in args.trail_atr.split(",")] if args.trail_atr else None

    global POSITION_PCT
    POSITION_PCT = args.position_pct

    # Load data
    safe = args.symbol.replace("/", "_")
    path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
    if not os.path.exists(path):
        print(f"No data at {path}")
        sys.exit(1)

    df = pd.read_parquet(path)
    df = compute_indicators(df)
    total_days = len(df) / 24

    print(f"\n{'='*80}")
    print(f"LEVERAGE NIGHT SHIFT — Flash Trade Fee Model + Compounding")
    print(f"{'='*80}")
    print(f"Symbol:           {args.symbol}")
    print(f"Period:           {df.index[250].date()} -> {df.index[-1].date()} ({total_days:.0f} days)")
    print(f"Data:             {len(df)} candles")
    print(f"WFA:              {WFA_FOLDS} folds, {TEST_FOLD_DAYS}-day test windows")
    print(f"Position sizing:  {POSITION_PCT:.0%} of capital")
    print(f"Initial capital:  {INITIAL_CAPITAL} SOL")
    print(f"Flash Trade fees:")
    print(f"  Open:           {FLASH_OPEN_FEE_PCT}% of notional")
    print(f"  Close:          {FLASH_CLOSE_FEE_PCT}% of notional")
    print(f"  Hourly borrow:  {FLASH_HOURLY_BORROW_PCT}% of notional/hr")
    print(f"Leverage levels:  {leverage_levels}")
    print()

    # For each leverage, test with adapted stop-loss widths
    # Higher leverage needs wider stops to avoid premature stops
    # Default mapping (unless overridden):
    leverage_sl_trail = {
        3:  (2.5, 0.3),
        4:  (2.5, 0.3),
        5:  (2.5, 0.3),
        6:  (3.0, 0.3),
        7:  (3.0, 0.3),
        8:  (3.0, 0.3),
        9:  (3.0, 0.3),
        10: (3.0, 0.3),
    }

    results = []

    # Also run 1x baseline for reference
    print(f"Running 1x baseline...")
    baseline = run_wfa_leverage(df, 1.0, BASE_CONFIG, sl_atr=1.5, trail_atr=0.5)
    if baseline:
        results.append(baseline)
        print(f"  1x: {baseline['total_return_pct']:+.2f}%  DD={baseline['max_drawdown_pct']:.2f}%  "
              f"Trades={baseline['total_trades']}  Cons={baseline['consistency']:.0%}")

    for i, lev in enumerate(leverage_levels):
        sl = sl_levels[i] if sl_levels and i < len(sl_levels) else leverage_sl_trail.get(lev, (2.5, 0.3))[0]
        trail = trail_levels[i] if trail_levels and i < len(trail_levels) else leverage_sl_trail.get(lev, (2.5, 0.3))[1]

        print(f"Running {lev:.0f}x (sl={sl}, trail={trail})...", end="", flush=True)
        r = run_wfa_leverage(df, lev, BASE_CONFIG, sl_atr=sl, trail_atr=trail)
        if r:
            results.append(r)
            print(f"\r  {lev:.0f}x: {r['total_return_pct']:+.2f}%  DD={r['max_drawdown_pct']:.2f}%  "
                  f"Trades={r['total_trades']}  Liqs={r['liquidations']}  "
                  f"Cons={r['consistency']:.0%}  Fees={r['total_fees_sol']:.2f} SOL  "
                  f"Final={r['final_capital']:.2f} SOL")
        else:
            print(f"\r  {lev:.0f}x: insufficient data for folds")

    # === RESULTS TABLE ===
    print(f"\n{'='*80}")
    print(f"RESULTS — {args.symbol}, {POSITION_PCT:.0%} position, Flash Trade fees + compounding")
    print(f"{total_days:.0f}-day WFA ({WFA_FOLDS} folds × {TEST_FOLD_DAYS} days)")
    print(f"{'='*80}\n")

    print(f"{'Lev':<5} {'SL':<5} {'Tr':<5} {'Final':>10s} {'Return':>10s} {'Annual':>10s} "
          f"{'MaxDD':>8s} {'Sharpe':>8s} {'Trades':>7s} {'WR':>6s} {'Cons':>5s} {'Liq':>5s} {'Fees':>8s}")
    print("─" * 105)

    for r in sorted(results, key=lambda x: x["leverage"]):
        print(f"{r['leverage']:<5.0f}x {r['sl_atr']:<5.1f} {r['trail_atr']:<5.1f} "
              f"{r['final_capital']:>9.2f}S {r['total_return_pct']:>+9.2f}% "
              f"{r['annualized_return_pct']:>+9.2f}% {r['max_drawdown_pct']:>7.2f}% "
              f"{r['sharpe']:>7.2f} {r['total_trades']:>7d} {r['win_rate']:>5.0%} "
              f"{r['consistency']:>4.0%} {r['liquidations']:>5d} {r['total_fees_sol']:>7.2f}S")

    # === PER-FOLD DETAIL ===
    print(f"\n{'─'*80}")
    print(f"PER-FOLD DETAIL")
    print(f"{'─'*80}")
    for r in sorted(results, key=lambda x: x["leverage"]):
        tag = f"{r['leverage']:.0f}x"
        print(f"\n  {tag} — Final: {r['final_capital']:.2f} SOL, Return: {r['total_return_pct']:+.2f}%")
        print(f"  {'Fold':<6} {'Start':>8s} {'End':>8s} {'Return':>8s} {'Trades':>7s} {'DD':>6s} {'Liq':>4s} {'Fees':>7s}")
        print(f"  {'─'*6} {'─'*8} {'─'*8} {'─'*8} {'─'*7} {'─'*6} {'─'*4} {'─'*7}")
        for f in r["folds"]:
            print(f"  {f['fold']:<6} {f['start_capital']:>7.2f}S {f['end_capital']:>7.2f}S "
                  f"{f['return_pct']:>+7.2f}% {f['trades']:>7d} {f['max_dd_pct']:>5.1f}% "
                  f"{f['liquidations']:>4d} {f['fees_sol']:>6.2f}S")

    # === RECOMMENDATION ===
    print(f"\n{'='*80}")
    print(f"RECOMMENDATION")
    print(f"{'='*80}\n")

    # Find best risk-adjusted: highest return with < 20% max DD and 0 liquidations
    safe_results = [r for r in results if r["liquidations"] == 0 and r["max_drawdown_pct"] < 20]
    if not safe_results:
        # Relax: allow < 30% DD
        safe_results = [r for r in results if r["max_drawdown_pct"] < 30]

    if safe_results:
        best = max(safe_results, key=lambda r: r["total_return_pct"])
        baseline_r = next((r for r in results if r["leverage"] == 1.0), None)
        print(f"  Best risk-adjusted: {best['leverage']:.0f}x")
        print(f"    Return:   {best['total_return_pct']:+.2f}% (annualized {best['annualized_return_pct']:+.2f}%)")
        print(f"    Max DD:   {best['max_drawdown_pct']:.2f}%")
        print(f"    Sharpe:   {best['sharpe']:.2f}")
        print(f"    Trades:   {best['total_trades']}, Win rate: {best['win_rate']:.0%}")
        print(f"    Liqs:     {best['liquidations']}, Consistency: {best['consistency']:.0%}")
        print(f"    Fees:     {best['total_fees_sol']:.2f} SOL total")
        print(f"    Params:   sl={best['sl_atr']}, trail={best['trail_atr']}")

        if baseline_r:
            print(f"\n  vs 1x baseline:")
            print(f"    Return:   {baseline_r['total_return_pct']:+.2f}% → {best['total_return_pct']:+.2f}% "
                  f"({best['total_return_pct'] - baseline_r['total_return_pct']:+.2f}%)")
            print(f"    Max DD:   {baseline_r['max_drawdown_pct']:.2f}% → {best['max_drawdown_pct']:.2f}% "
                  f"({best['max_drawdown_pct'] - baseline_r['max_drawdown_pct']:+.2f}%)")

        # Show all levels ranked
        print(f"\n  All levels ranked by return/DD ratio:")
        ranked = sorted(results, key=lambda r: r["total_return_pct"] / max(r["max_drawdown_pct"], 0.1), reverse=True)
        for i, r in enumerate(ranked[:5]):
            ratio = r["total_return_pct"] / max(r["max_drawdown_pct"], 0.1)
            print(f"    {i+1}. {r['leverage']:.0f}x — Ret={r['total_return_pct']:+.1f}% DD={r['max_drawdown_pct']:.1f}% "
                  f"Ratio={ratio:.2f} Liqs={r['liquidations']}")

        if best["liquidations"] == 0 and best["max_drawdown_pct"] < 15:
            print(f"\n  VERDICT: GO — deploy {best['leverage']:.0f}x with sl={best['sl_atr']}, trail={best['trail_atr']}")
        elif best["liquidations"] == 0:
            print(f"\n  VERDICT: GO WITH CAUTION — acceptable risk at {best['leverage']:.0f}x")
        else:
            print(f"\n  VERDICT: RISKY — liquidations detected, reduce leverage")
    else:
        print(f"  No safe leverage level found. All levels have excessive DD or liquidations.")

    # Save
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    out_path = os.path.join(OUTPUT_DIR, f"leverage_night_shift_{safe}.json")
    with open(out_path, "w") as f:
        json.dump({
            "run_at": pd.Timestamp.now().isoformat(),
            "symbol": args.symbol,
            "flash_trade_fees": {
                "open_pct": FLASH_OPEN_FEE_PCT,
                "close_pct": FLASH_CLOSE_FEE_PCT,
                "hourly_borrow_pct": FLASH_HOURLY_BORROW_PCT,
            },
            "position_pct": POSITION_PCT,
            "initial_capital": INITIAL_CAPITAL,
            "wfa_folds": WFA_FOLDS,
            "test_fold_days": TEST_FOLD_DAYS,
            "results": results,
        }, f, indent=2, default=str)
    print(f"\n  Saved: {out_path}")


if __name__ == "__main__":
    main()
