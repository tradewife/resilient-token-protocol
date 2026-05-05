"""
Leverage Sweep — find the optimal leverage × stop-loss combination.

Sweeps leverage levels against stop-loss ATR multipliers (and optionally
trailing stops) through the same 9-fold WFA used by the night shift.
Models liquidation at 100% margin loss.

The fast simulator (per_symbol_optimizer) already handles leverage in PnL
calculation — this script just constructs the parameter grid and runs WFA.

Usage:
    python -m research.simulation.leverage_sweep
    python -m research.simulation.leverage_sweep --symbol SOL/USDT
    python -m research.simulation.leverage_sweep --leverage 1,2,3,5,7,10 --sl-atr 0.5,1.0,1.5,2.0
    python -m research.simulation.leverage_sweep --output data/leverage_sweep/results.csv
"""
import argparse
import json
import os
import sys
from itertools import product

import numpy as np
import pandas as pd

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))

from research.optimization.per_symbol_optimizer import (
    compute_indicators,
    simulate_trades,
    compute_metrics,
)
from research.orchestration.night_shift import (
    create_folds,
    evaluate_on_fold,
)

ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
DATA_DIR = os.path.join(ROOT, "data", "ohlcv")
OUTPUT_DIR = os.path.join(ROOT, "data", "leverage_sweep")

# Survivor 2.69 base config — the strategy already live in rtp-trader.
BASE_CONFIG = {
    "signal_threshold": 0.3,
    "min_alignment": 3,
    "take_profit_atr": 3.0,
    "stop_loss_atr": 1.5,
    "max_hold_hours": 36,
    "time_decay_hours": 12,
    "trailing_stop_atr": 0.5,
}

# Default sweep ranges
DEFAULT_LEVERAGE = [1, 2, 3, 5, 7, 10]
DEFAULT_SL_ATR = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0]
DEFAULT_TRAILING = [0.0, 0.3, 0.5, 0.8]

WFA_CONFIG = {
    "num_folds": 9,
    "test_fold_days": 36,
}

SHARPE_CAP = 100.0
MAX_DD_CEILING = 25.0        # hard reject if any fold exceeds this leveraged DD
MIN_CONSISTENCY = 0.67       # 6/9 folds profitable
MAX_LIQUIDATIONS_PER_FOLD = 1


def load_symbol(symbol: str) -> pd.DataFrame:
    safe = symbol.replace("/", "_")
    path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
    if not os.path.exists(path):
        raise FileNotFoundError(f"No data for {symbol}: {path}")
    df = pd.read_parquet(path)
    df = compute_indicators(df)
    return df


def evaluate_leveraged_candidate(df, folds, params, symbol):
    """Full 9-fold WFA for one leveraged candidate."""
    fold_results = []
    for fold in folds:
        fm = evaluate_on_fold(df, fold, params)
        fold_results.append(fm)

    oos_sharpes_raw = [f["oos_sharpe"] for f in fold_results]
    oos_sharpes = [max(-SHARPE_CAP, min(SHARPE_CAP, s)) for s in oos_sharpes_raw]
    oos_pnls = [f["oos_pnl"] for f in fold_results]
    oos_dds = [f["oos_max_dd"] for f in fold_results]
    oos_trades = [f["oos_trades"] for f in fold_results]
    oos_exits = {}
    liquidations = 0
    for f in fold_results:
        for reason, count in f.get("oos_exit_reasons", {}).items():
            oos_exits[reason] = oos_exits.get(reason, 0) + count
            if reason == "liquidation":
                liquidations += count

    avg_oos_sharpe = float(np.median(oos_sharpes))
    avg_oos_pnl = float(np.sum(oos_pnls))
    avg_oos_dd = float(np.mean(oos_dds))
    avg_oos_trades = float(np.mean(oos_trades))
    positive_folds = sum(1 for s in oos_sharpes if s > 0)
    consistency = positive_folds / len(oos_sharpes)

    rejected = False
    reject_reason = ""
    if avg_oos_dd > MAX_DD_CEILING:
        rejected = True
        reject_reason = f"avg_dd={avg_oos_dd:.1f}%>{MAX_DD_CEILING}%"
    if consistency < MIN_CONSISTENCY:
        rejected = True
        reject_reason += f" consistency={consistency:.0%}<{MIN_CONSISTENCY:.0%}"
    if liquidations > MAX_LIQUIDATIONS_PER_FOLD * len(folds):
        rejected = True
        reject_reason += f" liquidations={liquidations}>{MAX_LIQUIDATIONS_PER_FOLD * len(folds)}"

    # Survivor score (same formula as night shift)
    dd_factor = 1.0 / (1.0 + avg_oos_dd / 100)
    trade_factor = min(avg_oos_trades / 10, 1.0)
    survivor = avg_oos_sharpe * consistency * dd_factor * trade_factor

    return {
        "symbol": symbol,
        "leverage": params["leverage"],
        "stop_loss_atr": params["stop_loss_atr"],
        "trailing_stop_atr": params["trailing_stop_atr"],
        "oos_sharpe": round(avg_oos_sharpe, 3),
        "oos_pnl": round(avg_oos_pnl, 2),
        "oos_dd": round(avg_oos_dd, 2),
        "consistency": round(consistency, 3),
        "avg_trades": round(avg_oos_trades, 1),
        "liquidations": liquidations,
        "survivor": round(survivor, 3),
        "rejected": rejected,
        "reject_reason": reject_reason.strip(),
        "exit_reasons": oos_exits,
        "params": {k: v for k, v in params.items()},
    }


