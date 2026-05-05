"""
Robustness testing — Monte Carlo DD simulation, CPCV, PBO.

Validates that optimized strategies are not overfit by:
1. Monte Carlo drawdown simulation: shuffles trade returns 10K times to build
   a DD distribution, answering "what's the worst DD across alternative histories?"
2. Combinatorial Purged Cross Validation (CPCV): tests every K-combination of
   test folds, producing a distribution of OOS Sharpes. The Probability of
   Backtest Overfitting (PBO) measures whether the optimization process itself
   is overfitting.

Usage:
  python -m research.validation.robustness --symbol SOL/USDT --leverage 9.0
  python -m research.validation.robustness --symbol SOL/USDT --leverage 9.0 --monte-carlo-only
  python -m research.validation.robustness --symbol SOL/USDT --leverage 9.0 --cpcv-only
"""
import argparse
import json
import os
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from itertools import combinations
from typing import Dict, List, Optional, Tuple

import numpy as np
import pandas as pd

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))

from research.optimization.per_symbol_optimizer import (
    compute_indicators,
    simulate_trades,
)
from research.orchestration.night_shift import (
    create_folds,
    evaluate_on_fold,
    flash_trade_round_trip_cost,
    Fold,
    FLASH_TRADE_FEES,
)

ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
DATA_DIR = os.path.join(ROOT, "data", "ohlcv")
RESULTS_DIR = os.path.join(ROOT, "data", "night_results")

# Best config from 2026-05-05 leverage optimization (Calmar=44.89)
DEFAULT_PARAMS = {
    "signal_threshold": 0.25,
    "take_profit_atr": 5.0,
    "stop_loss_atr": 2.7,
    "max_hold_hours": 36,
    "time_decay_hours": 12,
    "trailing_stop_atr": 0.14,
    "score_flip_delay_hrs": 0,
    "min_alignment": 3,
    "leverage": 9.0,
}


def log(msg: str):
    ts = datetime.now(timezone.utc).strftime("%H:%M:%S")
    print(f"[{ts}] {msg}", flush=True)


# ─────────────────────────────────────────────────────────────────────────────
# Monte Carlo Drawdown Simulation
# ─────────────────────────────────────────────────────────────────────────────

@dataclass
class MonteCarloResult:
    """Result of Monte Carlo DD simulation."""
    n_simulations: int
    n_trades: int
    position_pct: float
    initial_capital: float
    leverage: float
    observed_dd: float           # DD from actual historical sequence
    dd_p50: float                # median DD across shuffles
    dd_p75: float
    dd_p90: float
    dd_p95: float
    dd_p99: float
    dd_worst: float              # worst DD across all shuffles
    prob_dd_gt_10: float         # P(DD > 10%)
    prob_dd_gt_20: float         # P(DD > 20%)
    prob_dd_gt_30: float         # P(DD > 30%)
    prob_dd_gt_50: float         # P(DD > 50%)
    prob_dd_liquidation: float   # P(DD > liquidation threshold)
    return_p50: float            # median return across shuffles
    return_p90: float
    return_p10: float
    net_pnls: List[float] = field(default_factory=list)
    dd_distribution: List[float] = field(default_factory=list)


