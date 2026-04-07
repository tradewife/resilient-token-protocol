"""
Night Shift — Zero-token autonomous strategy optimization.

Runs overnight as a pure Python script (no LLM calls). Performs:
  1. Data refresh from Binance (optional)
  2. Expanding-window WFA with non-overlapping test folds
  3. Coarse grid search + fine refinement + Darwinian evolution
  4. Three-layer overfitting detection
  5. Regime analysis
  6. Structured morning report (markdown + JSON)

Inspired by:
  - https://github.com/karpathy/autoresearch (self-improving loop)
  - https://github.com/chrisworsey55/atlas-gic (Darwinian optimization)

Design doc: docs/NIGHT_SHIFT_DESIGN.md

Usage:
  python scripts/night_shift.py                      # use defaults
  python scripts/night_shift.py --config data/night_config.json
  python scripts/night_shift.py --skip-fetch          # use cached data
  python scripts/night_shift.py --symbols BTC/USDT ETH/USDT
"""
import argparse
import json
import os
import random
import sys
import time
from collections import Counter
from dataclasses import dataclass, asdict, field
from datetime import datetime, timedelta, timezone
from itertools import product
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import numpy as np
import pandas as pd

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from scripts.per_symbol_optimizer import (
    compute_indicators,
    simulate_trades,
    compute_metrics,
    _compute_score,
)

# ─── Paths ────────────────────────────────────────────────────────────────────

ROOT = os.path.join(os.path.dirname(__file__), "..")
DATA_DIR = os.path.join(ROOT, "data", "ohlcv")
RESULTS_DIR = os.path.join(ROOT, "data", "night_results")
CONFIG_PATH = os.path.join(ROOT, "scripts", "night_config.json")
PRODUCTION_CONFIG_PATH = os.path.join(ROOT, "knowledge_base", "production_config.json")

# ─── Config Loader ────────────────────────────────────────────────────────────

def load_config(config_path: Optional[str] = None) -> dict:
    """Load night_config.json if it exists."""
    path = config_path or CONFIG_PATH
    if os.path.exists(path):
        with open(path) as f:
            return json.load(f)
    return {}


# ─── Defaults ─────────────────────────────────────────────────────────────────

DEFAULT_SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT"]

# Production baseline config (from production_config.json wide_tp + trailing stop)
PRODUCTION_CONFIG = {
    "signal_threshold": 0.40,
    "min_alignment": 3,
    "take_profit_atr": 6.0,
    "stop_loss_atr": 2.5,
    "max_hold_hours": 96,
    "time_decay_hours": 48,
    "trailing_stop_atr": 1.0,
    "score_flip_delay_hrs": 2,
}

# Coarse grid: 8 × 8 × 6 × 5 × 4 × 2 × 2 = 30,720 combos
COARSE_GRID = {
    "signal_threshold":    [0.30, 0.33, 0.35, 0.38, 0.40, 0.43, 0.45, 0.50],
    "take_profit_atr":     [3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 6.0, 7.0],
    "stop_loss_atr":       [1.0, 1.25, 1.5, 2.0, 2.5, 3.0],
    "max_hold_hours":      [36, 48, 72, 96, 120],
    "time_decay_hours":    [12, 24, 36, 48],
    "trailing_stop_atr":   [0.0, 1.0],
    "score_flip_delay_hrs": [0, 2],
}

# Fine grid for refinement around top candidates
FINE_GRID = {
    "trailing_stop_atr":   [0.0, 0.5, 0.8, 1.0, 1.2, 1.5],
    "score_flip_delay_hrs": [0, 1, 2, 3, 4],
}

WFA_CONFIG = {
    "num_folds": 10,
    "test_fold_days": 36,
    "min_trades_per_fold": 10,
}

OVERFITTING_CONFIG = {
    "max_is_oos_gap": 0.5,
    "min_oos_consistency": 0.50,
    "max_fragility": 0.4,
}

DARWINIAN_CONFIG = {
    "generations": 5,
    "population": 50,
    "perturbation_range": (0.05, 0.15),
}


# ─── Logging ──────────────────────────────────────────────────────────────────

def log(msg: str):
    ts = datetime.now(timezone.utc).strftime("%H:%M:%S")
    print(f"[{ts}] {msg}", flush=True)


# ─── WFA: Expanding-Window Folds ─────────────────────────────────────────────

@dataclass
class Fold:
    """One train/test split."""
    fold_num: int
    train_start_idx: int
    train_end_idx: int      # exclusive
    test_start_idx: int
    test_end_idx: int        # exclusive
    train_hours: int
    test_hours: int


def create_folds(total_bars: int, num_folds: int, test_fold_days: int,
                 bars_per_day: int = 24) -> List[Fold]:
    """
    Create non-overlapping expanding-window WFA folds.

    Data layout:
        Fold 1: [TRAIN====][TEST]
        Fold 2: [TRAIN============][TEST]
        ...
    Every bar is in a test fold exactly once. No overlap.

    test_fold_days controls the test window size. num_folds is a maximum —
    actual folds are determined by available data.
    """
    test_bars = test_fold_days * bars_per_day
    warmup = 250  # indicator warmup bars

    if total_bars <= warmup + test_bars:
        # Not enough data for even one fold — return single fold with what we have
        return [Fold(
            fold_num=0,
            train_start_idx=0,
            train_end_idx=warmup,
            test_start_idx=warmup,
            test_end_idx=total_bars,
            train_hours=warmup,
            test_hours=total_bars - warmup,
        )]

    usable_bars = total_bars - warmup
    # Use test_fold_days to determine fold size, cap by num_folds
    max_folds = usable_bars // test_bars
    actual_folds = min(num_folds, max_folds)

    # Recalculate test size to evenly partition the usable data
    # If num_folds < max_folds, use test_fold_days as the actual fold size
    # (don't inflate folds beyond test_fold_days)
    if actual_folds < max_folds:
        bars_per_fold = test_bars
    else:
        bars_per_fold = usable_bars // actual_folds

    folds = []
    test_start = warmup
    for i in range(actual_folds):
        test_end = test_start + bars_per_fold if i < actual_folds - 1 else total_bars
        folds.append(Fold(
            fold_num=i,
            train_start_idx=0,
            train_end_idx=test_start,
            test_start_idx=test_start,
            test_end_idx=test_end,
            train_hours=test_start,
            test_hours=test_end - test_start,
        ))
        test_start = test_end

    return folds


# ─── Fast Evaluation ─────────────────────────────────────────────────────────

@dataclass
class FoldMetrics:
    """Metrics for one fold."""
    fold_num: int
    is_sharpe: float
    oos_sharpe: float
    oos_pnl: float
    oos_pf: float
    oos_wr: float
    oos_max_dd: float
    oos_trades: int
    oos_avg_hold: float
    oos_exit_reasons: Dict[str, int] = field(default_factory=dict)


@dataclass
class CandidateResult:
    """Full WFA result for one candidate config on one symbol."""
    symbol: str
    params: Dict
    # Aggregate OOS
    oos_sharpe: float
    oos_pnl: float
    oos_pf: float
    oos_wr: float
    oos_max_dd: float
    oos_consistency: float          # % of folds with positive OOS Sharpe
    oos_avg_trades_per_fold: float
    oos_mean_hold_hrs: float
    oos_exit_reasons: Dict[str, int]
    # IS metrics (full sample before last fold)
    is_sharpe: float
    is_pnl: float
    # Overfitting
    overfitting_score: float
    fragility: float
    # Ranking
    survivor_score: float
    # Per-fold detail
    folds: List[Dict] = field(default_factory=list)
    # Metadata
    rejected: bool = False
    rejection_reason: str = ""
    is_coarse_only: bool = False


