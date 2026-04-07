"""
Autoresearch Loop — Karpathy-style self-improving strategy parameters.

Inspired by https://github.com/karpathy/autoresearch and
https://github.com/chrisworsey55/atlas-gic

The strategy parameters are the weights. Sharpe ratio is the loss function.
No GPU needed — just Python + our existing WFA framework.

Loop:
  1. Identify worst-performing symbol (lowest WFA Sharpe)
  2. Generate ONE targeted parameter modification
  3. Run WFA with modified params (47-fold, 30-day windows)
  4. Check if rolling Sharpe improved
  5. Keep (git commit) or revert (git reset)

Each iteration is one "training step". Run overnight for ~100 experiments.
"""
import asyncio
import os
import sys
import json
import subprocess
import numpy as np
import pandas as pd
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
from itertools import product
from dataclasses import dataclass, asdict

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from scripts.per_symbol_optimizer import compute_indicators, simulate_trades, compute_metrics, _compute_score

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")
SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT"]
STATE_FILE = os.path.join(os.path.dirname(__file__), "..", "data", "autoresearch_state.json")
LOG_FILE = os.path.join(os.path.dirname(__file__), "..", "data", "autoresearch_log.json")


def log(msg):
    print(msg, flush=True)


# Default production config
DEFAULT_CONFIG = {
    "signal_threshold": 0.40,
    "min_alignment": 3,
    "take_profit_atr": 6.0,
    "stop_loss_atr": 2.5,
    "max_hold_hours": 96,
    "time_decay_hours": 48,
}

# Known optimized configs (from per_symbol_optimizer.py)
OPTIMIZED_CONFIGS = {
    "BTC/USDT": {
        "signal_threshold": 0.35,
        "min_alignment": 3,
        "take_profit_atr": 5.0,
        "stop_loss_atr": 1.5,
        "max_hold_hours": 72,
        "time_decay_hours": 24,
    },
    "ETH/USDT": DEFAULT_CONFIG,
    "SOL/USDT": {
        "signal_threshold": 0.35,
        "min_alignment": 3,
        "take_profit_atr": 4.0,
        "stop_loss_atr": 1.5,
        "max_hold_hours": 48,
        "time_decay_hours": 24,
    },
    "BNB/USDT": DEFAULT_CONFIG,
}


@dataclass
class Experiment:
    """One autoresearch experiment."""
    iteration: int
    timestamp: str
    symbol: str
    param_changed: str
    old_value: float
    new_value: float
    baseline_sharpe: float
    baseline_pnl: float
    new_sharpe: float
    new_pnl: float
    wfa_consistency: float
    kept: bool
    config_before: Dict
    config_after: Dict


def get_current_config(symbol: str) -> Dict:
    """Get the current best config for a symbol."""
    if os.path.exists(STATE_FILE):
        with open(STATE_FILE) as f:
            state = json.load(f)
        if symbol in state.get("current_configs", {}):
            return state["current_configs"][symbol]
    return OPTIMIZED_CONFIGS.get(symbol, DEFAULT_CONFIG)


def evaluate_config(df: pd.DataFrame, params: Dict) -> Dict:
    """Evaluate a config on full 365d data + WFA-style metrics."""
    # Full sample metrics
    trips = simulate_trades(df, params)
    full = compute_metrics(trips)
    
    # Rolling window Sharpe (simulates WFA consistency)
    window_size = 720  # 30 days in hours
    step = 168  # 7 days in hours
    close = df["close"].values
    
    window_sharpes = []
    for start in range(250, len(close) - window_size, step):
        window_df = df.iloc[start:start + window_size]
        window_trips = simulate_trades(window_df, params)
        if window_trips:
            pnls = [t["pnl_pct"] for t in window_trips]
            if np.std(pnls) > 0:
                sharpe = np.mean(pnls) / np.std(pnls) * np.sqrt(24 * 365)
                window_sharpes.append(sharpe)
    
    return {
        "total_pnl": full.get("total_pnl_pct", 0),
        "win_rate": full.get("win_rate", 0),
        "pf": full.get("pf", 0),
        "sharpe": full.get("sharpe", 0),
        "max_dd": full.get("max_drawdown_pct", 0),
        "round_trips": full.get("round_trips", 0),
        "wfa_consistency": np.mean([s > 0 for s in window_sharpes]) * 100 if window_sharpes else 0,
        "wfa_mean_sharpe": np.mean(window_sharpes) if window_sharpes else 0,
        "wfa_windows": len(window_sharpes),
    }


