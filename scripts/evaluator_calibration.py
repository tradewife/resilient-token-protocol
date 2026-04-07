"""
Evaluator Calibration Module

Compares fast simulator vs full simulator (FutureBlindSimulator) on a
sample of random configs to detect systematic bias. Outputs a calibration
report that the night shift can use to adjust its evaluation.

This is the first "self-awareness" module — the system detecting when its
own evaluation layer is unreliable.

Usage:
    python scripts/evaluator_calibration.py                    # all symbols, 20 samples
    python scripts/evaluator_calibration.py --symbol SOL/USDT  # single symbol
    python scripts/evaluator_calibration.py --samples 50      # more samples
"""

import argparse
import asyncio
import json
import os
import random
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from scripts.per_symbol_optimizer import (
    compute_indicators,
    simulate_trades,
    compute_metrics,
)


# ── Param space for random sampling ──
PARAM_RANGES = {
    "signal_threshold": [0.25, 0.30, 0.35, 0.40, 0.45, 0.50],
    "min_alignment": [2, 3],
    "take_profit_atr": [2.0, 3.0, 4.0, 5.0, 6.0],
    "stop_loss_atr": [1.0, 1.25, 1.5, 2.0, 2.5],
    "max_hold_hours": [24, 36, 48, 72, 96],
    "time_decay_hours": [24, 36, 48],
    "trailing_stop_atr": [0.0, 0.5, 0.7, 1.0],
    "score_flip_delay_hrs": [0, 1, 2, 4],
}

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")
REPORT_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "calibration")


def random_config() -> dict:
    """Generate a random parameter set from the grid."""
    return {k: random.choice(v) for k, v in PARAM_RANGES.items()}


def fast_sim_eval(df: pd.DataFrame, params: dict, folds: list) -> dict:
    """Run fast simulator on all folds. Returns aggregated metrics."""
    df_ind = compute_indicators(df)
    fold_results = []

    for fold_num, train_end, test_end in folds:
        test_df = df_ind.iloc[train_end:test_end]
        test_hours = test_end - train_end
        trips = simulate_trades(test_df, params)
        m = compute_metrics(trips, total_hours=test_hours)
        fold_results.append(m)

    sharpes = [f["sharpe"] for f in fold_results]
    pnls = [f["total_pnl_pct"] for f in fold_results]
    trades = [f["round_trips"] for f in fold_results]

    return {
        "median_sharpe": float(np.median(sharpes)),
        "total_pnl": float(sum(pnls)),
        "consistency": sum(1 for s in sharpes if s > 0) / len(sharpes),
        "total_trades": sum(trades),
    }


async def full_sim_eval(symbol: str, df: pd.DataFrame, params: dict,
                        folds: list) -> dict:
    """Run full simulator on all folds. Returns aggregated metrics."""
    from scripts.validate_night_shift import backtest_fold

    fold_results = []
    for fold_num, train_end, test_end in folds:
        m = await backtest_fold(symbol, df, params, train_end, test_end)
        if m:
            fold_results.append(m)

    if not fold_results:
        return {"median_sharpe": 0, "total_pnl": 0, "consistency": 0, "total_trades": 0}

    sharpes = [f["sharpe"] for f in fold_results]
    pnls = [f["total_pnl_pct"] for f in fold_results]
    trades = [f["round_trips"] for f in fold_results]

    return {
        "median_sharpe": float(np.median(sharpes)),
        "total_pnl": float(sum(pnls)),
        "consistency": sum(1 for p in pnls if p > 0) / len(pnls),
        "total_trades": sum(trades),
    }


