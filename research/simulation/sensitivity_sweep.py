"""
Parameter Sensitivity Sweep for validated strategies.

For each parameter in param_ranges, varies it +/-20% in 5 steps while holding
all others at base_params. Runs full walk-forward validation for each variant.
Returns a DataFrame with columns:
    param_name, param_value, pct_change, oos_sharpe, win_rate, n_folds_passed

The goal is a "flat landscape" chart showing the strategy is robust,
not a magic-number fit. This is demo evidence.

Usage:
    python -m research.simulation.sensitivity_sweep --strategy sol_survivor_2_69
    python -m research.simulation.sensitivity_sweep --strategy sol_survivor_2_69 --output research/data/sweep.csv
"""
import argparse
import json
import os
import sys

import numpy as np
import pandas as pd

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))

from research.optimization.per_symbol_optimizer import (
    compute_indicators,
)
from research.orchestration.night_shift import (
    create_folds,
    evaluate_on_fold,
)

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "data", "ohlcv")
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "data")

# Named strategy configs
STRATEGY_CONFIGS = {
    "sol_survivor_2_69": {
        "symbol": "SOL/USDT",
        "params": {
            "signal_threshold": 0.3,
            "min_alignment": 3,
            "take_profit_atr": 3.0,
            "stop_loss_atr": 1.5,
            "max_hold_hours": 36,
            "time_decay_hours": 12,
            "trailing_stop_atr": 0.5,
            "score_flip_delay_hrs": 0,
        },
    },
    "production_baseline": {
        "symbol": "SOL/USDT",
        "params": {
            "signal_threshold": 0.4,
            "min_alignment": 3,
            "take_profit_atr": 6.0,
            "stop_loss_atr": 2.5,
            "max_hold_hours": 96,
            "time_decay_hours": 48,
            "trailing_stop_atr": 1.0,
            "score_flip_delay_hrs": 2,
        },
    },
}

WFA_CONFIG = {
    "num_folds": 9,
    "test_fold_days": 36,
}

SHARPE_CAP = 100.0