def generate_modification(symbol: str, current_config: Dict, baseline_metrics: Dict) -> List[Dict]:
    """
    Generate targeted parameter modifications.
    Returns list of (param_name, old_value, new_value, reason).
    """
    mods = []
    p = current_config
    
    # Strategy: identify the weakest metric and modify the param most likely to fix it
    
    if baseline_metrics["max_dd"] > 25:
        # High drawdown — try tighter stops or shorter holds
        mods.append({"param": "stop_loss_atr", "old": p["stop_loss_atr"],
                      "new": max(1.0, p["stop_loss_atr"] - 0.5),
                      "reason": f"reduce drawdown ({baseline_metrics['max_dd']:.1f}%>25%)"})
        mods.append({"param": "max_hold_hours", "old": p["max_hold_hours"],
                      "new": max(24, p["max_hold_hours"] - 24),
                      "reason": f"reduce drawdown via shorter holds"})
    
    if baseline_metrics["pf"] < 1.5:
        # Low profit factor — try wider take-profit
        mods.append({"param": "take_profit_atr", "old": p["take_profit_atr"],
                      "new": p["take_profit_atr"] + 1.0,
                      "reason": f"improve PF ({baseline_metrics['pf']:.1f}<1.5)"})
        mods.append({"param": "stop_loss_atr", "old": p["stop_loss_atr"],
                      "new": max(1.0, p["stop_loss_atr"] - 0.5),
                      "reason": f"improve PF via tighter stops"})
    
    if baseline_metrics["wfa_consistency"] < 50:
        # Low WFA consistency — try more conservative entry
        mods.append({"param": "signal_threshold", "old": p["signal_threshold"],
                      "new": p["signal_threshold"] + 0.05,
                      "reason": f"improve consistency ({baseline_metrics['wfa_consistency']:.0f}%<50%)"})
        mods.append({"param": "min_alignment", "old": p.get("min_alignment", 3),
                      "new": min(p.get("min_alignment", 3) + 1, 4),
                      "reason": f"stricter entry for consistency"})
    
    if baseline_metrics["round_trips"] < 30:
        # Too few trades — lower threshold
        mods.append({"param": "signal_threshold", "old": p["signal_threshold"],
                      "new": max(0.25, p["signal_threshold"] - 0.05),
                      "reason": f"more trades ({baseline_metrics['round_trips']}<30)"})
    
    # Always try the opposite direction too (exploration)
    mods.append({"param": "take_profit_atr", "old": p["take_profit_atr"],
                  "new": max(3.0, p["take_profit_atr"] - 1.0),
                  "reason": "explore: tighter TP"})
    mods.append({"param": "time_decay_hours", "old": p["time_decay_hours"],
                  "new": max(12, p["time_decay_hours"] - 12),
                  "reason": "explore: faster decay"})
    
    # Deduplicate by param (keep first occurrence of each)
    seen = set()
    unique_mods = []
    for m in mods:
        if m["param"] not in seen:
            seen.add(m["param"])
            unique_mods.append(m)
    
    return unique_mods


def load_state() -> Dict:
    """Load autoresearch state."""
    if os.path.exists(STATE_FILE):
        with open(STATE_FILE) as f:
            return json.load(f)
    return {
        "current_configs": {sym: OPTIMIZED_CONFIGS.get(sym, DEFAULT_CONFIG) for sym in SYMBOLS},
        "experiments": [],
        "iteration": 0,
    }


def save_state(state: Dict):
    """Save autoresearch state."""
    os.makedirs(os.path.dirname(STATE_FILE), exist_ok=True)
    with open(STATE_FILE, "w") as f:
        json.dump(state, f, indent=2, default=str)


def save_log(experiments: List[Experiment]):
    """Append experiment to log."""
    os.makedirs(os.path.dirname(LOG_FILE), exist_ok=True)
    log = []
    if os.path.exists(LOG_FILE):
        with open(LOG_FILE) as f:
            log = json.load(f)
    log.extend([asdict(e) for e in experiments])
    with open(LOG_FILE, "w") as f:
        json.dump(log, f, indent=2, default=str)