def make_folds(total_bars: int) -> list:
    """Create expanding-window WFA folds matching night_shift.py."""
    test_bars = 36 * 24  # 36 days
    warmup = 250
    usable = total_bars - warmup
    num_folds = min(9, usable // test_bars)
    if num_folds < 1:
        return [(0, warmup, total_bars)]

    bars_per_fold = test_bars if num_folds < (usable // test_bars) else usable // num_folds
    folds = []
    start = warmup
    for i in range(num_folds):
        end = start + bars_per_fold if i < num_folds - 1 else total_bars
        folds.append((i, start, end))
        start = end
    return folds


async def calibrate_symbol(symbol: str, n_samples: int, full_sim: bool = True) -> dict:
    """Compare fast sim vs full sim on random configs for one symbol."""
    safe = symbol.replace("/", "_")
    path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
    if not os.path.exists(path):
        print(f"  No data for {symbol}, skipping")
        return None

    df = pd.read_parquet(path)
    folds = make_folds(len(df))
    print(f"  {symbol}: {len(df)} bars, {len(folds)} folds, {n_samples} samples")

    results = []
    for i in range(n_samples):
        params = random_config()
        fast = fast_sim_eval(df, params, folds)

        if full_sim:
            f = await full_sim_eval(symbol, df, params, folds)
        else:
            f = None

        results.append({
            "params": params,
            "fast": fast,
            "full": f,
        })

        if (i + 1) % 5 == 0:
            print(f"    [{i+1}/{n_samples}]")

    # Analyze discrepancies
    sign_agreements = 0
    rank_correlations = []
    fast_pnls = [r["fast"]["total_pnl"] for r in results if r["full"] is not None]
    full_pnls = [r["full"]["total_pnl"] for r in results if r["full"] is not None]

    for r in results:
        if r["full"] is None:
            continue
        fp, fup = r["fast"]["total_pnl"], r["full"]["total_pnl"]
        if (fp > 0) == (fup > 0):
            sign_agreements += 1

    n_both = len(fast_pnls)
    if n_both == 0:
        # No full sim results — report fast sim diversity only
        all_fast_pnls = [r["fast"]["total_pnl"] for r in results]
        all_fast_sharpes = [r["fast"]["median_sharpe"] for r in results]
        positive_fast = sum(1 for p in all_fast_pnls if p > 0)
        return {
            "symbol": symbol,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "n_samples": n_samples,
            "mode": "fast_only",
            "fast_positive_rate": positive_fast / len(results),
            "fast_pnl_range": [round(min(all_fast_pnls), 1), round(max(all_fast_pnls), 1)],
            "fast_sharpe_range": [round(min(all_fast_sharpes), 2), round(max(all_fast_sharpes), 2)],
            "verdict": f"Fast sim diversity: {positive_fast}/{len(results)} configs positive",
            "samples": results[:10],
        }
    sign_accuracy = sign_agreements / n_both if n_both > 0 else 0

    # PnL bias: does fast sim systematically under/overestimate?
    if fast_pnls and full_pnls:
        bias = np.mean([f - fu for f, fu in zip(fast_pnls, full_pnls)])
        # Correlation
        if np.std(fast_pnls) > 0 and np.std(full_pnls) > 0:
            corr = np.corrcoef(fast_pnls, full_pnls)[0, 1]
        else:
            corr = 0.0
    else:
        bias = 0.0
        corr = 0.0

    # Sharpe agreement
    fast_sharpes = [r["fast"]["median_sharpe"] for r in results if r["full"] is not None]
    full_sharpes = [r["full"]["median_sharpe"] for r in results if r["full"] is not None]
    sharpe_agreements = sum(
        1 for fs, fus in zip(fast_sharpes, full_sharpes)
        if (fs > 0) == (fus > 0)
    )
    sharpe_accuracy = sharpe_agreements / len(fast_sharpes) if fast_sharpes else 0

    return {
        "symbol": symbol,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "n_samples": n_samples,
        "sign_agreement": round(sign_accuracy, 3),
        "sharpe_agreement": round(sharpe_accuracy, 3),
        "pnl_bias": round(bias, 2),
        "pnl_correlation": round(corr, 3),
        "fast_pnl_range": [round(min(fast_pnls), 1), round(max(fast_pnls), 1)] if fast_pnls else None,
        "full_pnl_range": [round(min(full_pnls), 1), round(max(full_pnls), 1)] if full_pnls else None,
        "verdict": _verdict(sign_accuracy, corr, bias),
        "samples": results[:10],  # keep first 10 for inspection
    }


def _verdict(sign_accuracy: float, corr: float, bias: float) -> str:
    """Generate a human-readable verdict."""
    if sign_accuracy >= 0.85 and corr >= 0.7:
        return "GOOD — fast sim is a reliable filter"
    elif sign_accuracy >= 0.70:
        return "ACCEPTABLE — fast sim mostly agrees, minor calibration needed"
    elif sign_accuracy >= 0.50:
        return "WARNING — fast sim disagrees on 30%+ of configs, investigate"
    else:
        return "BROKEN — fast sim is unreliable, do not trust rankings"


async def main():
    parser = argparse.ArgumentParser(description="Calibrate fast sim vs full sim")
    parser.add_argument("--symbol", default=None, help="Single symbol to test")
    parser.add_argument("--samples", type=int, default=20, help="Number of random configs per symbol")
    parser.add_argument("--fast-only", action="store_true", help="Skip full sim (just test fast sim diversity)")
    args = parser.parse_args()

    symbols = [args.symbol] if args.symbol else ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT"]

    print("=" * 70)
    print("EVALUATOR CALIBRATION — Fast Sim vs Full Sim")
    print(f"Samples: {args.samples} per symbol | Full sim: {'OFF' if args.fast_only else 'ON'}")
    print("=" * 70)

    os.makedirs(REPORT_DIR, exist_ok=True)

    all_results = {}
    for symbol in symbols:
        print(f"\nCalibrating {symbol}...")
        result = await calibrate_symbol(symbol, args.samples, full_sim=not args.fast_only)
        if result:
            all_results[symbol] = result
            print(f"  Sign agreement: {result['sign_agreement']:.0%}")
            print(f"  Sharpe agreement: {result['sharpe_agreement']:.0%}")
            print(f"  PnL bias: {result['pnl_bias']:+.1f}% (fast vs full)")
            print(f"  PnL correlation: {result['pnl_correlation']:.3f}")
            print(f"  Verdict: {result['verdict']}")

    # Save report
    report = {
        "calibrated_at": datetime.now(timezone.utc).isoformat(),
        "n_samples": args.samples,
        "results": all_results,
    }
    report_path = os.path.join(REPORT_DIR, f"calibration_{datetime.now().strftime('%Y-%m-%d')}.json")
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2, default=str)
    print(f"\nReport saved: {report_path}")


if __name__ == "__main__":
    asyncio.run(main())