def run_sensitivity_sweep(
    base_params: dict,
    param_ranges: dict,
    symbol: str,
    n_folds: int = 9,
) -> pd.DataFrame:
    """Run sensitivity sweep: vary each param +/-20% in 5 steps.

    Args:
        base_params: The validated strategy config.
        param_ranges: Dict of param_name -> [values] or None to auto-generate.
                      If None, auto-generates 5 steps from -20% to +20%.
        symbol: e.g. "SOL/USDT"
        n_folds: Number of WFA folds.

    Returns:
        DataFrame with columns:
            param_name, param_value, pct_change, oos_sharpe, win_rate,
            n_folds_passed, total_pnl, max_dd, avg_trades_per_fold
    """
    # Load data
    safe = symbol.replace("/", "_")
    path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
    if not os.path.exists(path):
        raise FileNotFoundError(f"No data for {symbol} at {path}")
    df = pd.read_parquet(path)
    df = compute_indicators(df)

    # Create folds
    folds = create_folds(len(df), n_folds, WFA_CONFIG["test_fold_days"])
    print(f"  Loaded {symbol}: {len(df)} candles, {len(folds)} folds")

    rows = []

    # Baseline evaluation
    print(f"\n  Baseline evaluation...")
    baseline_fold_results = []
    for fold in folds:
        fm = evaluate_on_fold(df, fold, base_params, skip_is=True)
        baseline_fold_results.append(fm)

    baseline_sharpes = [
        max(-SHARPE_CAP, min(SHARPE_CAP, f["oos_sharpe"]))
        for f in baseline_fold_results
    ]
    baseline_median_sharpe = float(np.median(baseline_sharpes))
    baseline_wrs = [f["oos_wr"] for f in baseline_fold_results]
    baseline_pnls = [f["oos_pnl"] for f in baseline_fold_results]
    baseline_dds = [f["oos_max_dd"] for f in baseline_fold_results]
    baseline_trades = [f["oos_trades"] for f in baseline_fold_results]
    baseline_folds_passed = sum(1 for s in baseline_sharpes if s > 0)

    print(f"  Baseline: Sharpe={baseline_median_sharpe:+.2f} "
          f"WR={np.mean(baseline_wrs):.0%} "
          f"Folds={baseline_folds_passed}/{len(folds)} "
          f"PnL={sum(baseline_pnls):+.2f}%")

    rows.append({
        "param_name": "BASELINE",
        "param_value": 0,
        "pct_change": 0.0,
        "oos_sharpe": baseline_median_sharpe,
        "win_rate": float(np.mean(baseline_wrs)),
        "n_folds_passed": baseline_folds_passed,
        "total_pnl": float(sum(baseline_pnls)),
        "max_dd": float(np.mean(baseline_dds)),
        "avg_trades_per_fold": float(np.mean(baseline_trades)),
    })

    # Determine which params to sweep (skip discrete/non-numeric)
    skip_params = {"min_alignment"}
    sweepable = {
        k: v for k, v in base_params.items()
        if isinstance(v, (int, float)) and k not in skip_params
    }

    # If param_ranges provided, use those; otherwise auto-generate
    if param_ranges:
        sweep_params = param_ranges
    else:
        sweep_params = {}
        for k, v in sweepable.items():
            steps = np.linspace(-0.20, 0.20, 5)
            values = [round(v * (1 + s), 6) for s in steps]
            sweep_params[k] = values

    # Sweep each parameter
    for param_name, values in sweep_params.items():
        base_val = base_params.get(param_name)
        if base_val is None:
            print(f"  Skipping {param_name}: not in base_params")
            continue

        print(f"\n  Sweeping {param_name} (base={base_val})...")
        for val in values:
            pct_change = (val - base_val) / abs(base_val) * 100 if base_val != 0 else 0

            # Build variant params
            variant_params = dict(base_params)
            variant_params[param_name] = val

            # Evaluate on all folds
            fold_results = []
            for fold in folds:
                fm = evaluate_on_fold(df, fold, variant_params, skip_is=True)
                fold_results.append(fm)

            oos_sharpes = [
                max(-SHARPE_CAP, min(SHARPE_CAP, f["oos_sharpe"]))
                for f in fold_results
            ]
            median_sharpe = float(np.median(oos_sharpes))
            wrs = [f["oos_wr"] for f in fold_results]
            pnls = [f["oos_pnl"] for f in fold_results]
            dds = [f["oos_max_dd"] for f in fold_results]
            trades = [f["oos_trades"] for f in fold_results]
            folds_passed = sum(1 for s in oos_sharpes if s > 0)

            row = {
                "param_name": param_name,
                "param_value": val,
                "pct_change": round(pct_change, 1),
                "oos_sharpe": round(median_sharpe, 4),
                "win_rate": round(float(np.mean(wrs)), 4),
                "n_folds_passed": folds_passed,
                "total_pnl": round(float(sum(pnls)), 4),
                "max_dd": round(float(np.mean(dds)), 4),
                "avg_trades_per_fold": round(float(np.mean(trades)), 1),
            }
            rows.append(row)

            delta = median_sharpe - baseline_median_sharpe
            marker = "✓" if folds_passed == len(folds) else (
                "⚠" if folds_passed >= len(folds) * 0.7 else "✗"
            )
            print(f"    {param_name}={val:.4f} ({pct_change:+.1f}%) "
                  f"Sharpe={median_sharpe:+.2f} ({delta:+.2f}) "
                  f"Folds={folds_passed}/{len(folds)} {marker}")

    result_df = pd.DataFrame(rows)
    return result_df