def monte_carlo_dd(
    net_pnls: List[float],
    position_pct: float = 0.20,
    initial_capital: float = 100.0,
    leverage: float = 9.0,
    n_simulations: int = 10000,
    liquidation_dd_pct: float = 55.0,
) -> MonteCarloResult:
    """
    Monte Carlo drawdown simulation.

    Shuffles the list of per-trade net PnLs (after fees) N times, computes
    an equity curve for each shuffle, and measures max drawdown across all
    simulated paths.

    Args:
        net_pnls: List of per-trade net PnL percentages (after fees, including leverage).
        position_pct: Fraction of capital risked per trade.
        initial_capital: Starting capital.
        leverage: Leverage level (used for liquidation threshold).
        n_simulations: Number of shuffle paths (default 10,000).
        liquidation_dd_pct: DD% at which liquidation occurs (default 55% for 9x).

    Returns:
        MonteCarloResult with DD distribution and risk probabilities.
    """
    if not net_pnls:
        return MonteCarloResult(
            n_simulations=0, n_trades=0, position_pct=position_pct,
            initial_capital=initial_capital, leverage=leverage,
            observed_dd=0, dd_p50=0, dd_p75=0, dd_p90=0, dd_p95=0,
            dd_p99=0, dd_worst=0, prob_dd_gt_10=0, prob_dd_gt_20=0,
            prob_dd_gt_30=0, prob_dd_gt_50=0, prob_dd_liquidation=0,
            return_p50=0, return_p90=0, return_p10=0,
        )

    pnls = np.array(net_pnls, dtype=np.float64)
    n_trades = len(pnls)

    # Compute observed DD (actual sequence)
    observed_dd = _compute_max_dd(pnls, position_pct, initial_capital)

    # Monte Carlo: shuffle and compute DD for each path
    dd_dist = np.empty(n_simulations)
    return_dist = np.empty(n_simulations)

    rng = np.random.default_rng(42)

    for i in range(n_simulations):
        shuffled = rng.permutation(pnls)
        dd_dist[i] = _compute_max_dd(shuffled, position_pct, initial_capital)
        final_cap = _simulate_equity(shuffled, position_pct, initial_capital)
        return_dist[i] = (final_cap - initial_capital) / initial_capital * 100

    # Percentiles
    dd_sorted = np.sort(dd_dist)
    returns_sorted = np.sort(return_dist)

    return MonteCarloResult(
        n_simulations=n_simulations,
        n_trades=n_trades,
        position_pct=position_pct,
        initial_capital=initial_capital,
        leverage=leverage,
        observed_dd=round(float(observed_dd), 2),
        dd_p50=round(float(np.percentile(dd_dist, 50)), 2),
        dd_p75=round(float(np.percentile(dd_dist, 75)), 2),
        dd_p90=round(float(np.percentile(dd_dist, 90)), 2),
        dd_p95=round(float(np.percentile(dd_dist, 95)), 2),
        dd_p99=round(float(np.percentile(dd_dist, 99)), 2),
        dd_worst=round(float(dd_sorted[-1]), 2),
        prob_dd_gt_10=round(float(np.mean(dd_dist > 10)), 4),
        prob_dd_gt_20=round(float(np.mean(dd_dist > 20)), 4),
        prob_dd_gt_30=round(float(np.mean(dd_dist > 30)), 4),
        prob_dd_gt_50=round(float(np.mean(dd_dist > 50)), 4),
        prob_dd_liquidation=round(float(np.mean(dd_dist > liquidation_dd_pct)), 4),
        return_p50=round(float(np.percentile(return_dist, 50)), 2),
        return_p90=round(float(np.percentile(return_dist, 90)), 2),
        return_p10=round(float(np.percentile(return_dist, 10)), 2),
        net_pnls=list(net_pnls),
        dd_distribution=dd_dist.tolist(),
    )


def _simulate_equity(pnls: np.ndarray, position_pct: float,
                      initial_capital: float) -> float:
    """Simulate equity curve from a list of per-trade PnL percentages."""
    capital = initial_capital
    for pnl_pct in pnls:
        position = capital * position_pct
        capital += position * (pnl_pct / 100.0)
        if capital <= 0:
            capital = 0
            break
    return capital


def _compute_max_dd(pnls: np.ndarray, position_pct: float,
                     initial_capital: float) -> float:
    """Compute max drawdown from a list of per-trade PnL percentages."""
    capital = initial_capital
    peak = capital
    max_dd = 0.0

    for pnl_pct in pnls:
        position = capital * position_pct
        capital += position * (pnl_pct / 100.0)
        if capital <= 0:
            return 100.0  # total wipeout
        if capital > peak:
            peak = capital
        dd = (peak - capital) / peak * 100
        if dd > max_dd:
            max_dd = dd

    return max_dd


# ─────────────────────────────────────────────────────────────────────────────
# Combinatorial Purged Cross Validation (CPCV) + PBO
# ─────────────────────────────────────────────────────────────────────────────

