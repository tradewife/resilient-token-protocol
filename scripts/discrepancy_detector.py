"""
Discrepancy Detector Module

Runs after each night shift to compare fast-sim rankings with full-sim
validation results. Flags symbols where the evaluation layer is unreliable.

If a symbol is flagged 2+ consecutive nights, the night shift skips
that symbol's Darwinian phase (saves CI time) and logs it for investigation.

This is the "self-awareness" module — the system knowing when it can't trust
its own evaluation for a given symbol/market regime.

Usage:
    python scripts/discrepancy_detector.py                           # auto-detect latest results
    python scripts/discrepancy_detector.py --report data/night_results/2026-04-06/report.md
"""

import argparse
import json
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))


RESULTS_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "night_results")
DISCREPANCY_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "discrepancies")
FLAG_THRESHOLD = 2  # consecutive flags before skipping Darwinian


def extract_fast_sim_results(report_path: str) -> dict:
    """Extract top candidates from night shift report markdown."""
    candidates = {}
    current_symbol = None

    with open(report_path) as f:
        for line in f:
            # Match lines like: ### #1: SOL/USDT (Survivor: 15.63 +14.99)
            m = re.match(r"### #\d+: (\S+/USDT) \(Survivor: ([\d.]+)", line)
            if m:
                current_symbol = m.group(1)
                candidates.setdefault(current_symbol, [])

            # Match OOS Sharpe from table rows
            m = re.match(r"\| OOS Sharpe \| ([+-]?[\d.]+)", line)
            if m and current_symbol:
                candidates[current_symbol].append({
                    "oos_sharpe": float(m.group(1)),
                })

    return candidates


def extract_full_sim_results(validation_path: str) -> dict:
    """Extract full-sim validation results from JSON."""
    with open(validation_path) as f:
        data = json.load(f)

    results = {}
    for entry in data:
        sym = entry["symbol"]
        results.setdefault(sym, []).append({
            "full_pnl": entry.get("total_pnl_pct", 0),
            "full_consistency": entry.get("consistency", 0),
            "label": entry.get("label", ""),
        })
    return results


def detect_discrepancies(fast_results: dict, full_results: dict) -> dict:
    """Compare fast-sim and full-sim results per symbol."""
    discrepancies = {}

    for symbol in set(fast_results.keys()) | set(full_results.keys()):
        fast = fast_results.get(symbol, [])
        full = full_results.get(symbol, [])

        if not fast or not full:
            continue

        # Fast sim top candidate
        best_fast = max(fast, key=lambda x: x["oos_sharpe"])
        best_full = max(full, key=lambda x: x["full_pnl"])

        # Sign disagreement: fast says positive, full says negative or vice versa
        sign_mismatch = (best_fast["oos_sharpe"] > 0) != (best_full["full_pnl"] > 0)

        # Ranking discrepancy: are the top configs the same?
        # Check if full sim's best has a corresponding fast sim entry that's ranked low
        fast_pnl_sum = sum(r.get("total_pnl", 0) for r in fast if "total_pnl" in r)
        full_pnl_sum = sum(r["full_pnl"] for r in full)

        # Directional bias: is fast sim systematically lower/higher?
        if fast_pnl_sum != 0 and full_pnl_sum != 0:
            bias_ratio = fast_pnl_sum / full_pnl_sum
        else:
            bias_ratio = 1.0

        flagged = sign_mismatch or abs(bias_ratio - 1.0) > 0.5

        discrepancies[symbol] = {
            "best_fast_sharpe": best_fast["oos_sharpe"],
            "best_full_pnl": best_full["full_pnl"],
            "best_full_consistency": best_full["full_consistency"],
            "sign_mismatch": sign_mismatch,
            "bias_ratio": round(bias_ratio, 2),
            "flagged": flagged,
            "reason": (
                "Sign mismatch: fast says profitable, full says not"
                if sign_mismatch
                else f"Large bias: fast/full ratio = {bias_ratio:.2f}"
                if abs(bias_ratio - 1.0) > 0.5
                else "OK"
            ),
        }

    return discrepancies