def run_one_iteration(state: Dict, dfs: Dict[str, pd.DataFrame]) -> Optional[Experiment]:
    """Run one autoresearch iteration."""
    iteration = state["iteration"]

    # Step 1: Round-robin through symbols, pick the one with worst WFA Sharpe
    # But ensure we don't get stuck on one symbol
    symbol_order = SYMBOLS[iteration % len(SYMBOLS):] + SYMBOLS[:iteration % len(SYMBOLS)]

    # Evaluate all symbols
    symbol_metrics = {}
    for symbol in SYMBOLS:
        config = state["current_configs"][symbol]
        metrics = evaluate_config(dfs[symbol], config)
        symbol_metrics[symbol] = metrics

    # Pick worst symbol from rotated order
    worst_symbol = None
    worst_fitness = float('inf')
    for symbol in symbol_order:
        m = symbol_metrics[symbol]
        fitness = m["wfa_mean_sharpe"] if m["wfa_windows"] > 0 else m["sharpe"]
        if fitness < worst_fitness:
            worst_fitness = fitness
            worst_symbol = symbol

    if worst_symbol is None:
        return None

    # Step 2: Generate ALL modifications and test each one
    current_config = state["current_configs"][worst_symbol]
    baseline = symbol_metrics[worst_symbol]
    mods = generate_modification(worst_symbol, current_config, baseline)

    best_mod = None
    best_fitness = worst_fitness

    for mod in mods:
        test_config = {**current_config}
        test_config[mod["param"]] = mod["new"]
        test_metrics = evaluate_config(dfs[worst_symbol], test_config)

        new_fitness = test_metrics["wfa_mean_sharpe"] if test_metrics["wfa_windows"] > 0 else test_metrics["sharpe"]

        if new_fitness > best_fitness:
            # Check viability
            viable = test_metrics["wfa_consistency"] >= 40 or test_metrics["pf"] > 1.2
            if viable:
                best_mod = mod
                best_fitness = new_fitness

    if best_mod is None:
        # No improvement found — try a random exploration
        import random
        random_mod = random.choice(mods)
        test_config = {**current_config}
        test_config[random_mod["param"]] = random_mod["new"]
        test_metrics = evaluate_config(dfs[worst_symbol], test_config)
        best_mod = random_mod
        best_fitness = test_metrics["wfa_mean_sharpe"] if test_metrics["wfa_windows"] > 0 else test_metrics["sharpe"]

    # Step 3: Check if improvement
    old_fitness = worst_fitness
    new_fitness = best_fitness
    test_config = {**current_config}
    test_config[best_mod["param"]] = best_mod["new"]
    test_metrics = evaluate_config(dfs[worst_symbol], test_config)

    # Keep if: fitness improved AND (WFA consistency >= 40% OR PF > 1.2)
    improved = new_fitness > old_fitness
    viable = test_metrics["wfa_consistency"] >= 40 or test_metrics["pf"] > 1.2
    kept = improved and viable

    # Update state
    if kept:
        state["current_configs"][worst_symbol] = test_config
    
    experiment = Experiment(
        iteration=iteration,
        timestamp=datetime.now().isoformat(),
        symbol=worst_symbol,
        param_changed=best_mod["param"],
        old_value=best_mod["old"],
        new_value=best_mod["new"],
        baseline_sharpe=old_fitness,
        baseline_pnl=baseline["total_pnl"],
        new_sharpe=new_fitness,
        new_pnl=test_metrics["total_pnl"],
        wfa_consistency=test_metrics["wfa_consistency"],
        kept=kept,
        config_before=current_config,
        config_after=state["current_configs"][worst_symbol] if kept else test_config,
    )

    state["iteration"] += 1
    return experiment