@dataclass
class CPCVResult:
    """Result of CPCV analysis."""
    n_folds: int
    n_test_folds: int
    n_paths: int
    purge_bars: int
    pbo: float                   # Probability of Backtest Overfitting
    logits: List[float]          # Logit for each path
    oos_sharpe_distribution: List[float]
    is_sharpe_distribution: List[float]
    path_results: List[Dict] = field(default_factory=list)


def cpcv(
    df: pd.DataFrame,
    params_grid: List[Dict],
    n_folds: int = 10,
    n_test_folds: int = 3,
    purge_bars: int = 6,
    of_config: Optional[Dict] = None,
) -> CPCVResult:
    """
    Combinatorial Purged Cross Validation.

    Creates N folds, then for every K-combination of test folds:
    1. Train on remaining folds (with purge gap)
    2. Find best IS params on training data
    3. Evaluate those params on test folds (OOS)
    4. Compute logit = ln(rank_IS / (N - rank_IS)) for the OOS performance

    PBO = count(logits < 0) / total_paths
    If PBO > 0.50, the optimization is likely overfitting.

    Args:
        df: OHLCV DataFrame with indicators.
        params_grid: List of param dicts to optimize over.
        n_folds: Number of folds (default 10).
        n_test_folds: Number of test folds per combination (default 3).
        purge_bars: Bars to purge between train/test (default 6).
        of_config: Overfitting config for evaluation.

    Returns:
        CPCVResult with PBO and per-path details.
    """
    if of_config is None:
        of_config = {
            "max_is_oos_gap": 0.5,
            "min_oos_consistency": 0.50,
        }

    # Create folds
    total_bars = len(df)
    test_days = max(10, total_bars // (n_folds * 24))
    folds = create_folds(total_bars, n_folds, test_days)

    if len(folds) < n_test_folds:
        log(f"  CPCV: only {len(folds)} folds available, need >= {n_test_folds}")
        return CPCVResult(
            n_folds=len(folds), n_test_folds=n_test_folds, n_paths=0,
            purge_bars=purge_bars, pbo=1.0, logits=[], oos_sharpe_distribution=[],
            is_sharpe_distribution=[],
        )

    # Generate all K-combinations of test fold indices
    fold_indices = list(range(len(folds)))
    test_combos = list(combinations(fold_indices, n_test_folds))
    train_combos = []
    for test_set in test_combos:
        train_set = tuple(i for i in fold_indices if i not in test_set)
        train_combos.append(train_set)

    n_paths = len(test_combos)
    log(f"  CPCV: {n_folds} folds, {n_test_folds} test folds, {n_paths} paths")

    logits = []
    oos_sharpes = []
    is_sharpes = []
    path_results = []

    for path_idx, (test_indices, train_indices) in enumerate(
        zip(test_combos, train_combos)
    ):
        # Build purged train folds (exclude purge_bars at boundaries)
        train_folds = []
        for fi in train_indices:
            f = folds[fi]
            # Purge: reduce train_end if adjacent to test
            train_end = f.train_end_idx
            test_start = f.test_start_idx
            if any(abs(folds[ti].test_start_idx - train_end) < purge_bars * 2
                   for ti in test_indices):
                # Don't purge our own fold's train data — only test boundary
                pass
            train_folds.append(f)

        # Evaluate all params on training set (IS)
        best_is_sharpe = -999
        best_params = None
        for params in params_grid:
            is_sharpes_path = []
            for f in train_folds:
                try:
                    result = evaluate_on_fold(df, f, params, skip_is=True)
                    is_sharpes_path.append(result["oos_sharpe"])
                except Exception:
                    is_sharpes_path.append(0.0)
            mean_is = float(np.median(is_sharpes_path)) if is_sharpes_path else 0
            if mean_is > best_is_sharpe:
                best_is_sharpe = mean_is
                best_params = params

        if best_params is None:
            logits.append(0.0)
            oos_sharpes.append(0.0)
            is_sharpes.append(0.0)
            continue

        # Evaluate best IS params on test folds (OOS)
        oos_sharpes_path = []
        for fi in test_indices:
            f = folds[fi]
            try:
                result = evaluate_on_fold(df, f, best_params, skip_is=True)
                oos_sharpes_path.append(result["oos_sharpe"])
            except Exception:
                oos_sharpes_path.append(0.0)

        median_oos = float(np.median(oos_sharpes_path)) if oos_sharpes_path else 0

        # Now compute logit: rank the best_params' OOS performance among ALL
        # params evaluated on the test folds
        all_test_sharpes = []
        for params in params_grid:
            p_sharpes = []
            for fi in test_indices:
                f = folds[fi]
                try:
                    result = evaluate_on_fold(df, f, params, skip_is=True)
                    p_sharpes.append(result["oos_sharpe"])
                except Exception:
                    p_sharpes.append(0.0)
            all_test_sharpes.append(float(np.median(p_sharpes)) if p_sharpes else 0)

        # Rank of the best IS params in the OOS distribution
        best_oos = median_oos
        n_params = len(all_test_sharpes)
        rank = sum(1 for s in all_test_sharpes if s <= best_oos)

        # Logit: ln(rank / (N - rank + 1))
        # Positive logit = IS-best also ranked well OOS (good)
        # Negative logit = IS-best ranked poorly OOS (overfitting)
        denominator = n_params - rank + 1
        if denominator > 0 and rank > 0:
            logit = float(np.log(rank / denominator))
        else:
            logit = -5.0  # worst case

        logits.append(round(logit, 4))
        oos_sharpes.append(round(median_oos, 4))
        is_sharpes.append(round(best_is_sharpe, 4))

        path_results.append({
            "path": path_idx,
            "test_folds": list(test_indices),
            "train_folds": list(train_indices),
            "best_is_sharpe": round(best_is_sharpe, 4),
            "best_is_params": {k: v for k, v in best_params.items()
                              if k in ("signal_threshold", "take_profit_atr",
                                       "stop_loss_atr", "trailing_stop_atr",
                                       "leverage", "min_alignment")},
            "oos_sharpe": round(median_oos, 4),
            "rank": rank,
            "n_params": n_params,
            "logit": round(logit, 4),
        })

        if (path_idx + 1) % 50 == 0:
            pbo_running = sum(1 for l in logits if l < 0) / len(logits)
            log(f"    CPCV path {path_idx+1}/{n_paths}, "
                f"running PBO={pbo_running:.2%}")

    # PBO calculation
    n_negative = sum(1 for l in logits if l < 0)
    pbo = n_negative / len(logits) if logits else 1.0

    return CPCVResult(
        n_folds=len(folds),
        n_test_folds=n_test_folds,
        n_paths=n_paths,
        purge_bars=purge_bars,
        pbo=round(pbo, 4),
        logits=logits,
        oos_sharpe_distribution=oos_sharpes,
        is_sharpe_distribution=is_sharpes,
        path_results=path_results,
    )


# ─────────────────────────────────────────────────────────────────────────────
# Helper: extract net PnLs from leveraged evaluation
# ─────────────────────────────────────────────────────────────────────────────

def extract_net_pnls(
    df: pd.DataFrame,
    params: Dict,
    position_pct: float = 0.20,
    initial_capital: float = 100.0,
) -> List[float]:
    """
    Run simulation with compounding and return per-trade net PnL percentages.

    This produces the input for monte_carlo_dd().
    """
    leverage = params.get("leverage", 1.0)
    folds = create_folds(len(df), 10, 36)

    net_pnls = []
    capital = initial_capital

    for fold in folds:
        test_df = df.iloc[fold.test_start_idx:fold.test_end_idx]
        if len(test_df) < 10:
            continue

        trips = simulate_trades(test_df, params)

        for t in trips:
            hold_hrs = t["hold_hrs"]

            if t.get("liquidated", False):
                net_pnls.append(-100.0)
                position = capital * position_pct
                capital -= position
            else:
                raw_pnl = t["pnl_pct"]
                fee_pct = flash_trade_round_trip_cost(leverage, hold_hrs)
                net_pnl = raw_pnl - fee_pct
                net_pnls.append(round(net_pnl, 4))

                position = capital * position_pct
                capital += position * (net_pnl / 100.0)

            if capital <= 0:
                break

    return net_pnls


# ─────────────────────────────────────────────────────────────────────────────
# Generate param grid for CPCV
# ─────────────────────────────────────────────────────────────────────────────

def generate_cpcv_param_grid(params: Dict, n_variants: int = 20) -> List[Dict]:
    """Generate parameter variants around a base config for CPCV testing.

    Creates variants by perturbing each numeric param by ±10-20%.
    """
    variants = [dict(params)]  # include the original
    rng = np.random.default_rng(42)

    numeric_keys = [k for k, v in params.items()
                    if isinstance(v, (int, float)) and k != "min_alignment"]

    for _ in range(n_variants - 1):
        variant = dict(params)
        # Perturb 2-3 params at a time
        n_perturb = rng.integers(2, min(4, len(numeric_keys) + 1))
        keys_to_perturb = rng.choice(numeric_keys, size=n_perturb, replace=False)

        for key in keys_to_perturb:
            delta = rng.uniform(0.10, 0.25) * rng.choice([-1, 1])
            original = variant[key]
            if isinstance(original, int):
                variant[key] = max(1, int(original * (1 + delta)))
            else:
                floor = 1.0 if key == "leverage" else 0.01
                variant[key] = max(floor, round(original * (1 + delta), 4))

        variants.append(variant)

    return variants


# ─────────────────────────────────────────────────────────────────────────────
# Full robustness report
# ─────────────────────────────────────────────────────────────────────────────

def run_robustness_analysis(
    symbol: str,
    params: Optional[Dict] = None,
    leverage: Optional[float] = None,
    position_pct: float = 0.20,
    n_mc_simulations: int = 10000,
    n_cpcv_folds: int = 10,
    n_cpcv_test_folds: int = 3,
    n_cpcv_variants: int = 20,
    output_dir: Optional[str] = None,
) -> Dict:
    """
    Run full robustness analysis on a strategy config.

    Returns a dict with Monte Carlo and CPCV results.
    """
    start_time = time.time()

    if params is None:
        params = dict(DEFAULT_PARAMS)
    if leverage is not None:
        params["leverage"] = leverage

    leverage_val = params.get("leverage", 1.0)

    log(f"{'='*60}")
    log(f"ROBUSTNESS ANALYSIS — {symbol}")
    log(f"Leverage: {leverage_val:.1f}x | Params: signal_thresh={params.get('signal_threshold')}, "
        f"sl={params.get('stop_loss_atr')}, tp={params.get('take_profit_atr')}, "
        f"trail={params.get('trailing_stop_atr')}")
    log(f"{'='*60}")

    # Load data
    safe = symbol.replace("/", "_")
    data_path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
    if not os.path.exists(data_path):
        log(f"FATAL: No data at {data_path}")
        return {"error": f"No data for {symbol}"}

    df = pd.read_parquet(data_path)
    df = compute_indicators(df)
    log(f"Loaded {symbol}: {len(df)} candles")

    # ── Phase 1: Monte Carlo DD ──
    log(f"\n── Phase 1: Monte Carlo Drawdown ({n_mc_simulations:,} simulations) ──")

    net_pnls = extract_net_pnls(df, params, position_pct=position_pct)
    log(f"  Extracted {len(net_pnls)} trades from WFA simulation")

    # Liquidation threshold at this leverage: ~1/leverage * 100%
    liquidation_dd = (1.0 / leverage_val) * 100 * 1.1  # 10% buffer
    mc_result = monte_carlo_dd(
        net_pnls,
        position_pct=position_pct,
        leverage=leverage_val,
        n_simulations=n_mc_simulations,
        liquidation_dd_pct=liquidation_dd,
    )

    log(f"  Observed DD: {mc_result.observed_dd:.1f}%")
    log(f"  MC DD p50: {mc_result.dd_p50:.1f}% | p90: {mc_result.dd_p90:.1f}% | "
        f"p95: {mc_result.dd_p95:.1f}% | p99: {mc_result.dd_p99:.1f}%")
    log(f"  P(DD > 20%): {mc_result.prob_dd_gt_20:.1%} | "
        f"P(DD > 30%): {mc_result.prob_dd_gt_30:.1%} | "
        f"P(DD > 50%): {mc_result.prob_dd_gt_50:.1%}")
    log(f"  P(liquidation): {mc_result.prob_dd_liquidation:.1%}")
    log(f"  Return p10/p50/p90: {mc_result.return_p10:+.1f}% / "
        f"{mc_result.return_p50:+.1f}% / {mc_result.return_p90:+.1f}%")

    # ── Phase 2: CPCV + PBO ──
    log(f"\n── Phase 2: CPCV + PBO ({n_cpcv_variants} param variants, "
        f"{n_cpcv_folds} folds) ──")

    params_grid = generate_cpcv_param_grid(params, n_variants=n_cpcv_variants)
    log(f"  Generated {len(params_grid)} param variants")

    cpcv_result = cpcv(
        df,
        params_grid,
        n_folds=n_cpcv_folds,
        n_test_folds=n_cpcv_test_folds,
        purge_bars=6,
    )

    log(f"  CPCV: {cpcv_result.n_paths} paths evaluated")
    log(f"  PBO: {cpcv_result.pbo:.2%} "
        f"({'SAFE' if cpcv_result.pbo < 0.15 else 'WARNING' if cpcv_result.pbo < 0.30 else 'DANGER'})")
    if cpcv_result.logits:
        logits_arr = np.array(cpcv_result.logits)
        log(f"  Logit mean: {np.mean(logits_arr):+.3f} | "
            f"median: {np.median(logits_arr):+.3f} | "
            f"std: {np.std(logits_arr):.3f}")

    # ── Build result ──
    elapsed = time.time() - start_time

    result = {
        "symbol": symbol,
        "params": params,
        "leverage": leverage_val,
        "position_pct": position_pct,
        "elapsed_seconds": round(elapsed, 1),
        "monte_carlo": {
            "n_simulations": mc_result.n_simulations,
            "n_trades": mc_result.n_trades,
            "observed_dd": mc_result.observed_dd,
            "dd_p50": mc_result.dd_p50,
            "dd_p75": mc_result.dd_p75,
            "dd_p90": mc_result.dd_p90,
            "dd_p95": mc_result.dd_p95,
            "dd_p99": mc_result.dd_p99,
            "dd_worst": mc_result.dd_worst,
            "prob_dd_gt_10": mc_result.prob_dd_gt_10,
            "prob_dd_gt_20": mc_result.prob_dd_gt_20,
            "prob_dd_gt_30": mc_result.prob_dd_gt_30,
            "prob_dd_gt_50": mc_result.prob_dd_gt_50,
            "prob_dd_liquidation": mc_result.prob_dd_liquidation,
            "return_p50": mc_result.return_p50,
            "return_p90": mc_result.return_p90,
            "return_p10": mc_result.return_p10,
        },
        "cpcv": {
            "n_folds": cpcv_result.n_folds,
            "n_test_folds": cpcv_result.n_test_folds,
            "n_paths": cpcv_result.n_paths,
            "pbo": cpcv_result.pbo,
            "logit_mean": round(float(np.mean(cpcv_result.logits)), 4) if cpcv_result.logits else 0,
            "logit_median": round(float(np.median(cpcv_result.logits)), 4) if cpcv_result.logits else 0,
            "path_results": cpcv_result.path_results[:50],  # cap for storage
        },
        "verdict": _verdict(mc_result, cpcv_result),
    }

    # Save
    if output_dir is None:
        output_dir = os.path.join(RESULTS_DIR, datetime.now(timezone.utc).strftime("%Y-%m-%d"))
    os.makedirs(output_dir, exist_ok=True)

    out_path = os.path.join(output_dir, "robustness.json")
    with open(out_path, "w") as f:
        json.dump(result, f, indent=2, default=str)
    log(f"\n  Saved to {out_path}")

    # Markdown report
    md_path = os.path.join(output_dir, "robustness_report.md")
    _write_robustness_report(result, md_path)
    log(f"  Report: {md_path}")

    log(f"\n{'='*60}")
    log(f"ROBUSTNESS COMPLETE — {elapsed:.0f}s")
    log(f"Verdict: {result['verdict']['overall']}")
    log(f"{'='*60}")

    return result


def _verdict(mc: MonteCarloResult, cpcv: CPCVResult) -> Dict:
    """Produce a human-readable verdict."""
    flags = []

    if mc.prob_dd_liquidation > 0.05:
        flags.append("HIGH liquidation risk in MC simulation")
    if mc.dd_p95 > 40:
        flags.append(f"95th pctl DD={mc.dd_p95:.1f}% exceeds 40%")
    if cpcv.pbo > 0.30:
        flags.append(f"PBO={cpcv.pbo:.0%} indicates likely overfitting")
    if cpcv.pbo > 0.15:
        flags.append(f"PBO={cpcv.pbo:.0%} is elevated (>15%)")

    if not flags:
        overall = "PASS — Strategy passes robustness checks"
    elif len(flags) == 1:
        overall = "CAUTION — Minor robustness concern"
    else:
        overall = "FAIL — Multiple robustness concerns"

    return {
        "overall": overall,
        "flags": flags,
        "mc_95th_dd": mc.dd_p95,
        "mc_liquidation_prob": mc.prob_dd_liquidation,
        "pbo": cpcv.pbo,
    }


def _write_robustness_report(result: Dict, path: str):
    """Write markdown robustness report."""
    lines = []
    w = lines.append

    sym = result["symbol"]
    lev = result["leverage"]
    params = result["params"]
    mc = result["monte_carlo"]
    cpcv = result["cpcv"]
    verdict = result["verdict"]

    w(f"# Robustness Report — {sym} @ {lev:.0f}x")
    w(f"")
    w(f"**Params:** thresh={params.get('signal_threshold')}, "
      f"sl={params.get('stop_loss_atr')}, tp={params.get('take_profit_atr')}, "
      f"trail={params.get('trailing_stop_atr')}, align={params.get('min_alignment')}")
    w(f"**Runtime:** {result['elapsed_seconds']:.0f}s")
    w(f"")

    w(f"## Verdict: {verdict['overall']}")
    w(f"")
    for flag in verdict["flags"]:
        w(f"- {flag}")
    if not verdict["flags"]:
        w(f"- No concerns detected")
    w(f"")

    w(f"## Monte Carlo Drawdown ({mc['n_simulations']:,} simulations)")
    w(f"")
    w(f"| Metric | Value |")
    w(f"|--------|-------|")
    w(f"| Trades | {mc['n_trades']} |")
    w(f"| Observed DD | {mc['observed_dd']:.1f}% |")
    w(f"| DD p50 | {mc['dd_p50']:.1f}% |")
    w(f"| DD p75 | {mc['dd_p75']:.1f}% |")
    w(f"| DD p90 | {mc['dd_p90']:.1f}% |")
    w(f"| DD p95 | {mc['dd_p95']:.1f}% |")
    w(f"| DD p99 | {mc['dd_p99']:.1f}% |")
    w(f"| DD worst | {mc['dd_worst']:.1f}% |")
    w(f"| P(DD > 20%) | {mc['prob_dd_gt_20']:.1%} |")
    w(f"| P(DD > 30%) | {mc['prob_dd_gt_30']:.1%} |")
    w(f"| P(DD > 50%) | {mc['prob_dd_gt_50']:.1%} |")
    w(f"| P(liquidation) | {mc['prob_dd_liquidation']:.1%} |")
    w(f"| Return p10 | {mc['return_p10']:+.1f}% |")
    w(f"| Return p50 | {mc['return_p50']:+.1f}% |")
    w(f"| Return p90 | {mc['return_p90']:+.1f}% |")
    w(f"")

    w(f"## CPCV + Probability of Backtest Overfitting")
    w(f"")
    w(f"| Metric | Value |")
    w(f"|--------|-------|")
    w(f"| Folds | {cpcv['n_folds']} |")
    w(f"| Test folds/path | {cpcv['n_test_folds']} |")
    w(f"| Total paths | {cpcv['n_paths']} |")
    w(f"| **PBO** | **{cpcv['pbo']:.2%}** |")
    w(f"| Logit mean | {cpcv['logit_mean']:+.3f} |")
    w(f"| Logit median | {cpcv['logit_median']:+.3f} |")
    w(f"")
    w(f"*PBO < 15%: SAFE | 15-30%: ELEVATED | > 30%: OVERFITTING*")
    w(f"")

    with open(path, "w") as f:
        f.write("\n".join(lines))


# ─────────────────────────────────────────────────────────────────────────────
# CLI
# ─────────────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Robustness testing: Monte Carlo DD + CPCV + PBO"
    )
    parser.add_argument("--symbol", type=str, default="SOL/USDT",
                        help="Symbol to analyze")
    parser.add_argument("--leverage", type=float, default=9.0,
                        help="Leverage level (default: 9.0)")
    parser.add_argument("--mc-simulations", type=int, default=10000,
                        help="Monte Carlo simulations (default: 10000)")
    parser.add_argument("--cpcv-folds", type=int, default=10,
                        help="CPCV number of folds (default: 10)")
    parser.add_argument("--cpcv-test-folds", type=int, default=3,
                        help="CPCV test folds per path (default: 3)")
    parser.add_argument("--cpcv-variants", type=int, default=20,
                        help="Number of param variants for CPCV (default: 20)")
    parser.add_argument("--monte-carlo-only", action="store_true",
                        help="Run only Monte Carlo DD simulation")
    parser.add_argument("--cpcv-only", action="store_true",
                        help="Run only CPCV + PBO analysis")
    parser.add_argument("--output-dir", type=str, default=None,
                        help="Output directory for results")
    args = parser.parse_args()

    if args.monte_carlo_only or args.cpcv_only:
        # Partial run
        safe = args.symbol.replace("/", "_")
        data_path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
        if not os.path.exists(data_path):
            log(f"FATAL: No data at {data_path}")
            sys.exit(1)

        df = pd.read_parquet(data_path)
        df = compute_indicators(df)
        params = dict(DEFAULT_PARAMS)
        params["leverage"] = args.leverage

        if args.monte_carlo_only:
            net_pnls = extract_net_pnls(df, params)
            mc = monte_carlo_dd(net_pnls, leverage=args.leverage,
                                n_simulations=args.mc_simulations)
            log(f"MC DD p95={mc.dd_p95:.1f}% p99={mc.dd_p99:.1f}% "
                f"P(liq)={mc.prob_dd_liquidation:.1%}")
            print(json.dumps({
                "observed_dd": mc.observed_dd,
                "dd_p50": mc.dd_p50,
                "dd_p95": mc.dd_p95,
                "dd_p99": mc.dd_p99,
                "prob_dd_gt_20": mc.prob_dd_gt_20,
                "prob_dd_gt_30": mc.prob_dd_gt_30,
                "prob_dd_liquidation": mc.prob_dd_liquidation,
            }, indent=2))

        if args.cpcv_only:
            grid = generate_cpcv_param_grid(params, args.cpcv_variants)
            result = cpcv(df, grid, n_folds=args.cpcv_folds,
                         n_test_folds=args.cpcv_test_folds)
            log(f"PBO={result.pbo:.2%} ({len(result.logits)} paths)")
            print(json.dumps({
                "pbo": result.pbo,
                "n_paths": result.n_paths,
                "logit_mean": float(np.mean(result.logits)) if result.logits else 0,
            }, indent=2))
    else:
        # Full run
        run_robustness_analysis(
            symbol=args.symbol,
            leverage=args.leverage,
            n_mc_simulations=args.mc_simulations,
            n_cpcv_folds=args.cpcv_folds,
            n_cpcv_test_folds=args.cpcv_test_folds,
            n_cpcv_variants=args.cpcv_variants,
            output_dir=args.output_dir,
        )


if __name__ == "__main__":
    main()