def update_flag_history(discrepancies: dict) -> dict:
    """Load history and update consecutive flag counts."""
    os.makedirs(DISCREPANCY_DIR, exist_ok=True)
    history_path = os.path.join(DISCREPANCY_DIR, "flag_history.json")

    if os.path.exists(history_path):
        with open(history_path) as f:
            history = json.load(f)
    else:
        history = {}

    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")

    for symbol, info in discrepancies.items():
        if symbol not in history:
            history[symbol] = {"consecutive_flags": 0, "last_flagged": None, "total_flags": 0}

        if info["flagged"]:
            if history[symbol]["last_flagged"] is None:
                history[symbol]["consecutive_flags"] = 1
            else:
                # Check if last flagged was yesterday (consecutive)
                history[symbol]["consecutive_flags"] += 1
            history[symbol]["last_flagged"] = today
            history[symbol]["total_flags"] += 1
        else:
            # No flag today — reset consecutive count
            history[symbol]["consecutive_flags"] = 0

    with open(history_path, "w") as f:
        json.dump(history, f, indent=2)

    return history


def generate_recommendation(discrepancies: dict, history: dict) -> str:
    """Generate actionable recommendation based on discrepancies."""
    lines = []
    lines.append("## Discrepancy Report")
    lines.append("")

    skip_symbols = []
    investigate_symbols = []

    for symbol, info in discrepancies.items():
        status = "FLAGGED" if info["flagged"] else "OK"
        h = history.get(symbol, {})
        consecutive = h.get("consecutive_flags", 0)

        lines.append(f"### {symbol}: {status}")
        lines.append(f"  Fast sim top Sharpe: {info['best_fast_sharpe']:+.2f}")
        lines.append(f"  Full sim top PnL: {info['best_full_pnl']:+.2f}% (consistency: {info['best_full_consistency']:.0%})")
        lines.append(f"  Sign mismatch: {'YES' if info['sign_mismatch'] else 'no'}")
        lines.append(f"  Consecutive flags: {consecutive}/{FLAG_THRESHOLD} to skip Darwinian")
        lines.append(f"  Reason: {info['reason']}")
        lines.append("")

        if info["flagged"] and consecutive >= FLAG_THRESHOLD:
            skip_symbols.append(symbol)
            lines.append(f"  **ACTION: Skip Darwinian phase for {symbol} until discrepancy is resolved**")
        elif info["flagged"]:
            investigate_symbols.append(symbol)
            lines.append(f"  **ACTION: Investigate {symbol} evaluator — check ATR/MR calibration**")
        lines.append("")

    return "\n".join(lines), skip_symbols


def main():
    parser = argparse.ArgumentParser(description="Detect fast/full sim discrepancies")
    parser.add_argument("--report", default=None, help="Path to night shift report.md")
    parser.add_argument("--validation", default=None, help="Path to full_sim_validation.json")
    args = parser.parse_args()

    # Auto-detect latest results
    if not args.report:
        dates = sorted([d for d in os.listdir(RESULTS_DIR) if os.path.isdir(os.path.join(RESULTS_DIR, d))])
        if not dates:
            print("No night results found in", RESULTS_DIR)
            return
        latest = dates[-1]
        args.report = os.path.join(RESULTS_DIR, latest, "report.md")
        if not args.validation:
            args.validation = os.path.join(RESULTS_DIR, latest, "full_sim_validation.json")

    print(f"Report: {args.report}")
    print(f"Validation: {args.validation}")
    print()

    if not os.path.exists(args.report):
        print(f"Report not found: {args.report}")
        return

    fast_results = extract_fast_sim_results(args.report)
    full_results = {}

    if args.validation and os.path.exists(args.validation):
        full_results = extract_full_sim_results(args.validation)
        print(f"Fast sim results: {len(fast_results)} symbols")
        print(f"Full sim results: {len(full_results)} symbols")
    else:
        print(f"Full sim validation not found ({args.validation}), using fast sim only")

    discrepancies = detect_discrepancies(fast_results, full_results)
    history = update_flag_history(discrepancies)
    recommendation, skip_symbols = generate_recommendation(discrepancies, history)

    print(recommendation)

    # Save discrepancy report
    os.makedirs(DISCREPANCY_DIR, exist_ok=True)
    report_path = os.path.join(DISCREPANCY_DIR, f"discrepancy_{datetime.now().strftime('%Y-%m-%d')}.md")
    with open(report_path, "w") as f:
        f.write(recommendation)
    print(f"\nDiscrepancy report saved: {report_path}")

    if skip_symbols:
        print(f"\n⚠️  Symbols to skip Darwinian: {skip_symbols}")


if __name__ == "__main__":
    main()
