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
  python -m research.orchestration.night_shift
  python -m research.orchestration.night_shift --config research/orchestration/night_config.json
  python -m research.orchestration.night_shift --skip-fetch
  python -m research.orchestration.night_shift --symbols BTC/USDT ETH/USDT
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
from typing import Dict, List, Optional

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

# Paths

ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
DATA_DIR = os.path.join(ROOT, "data", "ohlcv")
RESULTS_DIR = os.path.join(ROOT, "data", "night_results")
CONFIG_PATH = os.path.join(os.path.dirname(__file__), "night_config.json")
PRODUCTION_CONFIG_PATH = os.path.join(ROOT, "knowledge_base", "production_config.json")

# Config Loader

def load_config(config_path: Optional[str] = None) -> dict:
    """Load night_config.json if it exists."""
    path = config_path or CONFIG_PATH
    if os.path.exists(path):
        with open(path) as f:
            return json.load(f)
    return {}


# Defaults

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

# Fine grid for refinement around top candidates — leverage sweep only (3x-10x)
# Top 100 × (6 trailing × 5 flip × 8 leverage) = 24,000 full WFA evaluations
# All leveraged candidates scored with compounding + Flash Trade fee model
FINE_GRID = {
    "trailing_stop_atr":   [0.0, 0.3, 0.5, 0.8, 1.0, 1.5],
    "score_flip_delay_hrs": [0, 1, 2, 3, 4],
    "leverage":            [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
}

# Flash Trade fee model (from SKILL.md / TransactionFlow.md)
FLASH_TRADE_FEES = {
    "open_fee_pct": 0.06,        # % of notional (openPositionFeePercent)
    "close_fee_pct": 0.06,       # % of notional
    "hourly_borrow_pct": 0.0042, # % of notional per hour (marginFeePercentage)
}

WFA_CONFIG = {
    "num_folds": 9,
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


# Logging

def log(msg: str):
    ts = datetime.now(timezone.utc).strftime("%H:%M:%S")
    print(f"[{ts}] {msg}", flush=True)


# WFA: Expanding-Window Folds

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


# Fast Evaluation

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
                     skip_is: bool = False,
                     starting_capital: float = None,
                     position_pct: float = 0.20) -> Dict:
    """Evaluate a config on a single train/test fold. Returns IS + OOS metrics.

    When starting_capital is provided, OOS metrics are computed using fee-adjusted
    compounding: each trade is sized at position_pct of current capital, Flash Trade
    fees (open/close/borrow) are deducted per trade, and capital carries forward.
    This gives realistic leveraged returns.
    """
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

    if starting_capital is not None:
        # --- Compounding + Flash Trade fee-adjusted OOS ---
        leverage = params.get("leverage", 1.0)
        capital = starting_capital
        peak = capital
        fold_max_dd = 0.0
        fold_wins = 0
        fold_trades = len(test_trips)
        trade_pnls = []       # net PnL % per trade (for Sharpe)
        exit_reasons = {}
        hold_times = []

        for t in test_trips:
            hold_hrs = t["hold_hrs"]
            hold_times.append(hold_hrs)
            reason = t.get("exit", "signal")

            if t.get("liquidated", False):
                # Liquidation: lose the entire margin on this position
                position = capital * position_pct
                capital -= position
                trade_pnls.append(-100.0)
                exit_reasons["liquidation"] = exit_reasons.get("liquidation", 0) + 1
            else:
                # Raw PnL already includes leverage (fast sim multiplies by leverage)
                raw_pnl_pct = t["pnl_pct"]
                # Flash Trade fees: open + close + hourly borrow, as % of margin
                fee_pct = flash_trade_round_trip_cost(leverage, hold_hrs)
                net_pnl_pct = raw_pnl_pct - fee_pct

                position = capital * position_pct
                capital += position * (net_pnl_pct / 100.0)
                trade_pnls.append(net_pnl_pct)
                if net_pnl_pct > 0:
                    fold_wins += 1
                exit_reasons[reason] = exit_reasons.get(reason, 0) + 1

            if capital > peak:
                peak = capital
            if peak > 0:
                dd = (peak - capital) / peak * 100
                fold_max_dd = max(fold_max_dd, dd)

        # Fold return
        fold_return = (capital - starting_capital) / starting_capital * 100 if starting_capital > 0 else 0

        # Fold Sharpe from per-trade returns
        if len(trade_pnls) > 1 and np.std(trade_pnls) > 0:
            fold_sharpe = np.mean(trade_pnls) / np.std(trade_pnls) * np.sqrt(len(trade_pnls) / test_hours * 8760)
        else:
            fold_sharpe = 0.0

        # Profit factor from net PnLs
        net_wins = [p for p in trade_pnls if p > 0]
        net_losses = [p for p in trade_pnls if p < 0]
        avg_win = np.mean(net_wins) if net_wins else 0
        avg_loss = abs(np.mean(net_losses)) if net_losses else 0
        fold_pf = avg_win / avg_loss if avg_loss > 0 else 999.0

        avg_hold = float(np.mean(hold_times)) if hold_times else 0

        result = {
            "is_sharpe": is_sharpe,
            "is_pnl": is_pnl,
            "oos_sharpe": fold_sharpe,
            "oos_pnl": fold_return,
            "oos_pf": fold_pf,
            "oos_wr": fold_wins / fold_trades if fold_trades > 0 else 0,
            "oos_max_dd": fold_max_dd,
            "oos_trades": fold_trades,
            "oos_avg_hold": avg_hold,
            "oos_exit_reasons": exit_reasons,
            "oos_final_capital": capital,
            "oos_fold_peak": peak,
        }
    else:
        # --- Standard additive OOS (no compounding, no fees) ---
        test_m = compute_metrics(test_trips, total_hours=test_hours)
        result = {
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

    return result


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
    leverage = params.get("leverage", 1.0)
    use_compounding = leverage > 1.0

    # When compounding, capital carries forward across sequential folds
    COMPOUND_INITIAL = 100.0
    POSITION_PCT = 0.20

    fold_results = []
    capital = COMPOUND_INITIAL
    global_peak = capital
    global_max_dd = 0.0

    for fold in folds:
        if use_compounding:
            fm = evaluate_on_fold(df, fold, params, skip_is=skip_is,
                                  starting_capital=capital, position_pct=POSITION_PCT)
            capital = fm.get("oos_final_capital", capital)
            fold_peak = fm.get("oos_fold_peak", capital)
            if fold_peak > global_peak:
                global_peak = fold_peak
            if global_peak > 0:
                dd = (global_peak - capital) / global_peak * 100
                global_max_dd = max(global_max_dd, dd)
        else:
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
    avg_oos_sharpe = float(np.median(oos_sharpes)) if oos_sharpes else 0

    if use_compounding:
        # Compounded: total return from actual capital growth, max DD from equity curve
        avg_oos_pnl = round((capital - COMPOUND_INITIAL) / COMPOUND_INITIAL * 100, 2)
        avg_oos_dd = round(global_max_dd, 2)
    else:
        # Additive: sum of per-fold PnLs, mean of per-fold DDs
        avg_oos_pnl = float(np.sum(oos_pnls)) if oos_pnls else 0
        avg_oos_dd = float(np.mean(oos_dds)) if oos_dds else 0

    avg_oos_pf = float(np.mean(oos_pfs)) if oos_pfs else 0
    avg_oos_wr = float(np.mean(oos_wrs)) if oos_wrs else 0
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

    # Overfitting Layer 1: IS-OOS Gap
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

    # Overfitting Layer 3: Parameter Sensitivity (Fragility)
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

    # Survivor Score
    of_penalty = 1.0 - min(overfitting_score, 1.0)
    dd_factor = 1.0 / (1.0 + avg_oos_dd / 100)
    trade_factor = min(avg_oos_trades / max(of_config.get("min_trades_per_fold", 10), 1), 1.0)
    # Fragility: inverse penalty that stays non-negative.
    # f=0→1.0, f=0.5→0.67, f=1.0→0.50, f=2.0→0.33, f=5.0→0.17
    # A config with positive Sharpe should never have negative survivor.
    fragility_penalty = 1.0 / (1.0 + fragility)
    survivor_score = avg_oos_sharpe * oos_consistency * of_penalty * dd_factor * trade_factor * fragility_penalty

    # Rejection check (only IS-OOS gap and consistency — fragility is now a penalty)
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


# Grid Search

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
    total_evals = 0

    for parent in top_candidates:
        if parent.rejected:
            continue
        base = dict(parent.params)
        # Remove trailing/flip/leverage params to re-sweep them at fine granularity
        base.pop("trailing_stop_atr", None)
        base.pop("score_flip_delay_hrs", None)
        base.pop("leverage", None)

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
            total_evals += 1

            if total_evals % 5000 == 0:
                passed = sum(1 for r in results if not r.rejected)
                best = max((r.survivor_score for r in results), default=0)
                log(f"    [{symbol}] Fine: {total_evals} evaluated, {passed} passed, "
                    f"best survivor={best:.3f}")

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
                    # Floor: leverage >= 1.0, all other params >= 0.01
                    floor = 1.0 if key == "leverage" else 0.01
                    params[key] = max(floor, round(original * (1 + delta), 4))

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


# Flash Trade fee helper (used by evaluate_on_fold for compounding)

def flash_trade_round_trip_cost(leverage: float, hold_hrs: float) -> float:
    """Flash Trade round-trip cost as a % of margin.

    Notional = margin * leverage.
    Open fee = notional * 0.06% = margin * leverage * 0.06%
    Close fee = notional * 0.06% = margin * leverage * 0.06%
    Borrow = notional * 0.0042% * hold_hrs = margin * leverage * 0.0042% * hold_hrs
    As % of margin: leverage * (0.06 + 0.06 + 0.0042 * hold_hrs)
    """
    fees = FLASH_TRADE_FEES
    notional_fee_pct = fees["open_fee_pct"] + fees["close_fee_pct"] + fees["hourly_borrow_pct"] * hold_hrs
    return leverage * notional_fee_pct


# BB Mean Reversion Strategy (Fast Evaluator)

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


# Experiment Runner

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


# Post-Run Auto-Validation

def auto_validate_top_candidates(all_results: Dict[str, List[CandidateResult]],
                                  output_dir: str, top_n: int = 3) -> str:
    """Validate top night shift candidates through FutureBlindSimulator.

    Imports and runs validate_night_shift logic synchronously within the night
    shift process. Only validates candidates that beat the production baseline.
    """
    log(f"\n── Phase 7: Auto-Validation (FutureBlindSimulator) ──")

    import asyncio
    from research.simulation.future_blind_simulator import FutureBlindSimulator

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
                        avg_loss = abs(np.mean([p for p in pnls if p < 0])) or 0.001
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
                    from research.simulation.data_window import DataWindow
                    from research.simulation.run_backtest_r2 import MultiTFStrategy
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
                            avg_loss = abs(np.mean([p for p in pnls if p < 0])) or 0.001
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


# Regime Analysis

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


# Data Loading

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


# Report Generation

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

    # Market State
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

    # Production Baseline
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

    # Top 10 Candidates
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

    # Overfitting Warnings
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

    # Per-Symbol Fold Detail
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

    # Action Items
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


# Leverage Optimization Mode

@dataclass
class LeverageCandidate:
    """Result for one leveraged candidate."""
    params: Dict
    final_capital: float
    total_return_pct: float
    max_drawdown_pct: float
    calmar_ratio: float
    oos_sharpe: float
    win_rate: float
    consistency: float
    total_trades: int
    avg_hold_hrs: float
    liquidations: int
    total_fees: float
    fold_details: List[Dict] = field(default_factory=list)
    rejected: bool = False
    rejection_reason: str = ""


def evaluate_leverage_candidate(df: pd.DataFrame, folds: List[Fold],
                                 params: Dict, symbol: str,
                                 position_pct: float = 0.20,
                                 initial_capital: float = 100.0) -> LeverageCandidate:
    """Evaluate one leveraged candidate via full WFA with compounding + Flash Trade fees.

    Scoring: Calmar ratio = total_return / max_drawdown.
    Capital carries across folds for realistic compounding.
    """
    leverage = params.get("leverage", 1.0)
    capital = initial_capital
    global_peak = capital
    global_max_dd = 0.0
    total_wins = 0
    total_trades = 0
    total_liqs = 0
    total_fees = 0.0
    all_net_pnls = []
    all_hold_hrs = []
    fold_details = []
    positive_folds = 0

    for fold in folds:
        fold_start = capital
        test_df = df.iloc[fold.test_start_idx:fold.test_end_idx]
        if len(test_df) < 10:
            continue

        # Simulate trades on this fold
        trips = simulate_trades(test_df, params)
        if not trips:
            fold_details.append({
                "fold": fold.fold_num,
                "start_capital": round(fold_start, 2),
                "end_capital": round(capital, 2),
                "return_pct": 0.0,
                "trades": 0,
            })
            continue

        fold_peak = capital
        fold_dd = 0.0
        fold_wins = 0
        fold_fees = 0.0
        fold_net_pnls = []
        fold_hold = []

        for t in trips:
            hold_hrs = t["hold_hrs"]
            fold_hold.append(hold_hrs)
            all_hold_hrs.append(hold_hrs)

            if t.get("liquidated", False):
                position = capital * position_pct
                capital -= position
                total_liqs += 1
                fold_net_pnls.append(-100.0)
                all_net_pnls.append(-100.0)
            else:
                raw_pnl = t["pnl_pct"]  # already includes leverage
                fee_pct = flash_trade_round_trip_cost(leverage, hold_hrs)
                net_pnl = raw_pnl - fee_pct

                position = capital * position_pct
                capital += position * (net_pnl / 100.0)
                fee_sol = position * (fee_pct / 100.0)
                total_fees += fee_sol
                fold_fees += fee_sol

                fold_net_pnls.append(net_pnl)
                all_net_pnls.append(net_pnl)
                if net_pnl > 0:
                    fold_wins += 1
                    total_wins += 1

            total_trades += 1

            if capital > global_peak:
                global_peak = capital
            if capital > fold_peak:
                fold_peak = capital
            if global_peak > 0:
                dd = (global_peak - capital) / global_peak * 100
                global_max_dd = max(global_max_dd, dd)
            if fold_peak > 0:
                dd = (fold_peak - capital) / fold_peak * 100
                fold_dd = max(fold_dd, dd)

        fold_return = (capital - fold_start) / fold_start * 100 if fold_start > 0 else 0
        if fold_return > 0:
            positive_folds += 1

        fold_details.append({
            "fold": fold.fold_num,
            "start_capital": round(fold_start, 2),
            "end_capital": round(capital, 2),
            "return_pct": round(fold_return, 2),
            "trades": len(trips),
            "wins": fold_wins,
            "max_dd_pct": round(fold_dd, 2),
            "fees_sol": round(fold_fees, 4),
        })

    # Aggregate metrics
    total_return = (capital - initial_capital) / initial_capital * 100 if initial_capital > 0 else 0
    consistency = positive_folds / len(fold_details) if fold_details else 0
    win_rate = total_wins / total_trades if total_trades > 0 else 0
    avg_hold = float(np.mean(all_hold_hrs)) if all_hold_hrs else 0

    # Sharpe from per-trade net PnLs
    if len(all_net_pnls) > 1 and np.std(all_net_pnls) > 0:
        total_hours = sum(fold.test_hours for fold in folds)
        trades_per_year = (len(all_net_pnls) / max(total_hours, 1)) * 8760
        oos_sharpe = float(np.mean(all_net_pnls) / np.std(all_net_pnls) * np.sqrt(max(trades_per_year, 0.1)))
    else:
        oos_sharpe = 0.0

    # Calmar ratio: return / max DD
    calmar = total_return / global_max_dd if global_max_dd > 0 else total_return if total_return > 0 else 0

    # Rejection
    rejected = False
    rejection_reason = ""
    if consistency < 0.50:
        rejected = True
        rejection_reason = f"consistency={consistency:.0%} < 50%"
    if total_liqs > 3:
        rejected = True
        rejection_reason += f" liquidations={total_liqs} > 3"

    return LeverageCandidate(
        params=dict(params),
        final_capital=round(capital, 4),
        total_return_pct=round(total_return, 2),
        max_drawdown_pct=round(global_max_dd, 2),
        calmar_ratio=round(calmar, 4),
        oos_sharpe=round(oos_sharpe, 4),
        win_rate=round(win_rate, 4),
        consistency=round(consistency, 4),
        total_trades=total_trades,
        avg_hold_hrs=round(avg_hold, 1),
        liquidations=total_liqs,
        total_fees=round(total_fees, 2),
        fold_details=fold_details,
        rejected=rejected,
        rejection_reason=rejection_reason.strip(),
    )


def run_leverage_optimization(config: Dict, config_path: Optional[str] = None):
    """Leverage optimization night shift: single-stage grid + Darwinian refinement.

    Optimizes exit/sizing params specifically for leveraged execution.
    Scoring: Calmar ratio (compounded return / max drawdown).
    All candidates evaluated with Flash Trade fee model + compounding.
    """
    start_time = time.time()
    lev_config = config.get("leverage_grid", {})
    symbols = config.get("symbols", ["SOL/USDT"])
    position_pct = config.get("position_pct", 0.20)
    initial_capital = config.get("initial_capital", 100.0)
    wfa = config.get("wfa", {})
    num_folds = wfa.get("num_folds", 9)
    test_days = wfa.get("test_fold_days", 36)
    output_dir = config.get("output_dir", "data/night_results")

    log(f"{'='*70}")
    log(f"LEVERAGE OPTIMIZATION NIGHT SHIFT")
    log(f"Mode:       Calmar-optimized leverage sweep")
    log(f"Symbols:    {', '.join(symbols)}")
    log(f"Position:   {position_pct:.0%}")
    log(f"Capital:    {initial_capital} SOL")
    log(f"Fees:       0.06% open + 0.06% close + 0.0042%/hr borrow")
    log(f"{'='*70}")

    # Phase 1: Data
    log(f"\n── Phase 1: Data ──")
    fetch = config.get("schedule", {}).get("fetch_fresh_data", False)
    if fetch:
        fetch_fresh_data(symbols)
    else:
        log(f"Using cached data")
    dfs = load_data(symbols)
    if not dfs:
        log(f"FATAL: No data loaded. Exiting.")
        sys.exit(1)

    # Phase 2: Folds
    log(f"\n── Phase 2: WFA Folds ──")
    all_results = {}
    for symbol, df in dfs.items():
        folds = create_folds(len(df), num_folds, test_days)
        log(f"  {symbol}: {len(df)} candles, {len(folds)} folds")

        # Phase 3: Grid Search
        grid_keys = list(lev_config.keys())
        grid_values = [lev_config[k] for k in grid_keys]
        combos = list(product(*grid_values))
        log(f"\n── Phase 3: Leverage Grid ({len(combos)} candidates) ──")

        results = []
        for i, combo in enumerate(combos):
            params = dict(zip(grid_keys, combo))
            lc = evaluate_leverage_candidate(
                df, folds, params, symbol,
                position_pct=position_pct,
                initial_capital=initial_capital,
            )
            results.append(lc)

            if (i + 1) % 2000 == 0:
                passed = sum(1 for r in results if not r.rejected)
                best_calmar = max((r.calmar_ratio for r in results if not r.rejected), default=0)
                best_return = max((r.total_return_pct for r in results if not r.rejected), default=0)
                log(f"  [{i+1}/{len(combos)}] passed={passed} best_calmar={best_calmar:.2f} "
                    f"best_return={best_return:+.1f}%")

        log(f"  Grid complete: {len(results)} candidates, "
            f"{sum(1 for r in results if not r.rejected)} passed filters")

        # Phase 4: Darwinian refinement
        darwin_cfg = config.get("darwinian", {"generations": 3, "population": 50})
        generations = darwin_cfg.get("generations", 3)
        pop_size = darwin_cfg.get("population", 50)
        perturb_range = (0.05, 0.15)

        log(f"\n── Phase 4: Darwinian Refinement ({generations} gens, pop={pop_size}) ──")

        current_gen = sorted(
            [r for r in results if not r.rejected],
            key=lambda r: r.calmar_ratio,
            reverse=True,
        )[:pop_size]

        if not current_gen:
            log(f"  No survivors for Darwinian evolution")
        else:
            all_survivors = list(current_gen)

            for gen in range(generations):
                offspring = []
                for parent in current_gen:
                    for _ in range(3):
                        params = dict(parent.params)
                        numeric_keys = [k for k, v in params.items()
                                       if isinstance(v, (int, float)) and k != "min_alignment"]
                        if not numeric_keys:
                            continue
                        key = random.choice(numeric_keys)
                        delta = random.uniform(*perturb_range) * random.choice([-1, 1])
                        original = params[key]
                        if isinstance(original, int):
                            params[key] = max(1, int(original * (1 + delta)))
                        else:
                            floor = 1.0 if key == "leverage" else 0.01
                            params[key] = max(floor, round(original * (1 + delta), 4))

                        lc = evaluate_leverage_candidate(
                            df, folds, params, symbol,
                            position_pct=position_pct,
                            initial_capital=initial_capital,
                        )
                        offspring.append(lc)

                combined = current_gen + offspring
                combined.sort(key=lambda r: r.calmar_ratio, reverse=True)
                current_gen = combined[:pop_size]
                all_survivors.extend(current_gen)

                best_calmar = current_gen[0].calmar_ratio
                best_return = current_gen[0].total_return_pct
                log(f"  Gen {gen+1}/{generations}: {len(offspring)} offspring, "
                    f"best_calmar={best_calmar:.2f} best_return={best_return:+.1f}%")

            # Deduplicate
            seen = set()
            unique = []
            for r in sorted(all_survivors, key=lambda r: r.calmar_ratio, reverse=True):
                key = tuple(sorted(r.params.items()))
                if key not in seen:
                    seen.add(key)
                    unique.append(r)
            results.extend(unique[:pop_size * 2])

        all_results[symbol] = results

    # Phase 5: Report
    log(f"\n── Phase 5: Leverage Report ──")
    run_time = time.time() - start_time
    report_path = _generate_leverage_report(all_results, dfs, run_time, output_dir, position_pct)

    # Phase 6: Summary
    final_time = time.time() - start_time
    log(f"\n{'='*70}")
    log(f"LEVERAGE OPTIMIZATION COMPLETE — {final_time:.0f}s")
    log(f"{'='*70}")

    for symbol, results in all_results.items():
        non_rejected = [r for r in results if not r.rejected]
        if not non_rejected:
            log(f"  {symbol}: NO candidates passed filters")
            continue

        # Best per leverage level
        for lev in sorted(set(r.params.get("leverage", 1) for r in non_rejected)):
            lev_results = [r for r in non_rejected if r.params.get("leverage") == lev]
            best = max(lev_results, key=lambda r: r.calmar_ratio)
            p = best.params
            log(f"  {symbol} {lev:.0f}x: calmar={best.calmar_ratio:.2f} "
                f"ret={best.total_return_pct:+.1f}% DD={best.max_drawdown_pct:.1f}% "
                f"WR={best.win_rate:.0%} cons={best.consistency:.0%} "
                f"sl={p.get('stop_loss_atr')} tr={p.get('trailing_stop_atr')} "
                f"tp={p.get('take_profit_atr')} th={p.get('signal_threshold')} "
                f"align={p.get('min_alignment')} liqs={best.liquidations}")

    log(f"  Report: {report_path}")

    # Save full results
    date_dir = os.path.join(output_dir, datetime.now(timezone.utc).strftime("%Y-%m-%d"))
    os.makedirs(date_dir, exist_ok=True)
    json_path = os.path.join(date_dir, "leverage_optimization.json")
    save_data = {}
    for sym, results in all_results.items():
        results_sorted = sorted(results, key=lambda r: r.calmar_ratio, reverse=True)
        save_data[sym] = [
            {
                "params": r.params,
                "final_capital": r.final_capital,
                "total_return_pct": r.total_return_pct,
                "max_drawdown_pct": r.max_drawdown_pct,
                "calmar_ratio": r.calmar_ratio,
                "oos_sharpe": r.oos_sharpe,
                "win_rate": r.win_rate,
                "consistency": r.consistency,
                "total_trades": r.total_trades,
                "avg_hold_hrs": r.avg_hold_hrs,
                "liquidations": r.liquidations,
                "total_fees": r.total_fees,
                "rejected": r.rejected,
                "rejection_reason": r.rejection_reason,
                "folds": r.fold_details,
            }
            for r in results_sorted[:100]
        ]
    with open(json_path, "w") as f:
        json.dump({
            "run_at": datetime.now(timezone.utc).isoformat(),
            "mode": "leverage_optimize",
            "config": config,
            "results": save_data,
        }, f, indent=2, default=str)
    log(f"  Full results: {json_path}")


def _generate_leverage_report(all_results: Dict[str, List[LeverageCandidate]],
                               dfs: Dict, run_time: float,
                               output_dir: str, position_pct: float) -> str:
    """Generate markdown report for leverage optimization run."""
    now = datetime.now(timezone.utc)
    date_str = now.strftime("%Y-%m-%d")
    report_path = os.path.join(output_dir, date_str, "leverage_report.md")
    os.makedirs(os.path.dirname(report_path), exist_ok=True)

    lines = []
    w = lines.append

    w(f"# Leverage Optimization Report — {date_str}")
    w(f"")
    w(f"**Runtime:** {run_time:.0f}s | **Scoring:** Calmar ratio (return/DD)")
    w(f"**Position sizing:** {position_pct:.0%} | **Fees:** Flash Trade (0.06%+0.06%+0.0042%/hr)")
    w(f"")

    for symbol, results in all_results.items():
        non_rejected = sorted(
            [r for r in results if not r.rejected],
            key=lambda r: r.calmar_ratio,
            reverse=True,
        )

        w(f"## {symbol}")
        w(f"")
        w(f"**Total candidates:** {len(results)} | **Passed:** {len(non_rejected)}")
        w(f"")

        # Top 10 overall
        w(f"### Top 10 by Calmar Ratio")
        w(f"")
        w(f"| # | Lev | SL | Trail | TP | Thresh | Align | Return | DD | Calmar | Sharpe | WR | Cons | Trades | Liqs |")
        w(f"|---|-----|----|-------|----|--------|-------|--------|----|--------|--------|----|------|--------|------|")
        for i, r in enumerate(non_rejected[:10]):
            p = r.params
            w(f"| {i+1} | {p.get('leverage', 1):.0f}x | {p.get('stop_loss_atr', '-'):.1f} | "
              f"{p.get('trailing_stop_atr', '-'):.2f} | {p.get('take_profit_atr', '-'):.1f} | "
              f"{p.get('signal_threshold', '-'):.2f} | {p.get('min_alignment', '-')} | "
              f"{r.total_return_pct:+.1f}% | {r.max_drawdown_pct:.1f}% | "
              f"{r.calmar_ratio:.2f} | {r.oos_sharpe:+.2f} | {r.win_rate:.0%} | "
              f"{r.consistency:.0%} | {r.total_trades} | {r.liquidations} |")
        w(f"")

        # Best per leverage level
        w(f"### Best Config per Leverage Level")
        w(f"")
        for lev in sorted(set(r.params.get("leverage", 1) for r in non_rejected)):
            lev_results = [r for r in non_rejected if r.params.get("leverage") == lev]
            best = max(lev_results, key=lambda r: r.calmar_ratio)
            p = best.params
            w(f"**{lev:.0f}x:** sl={p.get('stop_loss_atr')}, trail={p.get('trailing_stop_atr')}, "
              f"tp={p.get('take_profit_atr')}, thresh={p.get('signal_threshold')}, "
              f"align={p.get('min_alignment')}, hold={p.get('max_hold_hours')}h, decay={p.get('time_decay_hours')}h")
            w(f"- Return: {best.total_return_pct:+.1f}% | DD: {best.max_drawdown_pct:.1f}% | "
              f"Calmar: {best.calmar_ratio:.2f} | Sharpe: {best.oos_sharpe:+.2f}")
            w(f"- WR: {best.win_rate:.0%} | Consistency: {best.consistency:.0%} | "
              f"Trades: {best.total_trades} | Liquidations: {best.liquidations} | Fees: {best.total_fees:.2f} SOL")
            w(f"")

            # Per-fold detail
            w(f"| Fold | Start | End | Return | Trades | DD | Fees |")
            w(f"|------|-------|-----|--------|--------|----|------|")
            for fd in best.fold_details:
                w(f"| {fd['fold']} | {fd['start_capital']:.2f} | {fd['end_capital']:.2f} | "
                  f"{fd['return_pct']:+.2f}% | {fd['trades']} | {fd['max_dd_pct']:.1f}% | "
                  f"{fd.get('fees_sol', 0):.2f} |")
            w(f"")

    report_text = "\n".join(lines)
    with open(report_path, "w") as f:
        f.write(report_text)
    return report_path


# Strategy Exploration (Plugin-Based)

def run_strategy_exploration(
    dfs: Dict[str, pd.DataFrame],
    folds: List[Fold],
    config: Dict,
    all_results: Dict[str, list],
) -> Dict[str, list]:
    """
    Phase 2/4d: Explore alternative strategies via plugins.

    1. LLM selects 3-5 strategies from the library
    2. Each gets its plugin + param grid evaluated via WFA + Calmar scoring
    3. Results compared against the Survivor baseline

    Returns per-symbol results from all plugins.
    """
    from research.strategy_plugins import PLUGINS
    from research.strategy_plugins.base import simulate_plugin_trades
    from research.orchestration.llm_strategy_selector import select_strategies

    use_llm = config.get("use_llm", False)
    n_select = config.get("n_strategies", 3)
    position_pct = config.get("position_pct", 0.20)
    initial_capital = config.get("initial_capital", 100.0)
    max_candidates_per_plugin = config.get("max_candidates", 500)

    # Select strategies
    selections = select_strategies(use_llm=use_llm, n_select=n_select)

    if not selections:
        log("  No strategies selected, skipping exploration")
        return {}

    exploration_results = {}

    for symbol, df in dfs.items():
        sym_results = []
        log(f"  {symbol}: testing {len(selections)} strategies")

        for sel in selections:
            sid = sel["id"]
            reason = sel.get("reason", "")
            log(f"    [{sid}] {reason}")

            if sid not in PLUGINS:
                log(f"      Plugin {sid} not found, skipping")
                continue

            plugin = PLUGINS[sid]()

            # Compute plugin-specific indicators
            try:
                df_plugin = plugin.compute_indicators(df.copy())
            except Exception as e:
                log(f"      Indicator computation failed: {e}")
                continue

            # Generate param grid and cap size
            grid = plugin.param_grid()
            grid_keys = list(grid.keys())
            grid_values = [grid[k] for k in grid_keys]
            combos = list(product(*grid_values))

            # Cap candidates
            if len(combos) > max_candidates_per_plugin:
                rng = np.random.default_rng(42)
                indices = rng.choice(len(combos), size=max_candidates_per_plugin, replace=False)
                combos = [combos[i] for i in sorted(indices)]

            log(f"      Grid: {len(combos)} candidates")

            # Evaluate each candidate
            for i, combo in enumerate(combos):
                params = dict(zip(grid_keys, combo))

                # Run simulation via plugin
                try:
                    trips = simulate_plugin_trades(df_plugin, plugin, params,
                                                   leverage=params.get("leverage", 1.0))
                except Exception as e:
                    continue

                if len(trips) < 5:
                    continue

                # Score with Calmar using compounding
                leverage = params.get("leverage", 1.0)
                capital = initial_capital
                peak = capital
                max_dd = 0.0
                total_wins = 0
                net_pnls = []

                for t in trips:
                    hold_hrs = t["hold_hrs"]
                    if t.get("liquidated", False):
                        position = capital * position_pct
                        capital -= position
                        net_pnls.append(-100.0)
                    else:
                        raw_pnl = t["pnl_pct"]
                        fee_pct = flash_trade_round_trip_cost(leverage, hold_hrs)
                        net_pnl = raw_pnl - fee_pct
                        position = capital * position_pct
                        capital += position * (net_pnl / 100.0)
                        net_pnls.append(net_pnl)
                        if net_pnl > 0:
                            total_wins += 1

                    if capital > peak:
                        peak = capital
                    if peak > 0:
                        dd = (peak - capital) / peak * 100
                        max_dd = max(max_dd, dd)

                total_return = (capital - initial_capital) / initial_capital * 100
                calmar = total_return / max_dd if max_dd > 0 else (total_return if total_return > 0 else 0)
                wr = total_wins / len(trips) if trips else 0

                if calmar > 0 and total_return > 0:
                    lc = LeverageCandidate(
                        params={**params, "strategy": sid, "strategy_name": plugin.name},
                        final_capital=round(capital, 4),
                        total_return_pct=round(total_return, 2),
                        max_drawdown_pct=round(max_dd, 2),
                        calmar_ratio=round(calmar, 4),
                        oos_sharpe=0.0,
                        win_rate=round(wr, 4),
                        consistency=0.0,
                        total_trades=len(trips),
                        avg_hold_hrs=round(float(np.mean([t["hold_hrs"] for t in trips])), 1),
                        liquidations=sum(1 for t in trips if t.get("liquidated", False)),
                        total_fees=0.0,
                    )
                    sym_results.append(lc)

                if (i + 1) % 200 == 0:
                    best_calmar = max((r.calmar_ratio for r in sym_results), default=0)
                    log(f"        [{i+1}/{len(combos)}] best_calmar={best_calmar:.2f}")

            # Summary for this plugin
            plugin_results = [r for r in sym_results
                              if r.params.get("strategy") == sid]
            if plugin_results:
                best = max(plugin_results, key=lambda r: r.calmar_ratio)
                log(f"      {sid} best: calmar={best.calmar_ratio:.2f} "
                    f"ret={best.total_return_pct:+.1f}% DD={best.max_drawdown_pct:.1f}% "
                    f"trades={best.total_trades}")

        if sym_results:
            exploration_results[symbol] = sym_results

    # Cross-plugin comparison
    for symbol, results in exploration_results.items():
        best = max(results, key=lambda r: r.calmar_ratio)
        log(f"  {symbol} exploration winner: {best.params.get('strategy')} "
            f"calmar={best.calmar_ratio:.2f} ret={best.total_return_pct:+.1f}%")

    return exploration_results


# Robustness Testing Integration

def run_robustness_phase(
    dfs: Dict[str, pd.DataFrame],
    all_results: Dict[str, list],
    config: Dict,
):
    """
    Phase 3: Run robustness testing on top candidates from each phase.

    Monte Carlo DD + CPCV + PBO for the top 3 candidates.
    """
    from research.validation.robustness import (
        run_robustness_analysis, extract_net_pnls,
    )

    robustness_config = config.get("robustness", {})
    if not robustness_config.get("enabled", False):
        log("  Robustness testing disabled")
        return

    n_candidates = robustness_config.get("top_n", 3)
    n_mc = robustness_config.get("mc_simulations", 10000)
    n_cpcv_folds = robustness_config.get("cpcv_folds", 10)
    n_cpcv_test = robustness_config.get("cpcv_test_folds", 3)
    n_cpcv_variants = robustness_config.get("cpcv_variants", 20)
    position_pct = robustness_config.get("position_pct", 0.20)

    log(f"  Testing top {n_candidates} candidates per symbol")

    date_str = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    output_dir = os.path.join(RESULTS_DIR, date_str, "robustness")

    for symbol, results in all_results.items():
        # Get top candidates by Calmar (if leverage results) or survivor
        non_rejected = [r for r in results if not getattr(r, 'rejected', False)]
        if not non_rejected:
            continue

        # Sort by calmar_ratio if available, otherwise survivor_score
        def _score(r):
            if hasattr(r, 'calmar_ratio'):
                return r.calmar_ratio
            return getattr(r, 'survivor_score', 0)

        top = sorted(non_rejected, key=_score, reverse=True)[:n_candidates]

        df = dfs.get(symbol)
        if df is None:
            continue

        for i, cand in enumerate(top):
            params = dict(cand.params)
            leverage = params.get("leverage", 1.0)
            strategy_label = params.get("strategy", params.get("strategy_name", "survivor"))

            log(f"    {symbol} candidate #{i+1}: {strategy_label} "
                f"leverage={leverage:.0f}x calmar={_score(cand):.2f}")

            try:
                result = run_robustness_analysis(
                    symbol=symbol,
                    params=params,
                    leverage=leverage,
                    position_pct=position_pct,
                    n_mc_simulations=n_mc,
                    n_cpcv_folds=n_cpcv_folds,
                    n_cpcv_test_folds=n_cpcv_test,
                    n_cpcv_variants=n_cpcv_variants,
                    output_dir=output_dir,
                )
                verdict = result.get("verdict", {})
                log(f"      MC DD p95={result.get('monte_carlo', {}).get('dd_p95', 0):.1f}% | "
                    f"PBO={result.get('cpcv', {}).get('pbo', 0):.1%} | "
                    f"Verdict: {verdict.get('overall', 'N/A')}")
            except Exception as e:
                log(f"      Robustness failed: {e}")


# Main Night Shift

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

    # Phase 1: Data
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
        sys.exit(1)

    # Phase 2: WFA Folds
    log(f"\n── Phase 2: Expanding-Window WFA ──")
    # Use minimum data length across symbols for consistent folds
    min_bars = min(len(df) for df in dfs.values())
    folds = create_folds(min_bars, WFA_CONFIG["num_folds"], WFA_CONFIG["test_fold_days"])
    log(f"Created {len(folds)} folds from {min_bars} bars")
    for f in folds:
        log(f"  Fold {f.fold_num}: train=[{f.train_start_idx}:{f.train_end_idx}] "
            f"({f.train_hours}h) test=[{f.test_start_idx}:{f.test_end_idx}] ({f.test_hours}h)")

    # Phase 2b: Evaluate Production Baseline on all folds
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

    # Phase 3: Grid Search
    log(f"\n── Phase 3: Coarse Grid Search ──")
    # NOTE: all_results already has production baselines from Phase 2b — extend, don't overwrite
    for symbol in dfs:
        if symbol not in all_results:
            all_results[symbol] = []
        results = coarse_grid_search(dfs[symbol], folds, symbol, OVERFITTING_CONFIG)
        all_results[symbol].extend(results)

    # Phase 3b: Fine Refinement
    log(f"\n── Phase 3b: Fine Refinement ──")
    for symbol, results in all_results.items():
        top_n = sorted(results, key=lambda r: r.survivor_score, reverse=True)[:100]
        fine_results = fine_refinement(dfs[symbol], folds, symbol, top_n, OVERFITTING_CONFIG)
        all_results[symbol].extend(fine_results)

    # Phase 4: Darwinian
    log(f"\n── Phase 4: Darwinian Evolution ──")
    for symbol, results in all_results.items():
        survivors = darwinian_evolution(
            dfs[symbol], folds, symbol, results,
            OVERFITTING_CONFIG, DARWINIAN_CONFIG,
        )
        all_results[symbol].extend(survivors)

    # Phase 4b: BB Mean Reversion Grid Search
    log(f"\n── Phase 4b: BB Mean Reversion ──")
    for symbol in dfs:
        bb_results = run_bb_grid_search(dfs[symbol], folds, symbol, OVERFITTING_CONFIG)
        if symbol not in all_results:
            all_results[symbol] = []
        all_results[symbol].extend(bb_results)

    # Phase 4c: Custom Experiments
    experiments = config.get("experiments", [])
    if experiments:
        log(f"\n── Phase 4c: Custom Experiments ({len(experiments)}) ──")
        for symbol in dfs:
            exp_results = run_experiments(dfs[symbol], folds, symbol, experiments, OVERFITTING_CONFIG)
            all_results[symbol].extend(exp_results)
    else:
        log(f"\n── Phase 4c: No experiments configured (add to night_config.json) ──")

    # Phase 4d: Strategy Exploration (plugin-based)
    exploration_config = config.get("strategy_exploration", {})
    exploration_results = {}
    if exploration_config.get("enabled", False):
        log(f"\n── Phase 4d: Strategy Exploration ──")
        exploration_results = run_strategy_exploration(
            dfs, folds, exploration_config, all_results,
        )
        # Merge into all_results
        for sym, results in exploration_results.items():
            if sym not in all_results:
                all_results[sym] = []
            all_results[sym].extend(results)
    else:
        log(f"\n── Phase 4d: Strategy exploration disabled (enable in night_config.json) ──")

    # Phase 5: Regime Analysis
    log(f"\n── Phase 5: Regime Analysis ──")
    regime = regime_analysis(dfs)

    # Phase 6: Report
    log(f"\n── Phase 6: Morning Report ──")
    run_time = time.time() - start_time
    report_path = generate_report(all_results, regime, folds, run_time, RESULTS_DIR)
    log(f"Report saved to {report_path}")

    # Phase 7: Auto-Validation
    val_top = config.get("validation", {}).get("top_candidates", 3)
    val_path = auto_validate_top_candidates(all_results, RESULTS_DIR, top_n=val_top)

    # Phase 7b: Robustness Testing (Monte Carlo + CPCV + PBO)
    robustness_config = config.get("robustness", {})
    if robustness_config.get("enabled", False):
        log(f"\n── Phase 7b: Robustness Testing ──")
        run_robustness_phase(dfs, all_results, config)
    else:
        log(f"\n── Phase 7b: Robustness testing disabled (enable in night_config.json) ──")

    # Summary
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

    # Phase 8: Discrepancy Detection (self-awareness)
    log(f"\n── Phase 8: Discrepancy Detection ──")
    try:
        from research.validation.discrepancy_detector import detect_discrepancies, update_flag_history, generate_recommendation
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
                val_data = json.load(vf)
            val_entries = val_data.get("results", val_data) if isinstance(val_data, dict) else val_data
            for entry in val_entries:
                if isinstance(entry, dict) and "symbol" in entry:
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


# CLI

def bridge_mode():
    """Bridge mode: read BridgeRequest JSON from stdin, write BridgeResponse JSON to stdout.

    This is the interface the Rust swarm uses via bridge.rs.
    Input:  {"symbol": "SOL/USDT", "config": {"data_dir": "/path/to/data/ohlcv", "params": {...}}}
    Output: {"strategy": "...", "yield_estimate": ..., "confidence": ..., "params": {...},
             "folds_validated": ..., "consistency": ...}
    """
    try:
        input_data = json.loads(sys.stdin.read())
    except (json.JSONDecodeError, EOFError) as e:
        json.dump({"error": f"Invalid JSON input: {e}"}, sys.stdout)
        sys.stdout.flush()
        sys.exit(1)

    symbol = input_data.get("symbol", "SOL/USDT")
    config = input_data.get("config", {})

    # Resolve data directory:
    # 1. Explicit data_dir in config
    # 2. Relative to the binary's location (sys.executable for PyInstaller)
    # 3. Relative to CWD
    data_dir = config.get("data_dir")
    if not data_dir:
        # In PyInstaller onefile mode, sys.executable points to the actual binary.
        binary_dir = os.path.dirname(os.path.abspath(sys.executable))
        for candidate in [binary_dir, os.getcwd()]:
            candidate_data = os.path.join(candidate, "data", "ohlcv")
            if os.path.isdir(candidate_data):
                data_dir = candidate_data
                break
    if not data_dir:
        data_dir = DATA_DIR

    # Load data for the requested symbol.
    safe = symbol.replace("/", "_")
    path = os.path.join(data_dir, f"{safe}_1h.parquet")
    if not os.path.exists(path):
        json.dump({
            "error": f"No data for {symbol} at {path}",
            "strategy": "none",
            "yield_estimate": 0.0,
            "confidence": 0.0,
            "params": {},
            "folds_validated": 0,
            "consistency": 0.0,
        }, sys.stdout)
        sys.stdout.flush()
        sys.exit(0)

    try:
        df = pd.read_parquet(path)
        df = compute_indicators(df)

        # Create WFA folds.
        total_bars = len(df)
        num_folds = config.get("folds", WFA_CONFIG["num_folds"])
        test_days = config.get("test_fold_days", WFA_CONFIG["test_fold_days"])
        folds = create_folds(total_bars, num_folds, test_days)

        # Use the config params if provided, otherwise use production baseline.
        params = {**PRODUCTION_CONFIG, **config.get("params", {})}

        # Evaluate on all folds.
        result = evaluate_candidate(
            df, folds, params, symbol,
            OVERFITTING_CONFIG,
            compute_fragility=True,
        )

        response = {
            "strategy": config.get("strategy", params.get("strategy", "multi_tf")),
            "yield_estimate": round(result.oos_pnl, 2),
            "confidence": round(min(result.oos_consistency * (1.0 - result.overfitting_score), 1.0), 4),
            "params": {k: v for k, v in result.params.items() if k != "min_alignment"},
            "folds_validated": len(folds),
            "consistency": round(result.oos_consistency, 4),
        }

        json.dump(response, sys.stdout)
        sys.stdout.flush()
        sys.exit(0)

    except Exception as e:
        json.dump({
            "error": f"Evaluation error: {e}",
            "strategy": "none",
            "yield_estimate": 0.0,
            "confidence": 0.0,
            "params": {},
            "folds_validated": 0,
            "consistency": 0.0,
        }, sys.stdout)
        sys.stdout.flush()
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description="Night Shift: Zero-token autonomous strategy optimization")
    parser.add_argument("--config", type=str, default=None, help="Path to night_config.json")
    parser.add_argument("--skip-fetch", action="store_true", help="Use cached data, skip Binance fetch")
    parser.add_argument("--symbols", nargs="+", default=None, help="Symbols to optimize (default: all 4)")
    parser.add_argument("--folds", type=int, default=None, help="Number of WFA folds")
    parser.add_argument("--test-days", type=int, default=None, help="Test fold duration in days")
    parser.add_argument("--bridge-mode", action="store_true",
                        help="Bridge mode: read JSON from stdin, write JSON to stdout (for Rust bridge)")
    args = parser.parse_args()

    # Bridge mode: typed JSON interface for the Rust swarm.
    if args.bridge_mode:
        bridge_mode()
        return

    # Load config to check mode
    config = load_config(args.config)

    # Leverage optimization mode
    if config.get("mode") == "leverage_optimize":
        run_leverage_optimization(config, config_path=args.config)
        return

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