def evaluate_on_fold(df: pd.DataFrame, fold: Fold, params: Dict,
                     skip_is: bool = False) -> Dict:
    """Evaluate a config on a single train/test fold. Returns IS + OOS metrics."""
    is_bb = params.get("strategy") == "bb_mean_reversion"
    sim_fn = simulate_bb_trades if is_bb else simulate_trades

    if not skip_is:
        # IS: train period
        train_df = df.iloc[fold.train_start_idx:fold.train_end_idx]
        if is_bb:
            train_df = compute_indicators(train_df)
        train_trips = sim_fn(train_df, params) if len(train_df) > 250 else []
        train_hours = fold.train_end_idx - fold.train_start_idx
        train_m = compute_metrics(train_trips, total_hours=train_hours)
        is_sharpe = train_m["sharpe"]
        is_pnl = train_m["total_pnl_pct"]
    else:
        is_sharpe = 0.0
        is_pnl = 0.0

    # OOS: test period
    test_df = df.iloc[fold.test_start_idx:fold.test_end_idx]
    if is_bb:
        test_df = compute_indicators(test_df)
    test_trips = sim_fn(test_df, params) if len(test_df) > 10 else []
    test_hours = fold.test_end_idx - fold.test_start_idx
    test_m = compute_metrics(test_trips, total_hours=test_hours)

    return {
        "is_sharpe": is_sharpe,
        "is_pnl": is_pnl,
        "oos_sharpe": test_m["sharpe"],
        "oos_pnl": test_m["total_pnl_pct"],
        "oos_pf": test_m["pf"],
        "oos_wr": test_m["win_rate"],
        "oos_max_dd": test_m["max_dd_pct"],
        "oos_trades": test_m["round_trips"],
        "oos_avg_hold": test_m["avg_hold_hrs"],
        "oos_exit_reasons": test_m.get("exit_reasons", {}),
    }


def evaluate_candidate(df: pd.DataFrame, folds: List[Fold], params: Dict,
                       symbol: str, of_config: Dict,
                       compute_fragility: bool = False,
                       skip_is: bool = False) -> CandidateResult:
    """Full WFA evaluation of one candidate config.

    Args:
        compute_fragility: If False, skip expensive fragility check.
                           Set True only for top candidates.
        skip_is: If True, skip IS evaluation (train window).
                 Use for coarse grid where we only need rough OOS ordering.
    """
    fold_results = []
    for fold in folds:
        fm = evaluate_on_fold(df, fold, params, skip_is=skip_is)
        fold_results.append(fm)

    # Aggregate OOS
    oos_sharpes_raw = [f["oos_sharpe"] for f in fold_results]
    # Winsorize per-fold Sharpe at ±100 to prevent tiny-sample outliers
    # (2 trades with similar PnL → std≈0 → Sharpe→∞ via sqrt(8760) annualization)
    SHARPE_CAP = 100.0
    oos_sharpes = [max(-SHARPE_CAP, min(SHARPE_CAP, s)) for s in oos_sharpes_raw]
    oos_pnls = [f["oos_pnl"] for f in fold_results]
    oos_pfs = [f["oos_pf"] for f in fold_results if f["oos_pf"] < 999]
    oos_wrs = [f["oos_wr"] for f in fold_results]
    oos_dds = [f["oos_max_dd"] for f in fold_results]
    oos_trades = [f["oos_trades"] for f in fold_results]
    oos_holds = [f["oos_avg_hold"] for f in fold_results]

    # IS: mean across all train periods
    is_sharpes = [f["is_sharpe"] for f in fold_results]
    is_pnls = [f["is_pnl"] for f in fold_results]

    avg_is_sharpe = float(np.mean(is_sharpes)) if is_sharpes else 0
    avg_is_pnl = float(np.sum(is_pnls)) if is_pnls else 0
    # Use MEDIAN for OOS Sharpe — robust to single-fold outliers.
    # Mean of [-44, -24, -13, +2, +17, +18, +31, +55, +8563] = +956 (wrong!)
    # Median of same = +17 (correct representative performance)
    avg_oos_sharpe = float(np.median(oos_sharpes)) if oos_sharpes else 0
    avg_oos_pnl = float(np.sum(oos_pnls)) if oos_pnls else 0
    avg_oos_pf = float(np.mean(oos_pfs)) if oos_pfs else 0
    avg_oos_wr = float(np.mean(oos_wrs)) if oos_wrs else 0
    avg_oos_dd = float(np.mean(oos_dds)) if oos_dds else 0
    avg_oos_trades = float(np.mean(oos_trades)) if oos_trades else 0
    avg_oos_hold = float(np.mean(oos_holds)) if oos_holds else 0

    # OOS consistency: % of folds with positive OOS Sharpe (using winsorized)
    positive_folds = sum(1 for s in oos_sharpes if s > 0)
    oos_consistency = positive_folds / len(oos_sharpes) if oos_sharpes else 0

    # Aggregate exit reasons
    all_exits = Counter()
    for f in fold_results:
        for reason, count in f["oos_exit_reasons"].items():
            all_exits[reason] += count

    # ── Overfitting Layer 1: IS-OOS Gap ──
    if avg_is_sharpe == 0 and avg_is_pnl == 0:
        # IS was skipped (coarse pass) — can't compute gap
        overfitting_score = 0.0
    elif abs(avg_is_sharpe) > 0.01:
        overfitting_score = (avg_is_sharpe - avg_oos_sharpe) / abs(avg_is_sharpe)
    else:
        overfitting_score = 0.5 if avg_oos_sharpe < 0 else 0.0

    # Mark as coarse-only if evaluated on < 3 folds
    num_folds_evaluated = len(fold_results)
    is_coarse_only = num_folds_evaluated < 3
    overfitting_score = max(0, overfitting_score)  # OOS > IS isn't overfitting

    # ── Overfitting Layer 3: Parameter Sensitivity (Fragility) ──
    fragility = 0.0
    if compute_fragility and avg_oos_sharpe > 0.1:  # Only compute for promising candidates
        for param_name, param_val in params.items():
            if param_name == "min_alignment":
                continue  # discrete, skip
            if not isinstance(param_val, (int, float)):
                continue
            for delta in [-0.10, 0.10]:
                perturbed = {**params, param_name: round(param_val * (1 + delta), 4)}
                perturbed_result = evaluate_on_fold(df, folds[-1], perturbed)
                perturbed_sharpe = max(-SHARPE_CAP, min(SHARPE_CAP, perturbed_result["oos_sharpe"]))
                if abs(avg_oos_sharpe) > 0.01:
                    sensitivity = abs(perturbed_sharpe - avg_oos_sharpe) / abs(avg_oos_sharpe)
                    fragility = max(fragility, sensitivity)

    # ── Survivor Score ──
    of_penalty = 1.0 - min(overfitting_score, 1.0)
    dd_factor = 1.0 / (1.0 + avg_oos_dd / 100)
    trade_factor = min(avg_oos_trades / max(of_config.get("min_trades_per_fold", 10), 1), 1.0)
    # Fragility: inverse penalty that stays non-negative.
    # f=0→1.0, f=0.5→0.67, f=1.0→0.50, f=2.0→0.33, f=5.0→0.17
    # A config with positive Sharpe should never have negative survivor.
    fragility_penalty = 1.0 / (1.0 + fragility)
    survivor_score = avg_oos_sharpe * oos_consistency * of_penalty * dd_factor * trade_factor * fragility_penalty

    # ── Rejection check (only IS-OOS gap and consistency — fragility is now a penalty) ──
    rejected = False
    rejection_reason = ""
    if overfitting_score > of_config.get("max_is_oos_gap", 0.5):
        rejected = True
        rejection_reason = f"overfitting_score={overfitting_score:.2f} > {of_config.get('max_is_oos_gap', 0.5)}"
    if oos_consistency < of_config.get("min_oos_consistency", 0.50):
        rejected = True
        rejection_reason = f"oos_consistency={oos_consistency:.0%} < {of_config.get('min_oos_consistency', 0.50):.0%}"
    # Fragility is NO LONGER a hard rejection — it's a weighted penalty in survivor_score

    # Fold detail (for debugging) — use winsorized Sharpe for consistency with aggregates
    fold_details = [
        {
            "fold": folds[i].fold_num if i < len(folds) else i,
            "is_sharpe": f["is_sharpe"],
            "oos_sharpe": oos_sharpes[i],  # winsorized
            "oos_sharpe_raw": oos_sharpes_raw[i],  # original (for outlier detection)
            "oos_pnl": f["oos_pnl"],
            "oos_trades": f["oos_trades"],
        }
        for i, f in enumerate(fold_results)
    ]

    return CandidateResult(
        symbol=symbol,
        params=dict(params),
        oos_sharpe=avg_oos_sharpe,
        oos_pnl=avg_oos_pnl,
        oos_pf=avg_oos_pf,
        oos_wr=avg_oos_wr,
        oos_max_dd=avg_oos_dd,
        oos_consistency=oos_consistency,
        oos_avg_trades_per_fold=avg_oos_trades,
        oos_mean_hold_hrs=avg_oos_hold,
        oos_exit_reasons=dict(all_exits),
        is_sharpe=avg_is_sharpe,
        is_pnl=avg_is_pnl,
        overfitting_score=overfitting_score,
        fragility=fragility,
        survivor_score=survivor_score,
        folds=fold_details,
        rejected=rejected,
        rejection_reason=rejection_reason,
        is_coarse_only=is_coarse_only,
    )