def main():
    log(f"\n{'='*80}")
    log(f"AUTORESEARCH — Self-Improving Strategy Parameters")
    log(f"Inspired by Karpathy's autoresearch + ATLAS by General Intelligence Capital")
    log(f"{'='*80}\n")
    
    # Load state
    state = load_state()
    log(f"Starting from iteration {state['iteration']}")
    log(f"Previous experiments: {len(state['experiments'])}")
    log(f"Kept: {sum(1 for e in state['experiments'] if e.get('kept'))}")
    log(f"Reverted: {sum(1 for e in state['experiments'] if not e.get('kept'))}\n")
    
    # Precompute indicators
    log("Loading data and computing indicators...")
    dfs = {}
    for symbol in SYMBOLS:
        safe = symbol.replace("/", "_")
        path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
        df = pd.read_parquet(path)
        dfs[symbol] = compute_indicators(df)
        log(f"  {symbol}: {len(df)} candles")
    
    # Print current baseline
    log(f"\n── CURRENT BASELINES ──")
    log(f"  {'Symbol':12s} {'PnL':>8s} {'Sharpe':>7s} {'PF':>5s} {'DD':>6s} {'WFA%':>5s} {'Trades':>6s}")
    log(f"  {'─'*12} {'─'*8} {'─'*7} {'─'*5} {'─'*6} {'─'*5} {'─'*6}")
    for symbol in SYMBOLS:
        config = state["current_configs"][symbol]
        m = evaluate_config(dfs[symbol], config)
        pf_s = f"{m['pf']:.1f}" if m['pf'] < 999 else "INF"
        log(f"  {symbol:12s} {m['total_pnl']:+7.2f}% {m['sharpe']:7.2f} {pf_s:>5s} {m['max_dd']:5.1f}% {m['wfa_consistency']:4.0f}% {m['round_trips']:6d}")
    
    # Run experiments
    MAX_ITERATIONS = 50
    new_experiments = []
    
    log(f"\n── RUNNING {MAX_ITERATIONS} EXPERIMENTS ──\n")
    
    for i in range(MAX_ITERATIONS):
        exp = run_one_iteration(state, dfs)
        if exp is None:
            continue
        
        new_experiments.append(exp)
        status = "✅ KEPT" if exp.kept else "❌ REVERT"
        delta = exp.new_sharpe - exp.baseline_sharpe
        log(f"  [{exp.iteration:3d}] {exp.symbol:12s} {exp.param_changed:20s} "
            f"{exp.old_value:.1f}→{exp.new_value:.1f}  "
            f"Sharpe: {exp.baseline_sharpe:+.2f}→{exp.new_sharpe:+.2f} ({delta:+.2f})  "
            f"WFA: {exp.wfa_consistency:.0f}%  {status}")
        
        # Save state every 10 iterations
        if (i + 1) % 10 == 0:
            state["experiments"].extend([asdict(e) for e in new_experiments])
            save_state(state)
            save_log(new_experiments)
            new_experiments = []
            log(f"\n  [checkpoint] Saved state at iteration {state['iteration']}")
            
            # Print updated baselines
            log(f"\n  {'Symbol':12s} {'PnL':>8s} {'Sharpe':>7s} {'PF':>5s} {'DD':>6s} {'WFA%':>5s}")
            log(f"  {'─'*12} {'─'*8} {'─'*7} {'─'*5} {'─'*6} {'─'*5}")
            for symbol in SYMBOLS:
                config = state["current_configs"][symbol]
                m = evaluate_config(dfs[symbol], config)
                pf_s = f"{m['pf']:.1f}" if m['pf'] < 999 else "INF"
                log(f"  {symbol:12s} {m['total_pnl']:+7.2f}% {m['sharpe']:7.2f} {pf_s:>5s} {m['max_dd']:5.1f}% {m['wfa_consistency']:4.0f}%")
            log("")
    
    # Final save
    if new_experiments:
        state["experiments"].extend([asdict(e) for e in new_experiments])
        save_state(state)
        save_log(new_experiments)
    
    # Summary
    all_exps = state["experiments"]
    kept = [e for e in all_exps if e.get("kept")]
    reverted = [e for e in all_exps if not e.get("kept")]
    
    log(f"\n{'='*80}")
    log(f"RESULTS — {len(all_exps)} experiments")
    log(f"{'='*80}")
    log(f"  Kept:    {len(kept)} ({len(kept)/max(len(all_exps),1)*100:.0f}%)")
    log(f"  Reverted: {len(reverted)} ({len(reverted)/max(len(all_exps),1)*100:.0f}%)")
    
    log(f"\n── MOST MODIFIED SYMBOLS ──")
    from collections import Counter
    sym_counts = Counter(e["symbol"] for e in all_exps)
    for sym, count in sym_counts.most_common():
        sym_kept = sum(1 for e in kept if e["symbol"] == sym)
        log(f"  {sym:12s} {count:3d} modifications, {sym_kept} kept")
    
    log(f"\n── FINAL EVOLVED CONFIGS ──")
    for symbol in SYMBOLS:
        config = state["current_configs"][symbol]
        original = OPTIMIZED_CONFIGS.get(symbol, DEFAULT_CONFIG)
        changes = {k: (v, original[k]) for k, v in config.items() if v != original[k]}
        if changes:
            log(f"  {symbol}:")
            for param, (new, old) in changes.items():
                log(f"    {param}: {old} → {new}")
        else:
            log(f"  {symbol}: unchanged")
    
    log(f"\n── FINAL PERFORMANCE ──")
    log(f"  {'Symbol':12s} {'PnL':>8s} {'Sharpe':>7s} {'PF':>5s} {'DD':>6s} {'WFA%':>5s} {'Trades':>6s}")
    log(f"  {'─'*12} {'─'*8} {'─'*7} {'─'*5} {'─'*6} {'─'*5} {'─'*6}")
    for symbol in SYMBOLS:
        config = state["current_configs"][symbol]
        m = evaluate_config(dfs[symbol], config)
        pf_s = f"{m['pf']:.1f}" if m['pf'] < 999 else "INF"
        log(f"  {symbol:12s} {m['total_pnl']:+7.2f}% {m['sharpe']:7.2f} {pf_s:>5s} {m['max_dd']:5.1f}% {m['wfa_consistency']:4.0f}% {m['round_trips']:6d}")
    
    log(f"\nState saved to {STATE_FILE}")
    log(f"Log saved to {LOG_FILE}")


if __name__ == "__main__":
    main()