def print_summary(df: pd.DataFrame):
    """Print a summary table of sensitivity results."""
    print(f"\n{'='*80}")
    print(f"SENSITIVITY SWEEP SUMMARY")
    print(f"{'='*80}")
    baseline_row = df[df["param_name"] == "BASELINE"].iloc[0]
    print(f"  Baseline Sharpe: {baseline_row['oos_sharpe']:+.2f}")
    print()

    # Group by param_name (excluding BASELINE)
    sweep_df = df[df["param_name"] != "BASELINE"].copy()
    if sweep_df.empty:
        print("  No sweep results to summarize.")
        return

    print(f"  {'Parameter':25s} {'Base':>8s} {'Min Sharpe':>10s} {'Max Sharpe':>10s} "
          f"{'Range':>8s} {'Robust':>7s}")
    print(f"  {'─'*25} {'─'*8} {'─'*10} {'─'*10} {'─'*8} {'─'*7}")

    baseline_sharpe = df[df["param_name"] == "BASELINE"].iloc[0]["oos_sharpe"]

    for param_name in sweep_df["param_name"].unique():
        param_rows = sweep_df[sweep_df["param_name"] == param_name]
        base_row = param_rows[param_rows["pct_change"].astype(float) == 0.0]
        if not base_row.empty:
            base_val = base_row.iloc[0]["param_value"]
        else:
            base_val = param_rows.iloc[len(param_rows) // 2]["param_value"]

        min_sharpe = param_rows["oos_sharpe"].min()
        max_sharpe = param_rows["oos_sharpe"].max()
        sharpe_range = max_sharpe - min_sharpe

        # Robust = all variants within 20% of baseline
        threshold = abs(baseline_sharpe) * 0.2
        all_within = all(
            abs(s - baseline_sharpe) <= max(threshold, 0.5)
            for s in param_rows["oos_sharpe"]
        )
        robust = "FLAT ✓" if all_within else "PEAKED"

        print(f"  {param_name:25s} {base_val:8.4f} {min_sharpe:+10.2f} {max_sharpe:+10.2f} "
              f"{sharpe_range:8.2f} {robust:>7s}")

    # Overall verdict
    print()
    all_ranges = []
    for param_name in sweep_df["param_name"].unique():
        param_rows = sweep_df[sweep_df.param_name == param_name]
        all_ranges.append(param_rows["oos_sharpe"].max() - param_rows["oos_sharpe"].min())

    avg_range = np.mean(all_ranges)
    if avg_range < 1.0:
        verdict = "ROBUST — flat landscape, strategy is not overfit to specific parameter values"
    elif avg_range < 3.0:
        verdict = "MODERATE — some parameters matter more than others, acceptable"
    else:
        verdict = "FRAGILE — sharp peaks detected, strategy may be overfit"

    print(f"  Average Sharpe range across params: {avg_range:.2f}")
    print(f"  Verdict: {verdict}")
    print()


def main():
    parser = argparse.ArgumentParser(
        description="Run parameter sensitivity sweep for a validated strategy"
    )
    parser.add_argument(
        "--strategy",
        required=True,
        choices=list(STRATEGY_CONFIGS.keys()),
        help="Named strategy to sweep",
    )
    parser.add_argument(
        "--output",
        default=None,
        help="Output CSV path (default: research/data/sensitivity_<strategy>.csv)",
    )
    args = parser.parse_args()

    config = STRATEGY_CONFIGS[args.strategy]
    symbol = config["symbol"]
    base_params = config["params"]

    print(f"\n{'='*80}")
    print(f"PARAMETER SENSITIVITY SWEEP")
    print(f"  Strategy: {args.strategy}")
    print(f"  Symbol:   {symbol}")
    print(f"  Params:   {json.dumps(base_params, indent=2)}")
    print(f"{'='*80}")

    # Run sweep with auto-generated +/-20% ranges
    df = run_sensitivity_sweep(base_params, None, symbol, n_folds=9)

    # Print summary
    print_summary(df)

    # Save
    if args.output is None:
        output_path = os.path.join(OUTPUT_DIR, f"sensitivity_{args.strategy}.csv")
    else:
        output_path = args.output

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    df.to_csv(output_path, index=False)
    print(f"  Saved to: {output_path}")


if __name__ == "__main__":
    main()