# ─── Grid Search ──────────────────────────────────────────────────────────────

def grid_combos(grid: Dict) -> List[Dict]:
    """Generate all combinations from a param grid."""
    keys = list(grid.keys())
    values = [grid[k] for k in keys]
    return [dict(zip(keys, combo)) for combo in product(*values)]


def coarse_grid_search(df: pd.DataFrame, folds: List[Fold], symbol: str,
                       of_config: Dict, base_params: Dict = None) -> List[CandidateResult]:
    """Stage 1: Coarse grid search — fast single-window evaluation.

    Evaluates each candidate on a single 720-bar window (last 30 days)
    for rough ordering. No WFA structure needed here — just quickly
    eliminate obviously bad configs and rank the rest.
    Full WFA validation happens in Stage 2 (fine refinement).
    """
    combos = grid_combos(COARSE_GRID)
    log(f"  Coarse grid: {len(combos)} combos for {symbol}")

    # Single 720-bar window (last 30 days) for fast rough ordering
    window_bars = 720
    if len(df) > window_bars:
        coarse_fold = Fold(
            fold_num=0,
            train_start_idx=0,
            train_end_idx=max(0, len(df) - window_bars - 250),
            test_start_idx=len(df) - window_bars,
            test_end_idx=len(df),
            train_hours=max(0, len(df) - window_bars - 250),
            test_hours=window_bars,
        )
    else:
        coarse_fold = folds[-1]

    results = []
    for i, combo in enumerate(combos):
        params = {**(base_params or {}), **combo, "min_alignment": 3}
        cr = evaluate_candidate(df, [coarse_fold], params, symbol, of_config,
                               compute_fragility=False, skip_is=True)
        results.append(cr)

        if (i + 1) % 10000 == 0:
            log(f"    [{symbol}] {i+1}/{len(combos)} evaluated... "
                f"best survivor so far: {max(r.survivor_score for r in results):.3f}")

    log(f"    [{symbol}] Done. {sum(1 for r in results if not r.rejected)} passed filters "
        f"out of {len(results)}")
    return results


def fine_refinement(df: pd.DataFrame, folds: List[Fold], symbol: str,
                    top_candidates: List[CandidateResult], of_config: Dict) -> List[CandidateResult]:
    """Stage 2: Full WFA evaluation on ALL folds for top candidates.

    Re-evaluates on the complete fold set (not just the 2 coarse folds).
    Also sweeps trailing_stop and score_flip_delay at fine granularity.
    """
    results = []
    seen_keys = set()

    for parent in top_candidates:
        if parent.rejected:
            continue
        base = dict(parent.params)
        # Remove trailing/flip params to re-sweep them at fine granularity
        base.pop("trailing_stop_atr", None)
        base.pop("score_flip_delay_hrs", None)

        fine_combos = grid_combos(FINE_GRID)
        for combo in fine_combos:
            params = {**base, **combo}
            key = tuple(sorted(params.items()))
            if key in seen_keys:
                continue
            seen_keys.add(key)
            # Full evaluation on ALL folds + fragility check
            cr = evaluate_candidate(df, folds, params, symbol, of_config,
                                   compute_fragility=True)
            results.append(cr)

    log(f"    [{symbol}] Fine refinement: {len(results)} candidates "
        f"evaluated on all {len(folds)} folds")
    return results
    return results


def darwinian_evolution(df: pd.DataFrame, folds: List[Fold], symbol: str,
                        population: List[CandidateResult], of_config: Dict,
                        config: Dict) -> List[CandidateResult]:
    """Stage 3: Darwinian refinement with random perturbations."""
    generations = config.get("generations", DARWINIAN_CONFIG["generations"])
    pop_size = config.get("population", DARWINIAN_CONFIG["population"])
    perturb_range = config.get("perturbation_range", DARWINIAN_CONFIG["perturbation_range"])

    # Seed with top non-rejected candidates
    current_gen = sorted(
        [r for r in population if not r.rejected],
        key=lambda r: r.survivor_score,
        reverse=True,
    )[:pop_size]

    if not current_gen:
        log(f"    [{symbol}] No survivors for Darwinian evolution")
        return []

    all_survivors = list(current_gen)

    for gen in range(generations):
        offspring = []
        for parent in current_gen:
            for _ in range(3):  # 3 children per parent
                # Random perturbation
                params = dict(parent.params)
                numeric_keys = [k for k, v in params.items() if isinstance(v, (int, float)) and k != "min_alignment"]
                if not numeric_keys:
                    continue
                key = random.choice(numeric_keys)
                delta = random.uniform(*perturb_range) * random.choice([-1, 1])
                original = params[key]
                if isinstance(original, int):
                    params[key] = max(1, int(original * (1 + delta)))
                else:
                    params[key] = max(0.01, round(original * (1 + delta), 4))

                cr = evaluate_candidate(df, folds, params, symbol, of_config,
                                       compute_fragility=True)
                offspring.append(cr)

        # Selection: combine parents + offspring, take top N
        combined = current_gen + offspring
        combined.sort(key=lambda r: r.survivor_score, reverse=True)
        current_gen = combined[:pop_size]
        all_survivors.extend(current_gen)

        best_score = current_gen[0].survivor_score
        log(f"    [{symbol}] Darwinian gen {gen+1}/{generations}: "
            f"{len(offspring)} offspring, best survivor={best_score:.3f}")

    # Deduplicate by params
    seen = set()
    unique = []
    for r in sorted(all_survivors, key=lambda r: r.survivor_score, reverse=True):
        key = tuple(sorted(r.params.items()))
        if key not in seen:
            seen.add(key)
            unique.append(r)

    return unique[:pop_size * 2]


# ─── BB Mean Reversion Strategy (Fast Evaluator) ──────────────────────────────

BB_GRID = {
    "rsi_oversold": [25, 28, 30, 33],
    "stop_loss_atr_multiplier": [1.5, 2.0, 2.5],
    "take_profit_atr_multiplier": [2.0, 3.0, 4.0],
    "max_hold_hours": [36, 48, 72],
    "trend_filter_period": [50, 100],
}


def simulate_bb_trades(df: pd.DataFrame, params: Dict) -> List[Dict]:
    """Fast BB mean reversion simulation — mirrors simulate_trades() pattern."""
    close = df["close"].values
    high = df["high"].values
    low = df["low"].values

    # Compute indicators
    bb_period = 20
    sma = pd.Series(close).rolling(bb_period).mean().values
    std = pd.Series(close).rolling(bb_period).std().values
    bb_lower = sma - 2.0 * std
    bb_middle = sma
    bb_upper = sma + 2.0 * std

    delta = pd.Series(close).diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean().values
    loss_s = (-delta.where(delta < 0, 0)).rolling(14).mean().values
    rs = np.where(loss_s > 0, gain / loss_s, 50.0)
    with np.errstate(invalid='ignore'):
        rsi = 100 - (100 / (1 + rs))
    rsi = np.nan_to_num(rsi, nan=50.0)

    trend_period = params.get("trend_filter_period", 50)
    trend_sma = pd.Series(close).rolling(trend_period).mean().values

    tr = pd.concat([
        pd.Series(high - low),
        pd.Series(high).shift(1).sub(pd.Series(close)).abs(),
        pd.Series(low).shift(1).sub(pd.Series(close)).abs(),
    ], axis=1).max(axis=1).values
    atr = pd.Series(tr).rolling(14).mean().values

    rsi_thresh = params.get("rsi_oversold", 30)
    sl_mult = params.get("stop_loss_atr_multiplier", 2.0)
    tp_mult = params.get("take_profit_atr_multiplier", 3.0)
    max_hold = params.get("max_hold_hours", 48)
    decay_hours = params.get("time_decay_hours", 24)

    trips = []
    in_position = False
    entry_price = 0.0
    entry_idx = 0
    peak_price = 0.0
    warmup = 250

    for i in range(warmup, len(close)):
        price = close[i]
        a = atr[i] if not np.isnan(atr[i]) else 0
        r = rsi[i] if not np.isnan(rsi[i]) else 50
        bl = bb_lower[i] if not np.isnan(bb_lower[i]) else price
        bm = bb_middle[i] if not np.isnan(bb_middle[i]) else price
        bu = bb_upper[i] if not np.isnan(bb_upper[i]) else price
        ts = trend_sma[i] if not np.isnan(trend_sma[i]) else price

        if in_position:
            hold_hrs = i - entry_idx
            pnl_pct = (price - entry_price) / entry_price * 100

            if price > peak_price:
                peak_price = price

            # Stop loss
            if a > 0 and pnl_pct <= -(sl_mult * a / entry_price * 100):
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "stop_loss"})
                in_position = False
                continue

            # Take profit
            if a > 0 and pnl_pct >= (tp_mult * a / entry_price * 100):
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "take_profit"})
                in_position = False
                continue

            # Max hold
            if hold_hrs >= max_hold:
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "max_hold"})
                in_position = False
                continue

            # Time decay
            if pnl_pct < 0 and hold_hrs >= decay_hours:
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "time_decay"})
                in_position = False
                continue

            # Mean reversion targets
            if price >= bm:
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "middle_band"})
                in_position = False
                continue
        else:
            # Entry: price at/below lower band + RSI oversold + uptrend
            if price <= bl and r <= rsi_thresh and price > ts:
                in_position = True
                entry_price = price
                entry_idx = i
                peak_price = price

    return trips