def run_sweep(symbol, leverage_levels, sl_atr_levels, trailing_levels):
    df = load_symbol(symbol)
    folds = create_folds(
        len(df),
        num_folds=WFA_CONFIG["num_folds"],
        test_fold_days=WFA_CONFIG["test_fold_days"],
    )
    if not folds:
        print(f"[ERROR] Insufficient data for {symbol} to create folds")
        return []

    print(f"[SWEEP] {symbol}: {len(df)} bars, {len(folds)} folds")
    print(f"[SWEEP] Grid: {len(leverage_levels)} lev × {len(sl_atr_levels)} SL × {len(trailing_levels)} trail = "
          f"{len(leverage_levels) * len(sl_atr_levels) * len(trailing_levels)} candidates")

    results = []
    combos = list(product(leverage_levels, sl_atr_levels, trailing_levels))
    for i, (lev, sl, trail) in enumerate(combos):
        params = {**BASE_CONFIG, "leverage": lev, "stop_loss_atr": sl, "trailing_stop_atr": trail}
        r = evaluate_leveraged_candidate(df, folds, params, symbol)
        results.append(r)

        if (i + 1) % 50 == 0 or i == len(combos) - 1:
            passed = sum(1 for x in results if not x["rejected"])
            best = max((x["survivor"] for x in results if not x["rejected"]), default=0)
            print(f"  [{i+1}/{len(combos)}] passed={passed} best_survivor={best:.3f}")

    return results


def main():
    parser = argparse.ArgumentParser(description="Leverage sweep for Survivor 2.69")
    parser.add_argument("--symbol", default="SOL/USDT")
    parser.add_argument("--leverage", default=None, help="Comma-separated leverage levels")
    parser.add_argument("--sl-atr", default=None, help="Comma-separated stop-loss ATR multipliers")
    parser.add_argument("--trailing", default=None, help="Comma-separated trailing stop ATR multipliers")
    parser.add_argument("--output", default=None, help="Output CSV path")
    args = parser.parse_args()

    leverage_levels = [float(x) for x in args.leverage.split(",")] if args.leverage else DEFAULT_LEVERAGE
    sl_atr_levels = [float(x) for x in args.sl_atr.split(",")] if args.sl_atr else DEFAULT_SL_ATR
    trailing_levels = [float(x) for x in args.trailing.split(",")] if args.trailing else DEFAULT_TRAILING

    results = run_sweep(args.symbol, leverage_levels, sl_atr_levels, trailing_levels)

    if not results:
        print("[ERROR] No results")
        return

    # Save CSV
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    out_path = args.output or os.path.join(OUTPUT_DIR, f"leverage_sweep_{args.symbol.replace('/', '_')}.csv")

    flat = []
    for r in results:
        row = {k: v for k, v in r.items() if k not in ("exit_reasons", "params")}
        flat.append(row)
    pd.DataFrame(flat).to_csv(out_path, index=False)
    print(f"\n[SAVED] {out_path} ({len(flat)} rows)")

    # Print summary
    passed = [r for r in results if not r["rejected"]]
    passed.sort(key=lambda x: x["survivor"], reverse=True)

    print(f"\n{'='*80}")
    print(f"LEVERAGE SWEEP RESULTS — {args.symbol}")
    print(f"{'='*80}")
    print(f"Total candidates: {len(results)}")
    print(f"Passed filters:   {len(passed)}")

    if not passed:
        print("\nNo candidates passed filters. Relaxing to top 10 by survivor score:")
        top = sorted(results, key=lambda x: x["survivor"], reverse=True)[:10]
    else:
        top = passed[:10]

    print(f"\n{'Rank':<5} {'Lev':<5} {'SL':<6} {'Trail':<6} {'Sharpe':<8} {'PnL':<10} {'DD%':<8} {'Cons':<6} {'Liq':<5} {'Surv':<8}")
    print("-" * 75)
    for i, r in enumerate(top):
        print(f"{i+1:<5} {r['leverage']:<5.0f} {r['stop_loss_atr']:<6.2f} {r['trailing_stop_atr']:<6.1f} "
              f"{r['oos_sharpe']:<8.2f} {r['oos_pnl']:<10.1f} {r['oos_dd']:<8.1f} "
              f"{r['consistency']:<6.0%} {r['liquidations']:<5} {r['survivor']:<8.3f}"
              f"{'  REJECTED' if r['rejected'] else ''}")

    # Save JSON report
    report_path = out_path.replace(".csv", "_report.json")
    report = {
        "symbol": args.symbol,
        "base_config": BASE_CONFIG,
        "sweep_grid": {
            "leverage": leverage_levels,
            "stop_loss_atr": sl_atr_levels,
            "trailing_stop_atr": trailing_levels,
        },
        "filters": {
            "max_dd_ceiling": MAX_DD_CEILING,
            "min_consistency": MIN_CONSISTENCY,
            "max_liquidations_per_fold": MAX_LIQUIDATIONS_PER_FOLD,
        },
        "total_candidates": len(results),
        "passed": len(passed),
        "top_10": top,
    }
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2, default=str)
    print(f"\n[SAVED] {report_path}")


if __name__ == "__main__":
    main()