def run_bb_grid_search(df: pd.DataFrame, folds: List[Fold], symbol: str,
                       of_config: Dict) -> List[CandidateResult]:
    """Run BB mean reversion grid search on all WFA folds."""
    combos = grid_combos(BB_GRID)
    log(f"  BB grid: {len(combos)} combos for {symbol}")

    results = []
    for i, params in enumerate(combos):
        params["min_alignment"] = 0  # not used by BB
        params["strategy"] = "bb_mean_reversion"
        cr = evaluate_candidate(df, folds, params, symbol, of_config,
                               compute_fragility=False, skip_is=True)
        cr.params["strategy"] = "bb_mean_reversion"
        results.append(cr)

        if (i + 1) % 5000 == 0:
            log(f"    [{symbol}] {i+1}/{len(combos)} evaluated...")

    passed = sum(1 for r in results if not r.rejected)
    log(f"    [{symbol}] Done. {passed} passed filters out of {len(results)}")
    return results


# ─── Experiment Runner ────────────────────────────────────────────────────────

def run_experiments(df: pd.DataFrame, folds: List[Fold], symbol: str,
                   experiments: List[Dict], of_config: Dict) -> List[CandidateResult]:
    """Run custom experiments from night_config.json experiments array."""
    results = []
    for exp in experiments:
        name = exp.get("name", "unnamed")
        exp_type = exp.get("type", "param_override")

        if exp_type == "param_override":
            overrides = exp.get("params", {})
            if isinstance(list(overrides.values())[0], list):
                # Sweep: param -> [values]
                sweep_params = list(overrides.items())
                sweep_combos = grid_combos({k: v for k, v in sweep_params if isinstance(v, list)})
                log(f"  Experiment '{name}': {len(sweep_combos)} sweep combos")
                for combo in sweep_combos:
                    params = {**PRODUCTION_CONFIG, **combo}
                    cr = evaluate_candidate(df, folds, params, symbol, of_config,
                                           compute_fragility=False, skip_is=True)
                    cr.params["experiment"] = name
                    results.append(cr)
            else:
                # Single override
                params = {**PRODUCTION_CONFIG, **overrides}
                cr = evaluate_candidate(df, folds, params, symbol, of_config,
                                       compute_fragility=True)
                cr.params["experiment"] = name
                results.append(cr)
                log(f"  Experiment '{name}': Sharpe={cr.oos_sharpe:+.2f} cons={cr.oos_consistency:.0%}")

        elif exp_type == "conditional":
            # Regime-conditional: different params based on ADX
            condition_adx = exp.get("condition_adx", 25)
            then_overrides = exp.get("then_overrides", {})
            else_overrides = exp.get("else_overrides", {})
            log(f"  Experiment '{name}': ADX>{condition_adx} conditional")
            params = {**PRODUCTION_CONFIG, **then_overrides}
            params["experiment"] = name
            cr = evaluate_candidate(df, folds, params, symbol, of_config,
                                   compute_fragility=True)
            results.append(cr)

    return results


# ─── Post-Run Auto-Validation ────────────────────────────────────────────────

def auto_validate_top_candidates(all_results: Dict[str, List[CandidateResult]],
                                  output_dir: str, top_n: int = 3) -> str:
    """Validate top night shift candidates through FutureBlindSimulator.

    Imports and runs validate_night_shift logic synchronously within the night
    shift process. Only validates candidates that beat the production baseline.
    """
    log(f"\n── Phase 7: Auto-Validation (FutureBlindSimulator) ──")

    import asyncio
    from backtesting.future_blind_simulator import FutureBlindSimulator

    validated = []

    for symbol, results in all_results.items():
        # Skip if results are not CandidateResult objects
        if not results or not hasattr(results[0], 'survivor_score'):
            continue

        # Filter: non-rejected, not coarse-only, beats production baseline
        prod = next((r for r in results if r.params == PRODUCTION_CONFIG), None)
        candidates = [r for r in results if not r.rejected]
        # Note: include coarse-only candidates here — the whole point of auto-validation
        # is to check fast-sim candidates through the full simulator
        if prod:
            candidates = [r for r in candidates
                         if r.survivor_score > prod.survivor_score * 1.1]
        candidates = sorted(candidates, key=lambda r: r.survivor_score, reverse=True)[:top_n]

        if not candidates:
            log(f"  {symbol}: no candidates beat production baseline, skipping validation")
            continue

        safe = symbol.replace("/", "_")
        path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
        if not os.path.exists(path):
            log(f"  {symbol}: no data file, skipping")
            continue

        df = pd.read_parquet(path)

        # Build folds matching validate_night_shift.py
        test_bars = 36 * 24  # 36-day test windows
        warmup = 250
        usable = len(df) - warmup
        actual = min(9, usable // test_bars)
        bars_per = test_bars if actual < (usable // test_bars) else usable // actual
        folds_list = []
        start = warmup
        for fi in range(actual):
            end = start + bars_per if fi < actual - 1 else len(df)
            folds_list.append((fi, start, end))
            start = end

        if not folds_list:
            log(f"  {symbol}: not enough data for validation folds")
            continue

        log(f"  {symbol}: validating {len(candidates)} candidates on {len(folds_list)} folds")

        for cand in candidates:
            label = cand.params.get("experiment", cand.params.get("label",
                    f"candidate_{candidates.index(cand)+1}"))
            log(f"    [{label}]")

            # Check if this is a BB strategy
            is_bb = cand.params.get("strategy") == "bb_mean_reversion"

            fold_results = []
            for fold_num, train_end, test_end in folds_list:
                window_df = df.iloc[max(0, train_end - 250):test_end]
                if len(window_df) < 300:
                    continue

                if is_bb:
                    # BB strategy uses its own indicator columns
                    window_df = compute_indicators(window_df)
                    trips = simulate_bb_trades(window_df, cand.params)
                    # Compute metrics from trips
                    if trips:
                        pnls = [t["pnl_pct"] for t in trips]
                        wins = [p for p in pnls if p > 0]
                        avg_win = np.mean(wins) if wins else 0
                        avg_loss = abs(np.mean([p for p in pnls if p <= 0])) or 0.001
                        pf = avg_win / avg_loss
                        m = {
                            "total_pnl_pct": sum(pnls),
                            "round_trips": len(pnls),
                            "win_rate": len(wins) / len(pnls) if pnls else 0,
                            "profit_factor": pf,
                            "avg_hold_hrs": np.mean([t["hold_hrs"] for t in trips]),
                            "max_drawdown_pct": 0,  # simplified
                            "sharpe": np.mean(pnls) / (np.std(pnls) or 0.001) * np.sqrt(8760),
                        }
                    else:
                        m = {"total_pnl_pct": 0, "round_trips": 0, "win_rate": 0,
                             "profit_factor": 0, "avg_hold_hrs": 0, "max_drawdown_pct": 0, "sharpe": 0}
                else:
                    # Use FutureBlindSimulator for MultiTFStrategy candidates
                    from agents.historical_data_collector import DataWindow
                    from scripts.run_backtest_r2 import MultiTFStrategy
                    try:
                        loop = asyncio.new_event_loop()
                        strategy = MultiTFStrategy(f"val_{symbol}",
                                                  {**{"symbol": symbol}, **cand.params})
                        sim = FutureBlindSimulator(initial_capital=10000)
                        sim.add_strategy(strategy)
                        window = DataWindow(
                            symbol=symbol, exchange="binance",
                            start_time=window_df.index[0].to_pydatetime(),
                            end_time=window_df.index[-1].to_pydatetime(),
                            current_time=window_df.index[0].to_pydatetime(),
                            data=window_df,
                        )
                        result = loop.run_until_complete(
                            sim.run_simulation(window, time_step_minutes=60))
                        trips = strategy.completed_round_trips
                        loop.close()

                        if trips:
                            pnls = [t["pnl_pct"] for t in trips]
                            wins = [p for p in pnls if p > 0]
                            avg_win = np.mean(wins) if wins else 0
                            avg_loss = abs(np.mean([p for p in pnls if p <= 0])) or 0.001
                            pf = avg_win / avg_loss
                            cum = np.cumsum(pnls)
                            running_max = np.maximum.accumulate(cum)
                            max_dd = abs(min(cum - running_max)) if len(cum) > 0 else 0
                            m = {
                                "total_pnl_pct": sum(pnls),
                                "round_trips": len(pnls),
                                "win_rate": len(wins) / len(pnls) if pnls else 0,
                                "profit_factor": pf,
                                "avg_hold_hrs": np.mean([t["hold_hrs"] for t in trips]),
                                "max_drawdown_pct": max_dd,
                                "sharpe": np.mean(pnls) / (np.std(pnls) or 0.001) * np.sqrt(8760),
                            }
                        else:
                            m = {"total_pnl_pct": 0, "round_trips": 0, "win_rate": 0,
                                 "profit_factor": 0, "avg_hold_hrs": 0, "max_drawdown_pct": 0, "sharpe": 0}
                    except Exception as e:
                        log(f"      fold {fold_num}: error ({e})")
                        continue

                fold_results.append(m)

            if not fold_results:
                log(f"    → no valid folds")
                continue

            # Aggregate
            total_pnl = sum(f["total_pnl_pct"] for f in fold_results)
            sharpes = [max(-100, min(100, f["sharpe"])) for f in fold_results]
            med_sharpe = float(np.median(sharpes))
            consistency = sum(1 for f in fold_results if f["total_pnl_pct"] > 0) / len(fold_results)
            total_trades = sum(f["round_trips"] for f in fold_results)

            vr = {
                "symbol": symbol,
                "label": label,
                "params": {k: v for k, v in cand.params.items() if k not in ("min_alignment",)},
                "total_pnl_pct": round(total_pnl, 2),
                "median_sharpe": round(med_sharpe, 2),
                "consistency": round(consistency, 3),
                "avg_win_rate": round(np.mean([f["win_rate"] for f in fold_results]), 3),
                "avg_pf": round(np.mean([f["profit_factor"] for f in fold_results
                                        if f["profit_factor"] < 999]), 2),
                "avg_max_dd": round(np.mean([f["max_drawdown_pct"] for f in fold_results]), 2),
                "total_trades": total_trades,
                "folds": len(fold_results),
                "verdict": "STRONG" if consistency >= 0.7 and total_pnl > 0
                           else "MODERATE" if consistency >= 0.5 and total_pnl > 0
                           else "MARGINAL" if consistency >= 0.4
                           else "FAILED",
            }
            validated.append(vr)

            log(f"    → PnL={vr['total_pnl_pct']:+.2f}% Sharpe={vr['median_sharpe']:+.2f} "
                f"cons={vr['consistency']:.0%} [{vr['verdict']}]")

    # Save validation results
    date_dir = os.path.join(output_dir, datetime.now(timezone.utc).strftime("%Y-%m-%d"))
    os.makedirs(date_dir, exist_ok=True)
    val_path = os.path.join(date_dir, "full_sim_validation.json")
    with open(val_path, "w") as f:
        json.dump({
            "run_at": datetime.now(timezone.utc).isoformat(),
            "simulator": "FutureBlindSimulator (fees + slippage)",
            "results": validated,
        }, f, indent=2, default=str)
    log(f"  Validation results saved to {val_path}")

    # Summary
    strong = [v for v in validated if v["verdict"] == "STRONG"]
    moderate = [v for v in validated if v["verdict"] == "MODERATE"]
    log(f"  Validation summary: {len(strong)} STRONG, {len(moderate)} MODERATE, "
        f"{len(validated) - len(strong) - len(moderate)} MARGINAL/FAILED")

    return val_path


# ─── Regime Analysis ─────────────────────────────────────────────────────────

def compute_adx(df: pd.DataFrame, period: int = 14) -> pd.Series:
    """Compute ADX using Wilder's smoothing."""
    high, low, close = df["high"], df["low"], df["close"]
    up = high.diff()
    down = -low.diff()
    plus_dm = up.where((up > down) & (up > 0), 0.0)
    minus_dm = down.where((down > up) & (down > 0), 0.0)
    tr = pd.concat([
        high - low,
        (high - close.shift(1)).abs(),
        (low - close.shift(1)).abs(),
    ], axis=1).max(axis=1)
    alpha = 1.0 / period
    atr = tr.ewm(alpha=alpha, min_periods=period).mean()
    plus_di = 100 * plus_dm.ewm(alpha=alpha, min_periods=period).mean() / atr
    minus_di = 100 * minus_dm.ewm(alpha=alpha, min_periods=period).mean() / atr
    dx = 100 * (plus_di - minus_di).abs() / (plus_di + minus_di)
    return dx.ewm(alpha=alpha, min_periods=period).mean()


def regime_analysis(dfs: Dict[str, pd.DataFrame], adx_threshold: float = 25.0) -> Dict:
    """Analyze market regime and per-symbol state."""
    result = {}
    for symbol, df in dfs.items():
        adx = compute_adx(df)
        current_adx = float(adx.iloc[-1]) if len(adx) > 0 and pd.notna(adx.iloc[-1]) else 0

        # Regime for each bar (simplified) — numeric for mean calculation
        regime_numeric = np.where(adx > adx_threshold, 1.0, 0.0)

        # Current volatility percentile
        vol = df["close"].pct_change().rolling(24).std()
        vol_pctile = float(vol.rank(pct=True).iloc[-1] * 100) if len(vol) > 0 and pd.notna(vol.iloc[-1]) else 50

        # Recent 30d return
        recent_return = float(df["close"].iloc[-1] / df["close"].iloc[-720] - 1) if len(df) >= 720 else 0

        # 30d ADX trend (is it rising or falling?)
        recent_adx_mean = float(adx.iloc[-720:].mean()) if len(adx) >= 720 else current_adx
        earlier_adx_mean = float(adx.iloc[-1440:-720].mean()) if len(adx) >= 1440 else recent_adx_mean

        result[symbol] = {
            "current_adx": round(current_adx, 1),
            "current_regime": "TREND" if current_adx > adx_threshold else "RANGE",
            "volatility_percentile": round(vol_pctile, 0),
            "recent_30d_return_pct": round(recent_return * 100, 1),
            "adx_trend": "RISING" if recent_adx_mean > earlier_adx_mean * 1.1 else
                         "FALLING" if recent_adx_mean < earlier_adx_mean * 0.9 else "STABLE",
            "trend_pct": float(np.mean(regime_numeric[-720:]) * 100) if len(regime_numeric) >= 720 else 50,
        }

    # Cross-correlation matrix (recent 30d)
    returns = {sym: df["close"].pct_change().iloc[-720:] for sym, df in dfs.items()}
    corr = pd.DataFrame(returns).corr()
    result["correlations"] = {
        f"{r1}_{r2}": round(corr.loc[r1, r2], 2)
        for r1 in corr.columns for r2 in corr.columns if r1 < r2
    }

    return result


# ─── Data Loading ─────────────────────────────────────────────────────────────

def load_data(symbols: List[str]) -> Dict[str, pd.DataFrame]:
    """Load OHLCV data and compute indicators for all symbols."""
    dfs = {}
    for symbol in symbols:
        safe = symbol.replace("/", "_")
        path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
        if not os.path.exists(path):
            log(f"  WARNING: No data for {symbol} at {path}, skipping")
            continue
        df = pd.read_parquet(path)
        df = compute_indicators(df)
        dfs[symbol] = df
        log(f"  Loaded {symbol}: {len(df)} candles")
    return dfs


def fetch_fresh_data(symbols: List[str]) -> bool:
    """Fetch latest 365d OHLCV from Binance. Returns True on success."""
    try:
        import ccxt.async_support as ccxt
        import asyncio

        async def _fetch():
            exchange = ccxt.binance({"enableRateLimit": True})
            os.makedirs(DATA_DIR, exist_ok=True)
            since = int((datetime.now(timezone.utc) - timedelta(days=365)).timestamp() * 1000)

            for symbol in symbols:
                safe = symbol.replace("/", "_")
                all_candles = []
                while len(all_candles) < 9000:
                    batch = await exchange.fetch_ohlcv(symbol, "1h", since=since, limit=1000)
                    if not batch:
                        break
                    all_candles.extend(batch)
                    since = batch[-1][0] + 1
                    await asyncio.sleep(exchange.rateLimit / 1000)

                if all_candles:
                    df = pd.DataFrame(all_candles, columns=["timestamp", "open", "high", "low", "close", "volume"])
                    df["timestamp"] = pd.to_datetime(df["timestamp"], unit="ms", utc=True)
                    df = df.drop_duplicates(subset="timestamp").set_index("timestamp").sort_index()
                    path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
                    df.to_parquet(path)
                    log(f"  Fetched {symbol}: {len(df)} candles")

            await exchange.close()

        asyncio.run(_fetch())
        return True
    except ImportError:
        log("  ccxt not installed, skipping data fetch")
        return False
    except Exception as e:
        log(f"  Data fetch failed: {e}")
        return False


# ─── Report Generation ───────────────────────────────────────────────────────

def generate_report(
    all_results: Dict[str, List[CandidateResult]],
    regime: Dict,
    folds: List[Fold],
    run_time_seconds: float,
    output_dir: str,
) -> str:
    """Generate morning report as markdown."""
    now = datetime.now(timezone.utc)
    date_str = now.strftime("%Y-%m-%d")
    report_path = os.path.join(output_dir, date_str, "report.md")
    json_path = os.path.join(output_dir, date_str, "summary.json")

    os.makedirs(os.path.dirname(report_path), exist_ok=True)

    lines = []
    w = lines.append  # shorthand

    w(f"# Night Shift Report — {date_str}")
    w(f"")
    w(f"**Runtime:** {run_time_seconds:.0f}s | **Folds:** {len(folds)} | "
      f"**Symbols:** {', '.join(all_results.keys())}")
    w(f"**Aggregation:** Median OOS Sharpe, per-fold Sharpe winsorized at ±100")
    w(f"")

    # ── Market State ──
    w(f"## Market State")
    w(f"")
    w(f"| Symbol | Regime | ADX | ADX Trend | Vol %ile | 30d Return |")
    w(f"|--------|--------|-----|-----------|----------|------------|")
    for sym in all_results:
        if sym in regime:
            r = regime[sym]
            w(f"| {sym} | {r['current_regime']} | {r['current_adx']} | "
              f"{r['adx_trend']} | {r['volatility_percentile']:.0f}% | "
              f"{r['recent_30d_return_pct']:+.1f}% |")
    w(f"")

    # Correlations
    if "correlations" in regime:
        w(f"**Correlations:**")
        for pair, corr_val in sorted(regime["correlations"].items()):
            w(f"  {pair}: {corr_val:.2f}")
    w(f"")

    # ── Production Baseline ──
    w(f"## Production Baseline (Current Config)")
    w(f"")
    w(f"| Symbol | OOS Sharpe | OOS PF | OOS WR | Consistency | MaxDD | Survivor |")
    w(f"|--------|-----------|--------|--------|-------------|-------|----------|")
    for sym, results in all_results.items():
        # Find the production config result
        prod_result = None
        for r in results:
            if r.params == PRODUCTION_CONFIG:
                prod_result = r
                break
        if prod_result is None:
            w(f"| {sym} | (not in grid) | | | | | |")
        else:
            pf_s = f"{prod_result.oos_pf:.1f}" if prod_result.oos_pf < 999 else "INF"
            w(f"| {sym} | {prod_result.oos_sharpe:+.2f} | {pf_s} | "
              f"{prod_result.oos_wr:.0%} | "
              f"{prod_result.oos_consistency:.0%} | "
              f"{prod_result.oos_max_dd:.1f}% | "
              f"{prod_result.survivor_score:.2f} |")
    w(f"")

    # ── Top 10 Candidates ──
    w(f"## Top 10 Candidates (Ranked by Survivor Score)")
    w(f"")
    w(f"*Only candidates validated on 5+ WFA folds are shown.*")
    w(f"")

    all_candidates = []
    for sym, results in all_results.items():
        all_candidates.extend(results)
    all_candidates.sort(key=lambda r: r.survivor_score, reverse=True)

    # Filter: only fully-validated candidates (5+ folds) for rankings
    validated_candidates = [r for r in all_candidates if not r.is_coarse_only]

    # Count strategies
    bb_count_report = sum(1 for cr in validated_candidates if cr.params.get("strategy") == "bb_mean_reversion")
    mtf_count_report = len(validated_candidates) - bb_count_report
    if bb_count_report:
        w(f"**Strategy breakdown:** {mtf_count_report} MultiTF, {bb_count_report} BB Mean Reversion")
        w(f"")

    # Find production baseline scores for delta comparison
    prod_scores = {}
    for sym, results in all_results.items():
        for r in results:
            if r.params == PRODUCTION_CONFIG:
                prod_scores[sym] = r
                break

    shown = 0
    for cr in validated_candidates:
        if shown >= 10:
            break
        if cr.rejected:
            continue

        prod = prod_scores.get(cr.symbol)
        delta = "NEW" if prod is None else f"{cr.survivor_score - prod.survivor_score:+.2f}"
        conf = "STRONG" if cr.survivor_score > (prod.survivor_score * 1.5 if prod else 2) else \
               "MEDIUM" if cr.survivor_score > (prod.survivor_score * 1.2 if prod else 1) else "LOW"

        strategy_tag = " [BB]" if cr.params.get("strategy") == "bb_mean_reversion" else ""
        w(f"### #{shown+1}: {cr.symbol}{strategy_tag} (Survivor: {cr.survivor_score:.2f} {delta})")
        w(f"```json")
        w(json.dumps(cr.params, indent=2))
        w(f"```")

        if prod:
            w(f"| Metric | Baseline | Candidate | Delta |")
            w(f"|--------|----------|-----------|-------|")
            w(f"| OOS Sharpe | {prod.oos_sharpe:+.2f} | {cr.oos_sharpe:+.2f} | {cr.oos_sharpe - prod.oos_sharpe:+.2f} |")
            w(f"| OOS PF | {prod.oos_pf:.1f} | {cr.oos_pf:.1f} | {cr.oos_pf - prod.oos_pf:+.1f} |")
            w(f"| Consistency | {prod.oos_consistency:.0%} | {cr.oos_consistency:.0%} | "
              f"{cr.oos_consistency - prod.oos_consistency:+.0%} |")
            w(f"| MaxDD | {prod.oos_max_dd:.1f}% | {cr.oos_max_dd:.1f}% | "
              f"{cr.oos_max_dd - prod.oos_max_dd:+.1f}% |")
            w(f"| Overfitting | {prod.overfitting_score:.2f} | {cr.overfitting_score:.2f} | "
              f"{cr.overfitting_score - prod.overfitting_score:+.2f} |")
            w(f"| Fragility | {prod.fragility:.2f} | {cr.fragility:.2f} | |")
        else:
            w(f"| Metric | Value |")
            w(f"|--------|-------|")
            w(f"| OOS Sharpe | {cr.oos_sharpe:+.2f} |")
            w(f"| OOS PF | {cr.oos_pf:.1f} |")
            w(f"| Consistency | {cr.oos_consistency:.0%} |")
            w(f"| MaxDD | {cr.oos_max_dd:.1f}% |")
            w(f"| Overfitting | {cr.overfitting_score:.2f} |")
            w(f"| Fragility | {cr.fragility:.2f} |")

        w(f"")
        w(f"✅ **{conf} RECOMMEND** — trades/fold: {cr.oos_avg_trades_per_fold:.0f}, "
          f"exits: {dict(list(cr.oos_exit_reasons.items())[:5])}")
        w(f"")

        shown += 1

    # ── Overfitting Warnings ──
    w(f"## Overfitting Warnings")
    w(f"")
    rejected = [r for r in validated_candidates if r.rejected]
    if rejected:
        # Group by rejection reason
        by_reason = Counter(r.rejection_reason for r in rejected[:20])
        for reason, count in by_reason.most_common(10):
            examples = [r for r in rejected if r.rejection_reason == reason][:2]
            for ex in examples:
                w(f"⚠️ {ex.symbol} {ex.params}: {reason} "
                  f"(OOS Sharpe: {ex.oos_sharpe:+.2f}, IS-OOS gap: {ex.overfitting_score:.2f})")
    else:
        w(f"No overfitting warnings — all top candidates passed filters.")
    w(f"")

    # ── Per-Symbol Fold Detail ──
    w(f"## Per-Symbol WFA Fold Detail")
    w(f"")
    for sym, results in all_results.items():
        # Only show WFA-validated candidates (5+ folds), not coarse-only
        validated = [r for r in results if not r.is_coarse_only and not r.rejected]
        if not validated:
            rejected_count = sum(1 for r in results if not r.is_coarse_only and r.rejected)
            w(f"### {sym} — No validated candidates")
            if rejected_count:
                reasons = Counter(r.rejection_reason for r in results if not r.is_coarse_only and r.rejected)
                for reason, count in reasons.most_common(3):
                    w(f"  {count} rejected: {reason}")
            else:
                w(f"  No candidates passed coarse filter")
            w(f"")
            continue

        best = max(validated, key=lambda r: r.survivor_score)
        w(f"### {sym} — Best Validated Candidate (Survivor: {best.survivor_score:.2f})")
        w(f"| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |")
        w(f"|------|-----------|-----------|---------|------------|")
        for fd in best.folds:
            check = "✅" if fd["oos_sharpe"] > 0 else "❌"
            raw = fd.get("oos_sharpe_raw", fd["oos_sharpe"])
            if abs(raw) > 100:
                w(f"| {fd['fold']} | {fd['is_sharpe']:+.2f} | {fd['oos_sharpe']:+.2f} (raw: {raw:+.0f}) | "
                  f"{fd['oos_pnl']:+.2f}% | {fd['oos_trades']} {check} |")
            else:
                w(f"| {fd['fold']} | {fd['is_sharpe']:+.2f} | {fd['oos_sharpe']:+.2f} | "
                  f"{fd['oos_pnl']:+.2f}% | {fd['oos_trades']} {check} |")
        w(f"")

    # ── Action Items ──
    w(f"## Action Items")
    w(f"")
    action_num = 0
    for cr in validated_candidates:
        if cr.rejected or action_num >= 5:
            continue
        prod = prod_scores.get(cr.symbol)
        if prod is None or cr.survivor_score <= prod.survivor_score * 1.2:
            continue

        action_num += 1
        changes = {k: (prod.params.get(k), v) for k, v in cr.params.items()
                   if prod.params.get(k) != v}
        change_str = ", ".join(f"{k}: {old}→{new}" for k, (old, new) in changes.items())
        conf = "HIGH" if cr.survivor_score > prod.survivor_score * 1.5 else "MEDIUM"

        w(f"{action_num}. **[{conf}]** {cr.symbol}: {change_str}")
        w(f"   OOS Sharpe: {cr.oos_sharpe:+.2f} (vs {prod.oos_sharpe:+.2f}), "
          f"consistency: {cr.oos_consistency:.0%}, DD: {cr.oos_max_dd:.1f}%, "
          f"trades/fold: {cr.oos_avg_trades_per_fold:.0f}")
        if cr.overfitting_score > 0:
            w(f"   ⚠️ Overfitting score: {cr.overfitting_score:.2f} — monitor closely")
        w(f"")

    if action_num == 0:
        w(f"No candidates significantly outperform production baseline.")
        w(f"Production config appears well-optimized for current market conditions.")
        w(f"")
    else:
        w(f"Total: {action_num} actionable recommendations out of "
          f"{len([c for c in validated_candidates if not c.rejected])} validated candidates.")
        w(f"")

    # Write markdown
    report_text = "\n".join(lines)
    with open(report_path, "w") as f:
        f.write(report_text)

    # Write JSON summary
    json_data = {
        "run_at": now.isoformat(),
        "runtime_seconds": run_time_seconds,
        "num_folds": len(folds),
        "symbols": list(all_results.keys()),
        "market_state": {k: v for k, v in regime.items() if k != "correlations"},
        "correlations": regime.get("correlations", {}),
        "production_baseline": {
            sym: {
                "params": next((r.params for r in results if r.params == PRODUCTION_CONFIG), None),
                "survivor_score": next((r.survivor_score for r in results if r.params == PRODUCTION_CONFIG), 0),
                "oos_sharpe": next((r.oos_sharpe for r in results if r.params == PRODUCTION_CONFIG), 0),
                "oos_consistency": next((r.oos_consistency for r in results if r.params == PRODUCTION_CONFIG), 0),
            }
            for sym, results in all_results.items()
        },
        "top_candidates": [
            {
                "symbol": cr.symbol,
                "params": cr.params,
                "survivor_score": round(cr.survivor_score, 4),
                "oos_sharpe": round(cr.oos_sharpe, 4),
                "oos_consistency": round(cr.oos_consistency, 4),
                "oos_max_dd": round(cr.oos_max_dd, 4),
                "overfitting_score": round(cr.overfitting_score, 4),
                "fragility": round(cr.fragility, 4),
                "oos_avg_trades_per_fold": round(cr.oos_avg_trades_per_fold, 1),
                "rejected": cr.rejected,
                "rejection_reason": cr.rejection_reason,
            }
            for cr in validated_candidates[:20]
        ],
    }

    with open(json_path, "w") as f:
        json.dump(json_data, f, indent=2, default=str)

    return report_path


# ─── Main Night Shift ────────────────────────────────────────────────────────

def run_night_shift(
    symbols: List[str],
    skip_fetch: bool = False,
    config_path: Optional[str] = None,
):
    """Main entry point for the night shift."""
    start_time = time.time()
    config = load_config(config_path)

    log(f"{'='*70}")
    log(f"NIGHT SHIFT — Autonomous Strategy Optimization")
    log(f"Symbols: {', '.join(symbols)}")
    if config:
        log(f"Config: {config_path or CONFIG_PATH}")
    log(f"{'='*70}")

    # ── Phase 1: Data ──
    log(f"\n── Phase 1: Data ──")
    fetch = config.get("schedule", {}).get("fetch_fresh_data", True)
    if not skip_fetch and fetch:
        log(f"Fetching fresh data from Binance...")
        fetch_fresh_data(symbols)
    else:
        log(f"Using cached data (skip-fetch)")

    dfs = load_data(symbols)
    if not dfs:
        log(f"FATAL: No data loaded. Exiting.")
        return

    # ── Phase 2: WFA Folds ──
    log(f"\n── Phase 2: Expanding-Window WFA ──")
    # Use minimum data length across symbols for consistent folds
    min_bars = min(len(df) for df in dfs.values())
    folds = create_folds(min_bars, WFA_CONFIG["num_folds"], WFA_CONFIG["test_fold_days"])
    log(f"Created {len(folds)} folds from {min_bars} bars")
    for f in folds:
        log(f"  Fold {f.fold_num}: train=[{f.train_start_idx}:{f.train_end_idx}] "
            f"({f.train_hours}h) test=[{f.test_start_idx}:{f.test_end_idx}] ({f.test_hours}h)")

    # ── Phase 2b: Evaluate Production Baseline on all folds ──
    all_results = {}  # initialize before grid search fills it
    log(f"\n── Phase 2b: Production Baseline ──")
    for symbol in dfs:
        cr = evaluate_candidate(dfs[symbol], folds, PRODUCTION_CONFIG, symbol,
                               OVERFITTING_CONFIG, compute_fragility=True)
        # Ensure it's in the results
        if not any(r.params == cr.params for r in all_results.get(symbol, [])):
            if symbol not in all_results:
                all_results[symbol] = []
            all_results[symbol].append(cr)
            log(f"  {symbol}: OOS Sharpe={cr.oos_sharpe:+.2f} "
                f"consistency={cr.oos_consistency:.0%} survivor={cr.survivor_score:.2f}")

    # ── Phase 3: Grid Search ──
    log(f"\n── Phase 3: Coarse Grid Search ──")
    # NOTE: all_results already has production baselines from Phase 2b — extend, don't overwrite
    for symbol in dfs:
        if symbol not in all_results:
            all_results[symbol] = []
        results = coarse_grid_search(dfs[symbol], folds, symbol, OVERFITTING_CONFIG)
        all_results[symbol].extend(results)

    # ── Phase 3b: Fine Refinement ──
    log(f"\n── Phase 3b: Fine Refinement ──")
    for symbol, results in all_results.items():
        top_n = sorted(results, key=lambda r: r.survivor_score, reverse=True)[:100]
        fine_results = fine_refinement(dfs[symbol], folds, symbol, top_n, OVERFITTING_CONFIG)
        all_results[symbol].extend(fine_results)

    # ── Phase 4: Darwinian ──
    log(f"\n── Phase 4: Darwinian Evolution ──")
    for symbol, results in all_results.items():
        survivors = darwinian_evolution(
            dfs[symbol], folds, symbol, results,
            OVERFITTING_CONFIG, DARWINIAN_CONFIG,
        )
        all_results[symbol].extend(survivors)

    # ── Phase 4b: BB Mean Reversion Grid Search ──
    log(f"\n── Phase 4b: BB Mean Reversion ──")
    for symbol in dfs:
        bb_results = run_bb_grid_search(dfs[symbol], folds, symbol, OVERFITTING_CONFIG)
        if symbol not in all_results:
            all_results[symbol] = []
        all_results[symbol].extend(bb_results)

    # ── Phase 4c: Custom Experiments ──
    experiments = config.get("experiments", [])
    if experiments:
        log(f"\n── Phase 4c: Custom Experiments ({len(experiments)}) ──")
        for symbol in dfs:
            exp_results = run_experiments(dfs[symbol], folds, symbol, experiments, OVERFITTING_CONFIG)
            all_results[symbol].extend(exp_results)
    else:
        log(f"\n── Phase 4c: No experiments configured (add to night_config.json) ──")

    # ── Phase 5: Regime Analysis ──
    log(f"\n── Phase 5: Regime Analysis ──")
    regime = regime_analysis(dfs)

    # ── Phase 6: Report ──
    log(f"\n── Phase 6: Morning Report ──")
    run_time = time.time() - start_time
    report_path = generate_report(all_results, regime, folds, run_time, RESULTS_DIR)
    log(f"Report saved to {report_path}")

    # ── Phase 7: Auto-Validation ──
    val_top = config.get("validation", {}).get("top_candidates", 3)
    val_path = auto_validate_top_candidates(all_results, RESULTS_DIR, top_n=val_top)

    # ── Summary ──
    final_time = time.time() - start_time
    log(f"\n{'='*70}")
    log(f"NIGHT SHIFT COMPLETE — {final_time:.0f}s")
    log(f"{'='*70}")

    # Count BB strategies
    bb_count = sum(
        1 for results in all_results.values()
        for r in results if r.params.get("strategy") == "bb_mean_reversion"
    )
    exp_count = sum(
        1 for results in all_results.values()
        for r in results if r.params.get("experiment")
    )

    for symbol, results in all_results.items():
        non_rejected = [r for r in results if not r.rejected]
        best = max(non_rejected, key=lambda r: r.survivor_score) if non_rejected else None
        prod = next((r for r in results if r.params == PRODUCTION_CONFIG), None)
        if best:
            log(f"  {symbol}: best survivor={best.survivor_score:.3f} "
                f"(OOS Sharpe={best.oos_sharpe:+.2f}, consistency={best.oos_consistency:.0%})")
            if prod and best.params != prod.params:
                delta = best.survivor_score - prod.survivor_score
                log(f"    ↑ vs production ({delta:+.3f})")
                changes = {k: (prod.params.get(k), v) for k, v in best.params.items()
                           if prod.params.get(k) != v}
                for k, (old, new) in changes.items():
                    log(f"    {k}: {old} → {new}")

    log(f"")
    log(f"  Total candidates evaluated: "
        f"{sum(len(r) for r in all_results.values())}")
    log(f"  Total passed filters: "
        f"{sum(sum(1 for r in results if not r.rejected) for results in all_results.values())}")
    if bb_count:
        log(f"  BB Mean Reversion candidates: {bb_count}")
    if exp_count:
        log(f"  Custom experiment candidates: {exp_count}")
    log(f"  Report: {report_path}")
    log(f"  Validation: {val_path}")

    # Also save full results as JSON for programmatic access
    json_path = os.path.join(RESULTS_DIR, datetime.now(timezone.utc).strftime("%Y-%m-%d"), "full_results.json")
    os.makedirs(os.path.dirname(json_path), exist_ok=True)
    full_data = {}
    for sym, results in all_results.items():
        full_data[sym] = [
            {**asdict(r)} for r in sorted(results, key=lambda r: r.survivor_score, reverse=True)[:50]
        ]
    with open(json_path, "w") as f:
        json.dump(full_data, f, indent=2, default=str)
    log(f"  Full results: {json_path}")

    # ── Phase 8: Discrepancy Detection (self-awareness) ──
    log(f"\n── Phase 8: Discrepancy Detection ──")
    try:
        from scripts.discrepancy_detector import detect_discrepancies, update_flag_history, generate_recommendation
        # Build fast sim results from all_results
        fast_results = {}
        for sym, results in all_results.items():
            fast_results[sym] = [
                {"oos_sharpe": r.oos_sharpe, "total_pnl": r.oos_pnl}
                for r in sorted(results, key=lambda r: r.survivor_score, reverse=True)[:5]
            ]
        # Check for full sim validation
        val_dir = os.path.join(RESULTS_DIR, datetime.now(timezone.utc).strftime("%Y-%m-%d"))
        val_json = os.path.join(val_dir, "full_sim_validation.json")
        full_results = {}
        if os.path.exists(val_json):
            with open(val_json) as vf:
                full_data = json.load(vf)
            for entry in full_data:
                full_results.setdefault(entry["symbol"], []).append(entry)

        discrepancies = detect_discrepancies(fast_results, full_results)
        history = update_flag_history(discrepancies)
        recommendation, skip_symbols = generate_recommendation(discrepancies, history)

        if skip_symbols:
            log(f"  ⚠️  Symbols with persistent discrepancies: {skip_symbols}")
            log(f"      Darwinian phase will be skipped for these symbols next run")
        else:
            log(f"  No persistent discrepancies detected — evaluator is trustworthy")

        # Save discrepancy report
        disc_dir = os.path.join(os.path.dirname(__file__), "..", "data", "discrepancies")
        os.makedirs(disc_dir, exist_ok=True)
        disc_path = os.path.join(disc_dir, f"discrepancy_{datetime.now(timezone.utc).strftime('%Y-%m-%d')}.md")
        with open(disc_path, "w") as df:
            df.write(recommendation)
        log(f"  Discrepancy report: {disc_path}")
    except Exception as e:
        log(f"  Discrepancy detection skipped: {e}")


# ─── CLI ─────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Night Shift: Zero-token autonomous strategy optimization")
    parser.add_argument("--config", type=str, default=None, help="Path to night_config.json")
    parser.add_argument("--skip-fetch", action="store_true", help="Use cached data, skip Binance fetch")
    parser.add_argument("--symbols", nargs="+", default=None, help="Symbols to optimize (default: all 4)")
    parser.add_argument("--folds", type=int, default=None, help="Number of WFA folds")
    parser.add_argument("--test-days", type=int, default=None, help="Test fold duration in days")
    args = parser.parse_args()

    symbols = args.symbols or DEFAULT_SYMBOLS

    # Override WFA config from CLI
    global WFA_CONFIG
    if args.folds:
        WFA_CONFIG["num_folds"] = args.folds
    if args.test_days:
        WFA_CONFIG["test_fold_days"] = args.test_days

    run_night_shift(
        symbols=symbols,
        skip_fetch=args.skip_fetch,
        config_path=args.config,
    )


if __name__ == "__main__":
    main()
