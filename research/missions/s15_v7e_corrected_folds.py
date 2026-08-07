#!/usr/bin/env python3
"""
S15 v7e — corrected equal-window folds + regenerated final verdict.

The diagnostic surfaced a fold-construction artifact in the pipeline's
create_folds(): when the data is longer than num_folds * test_window, the
LAST fold absorbs all remaining bars. On the 2-year window that made fold 8
a 438-day mega-fold holding 212 of 370 trades — the headline OOS stats were
dominated by one oversized window, and "min trades/fold >= 10" passed
trivially.

Fix: rebuild the walk-forward as PROPER EQUAL 36-day anchored windows
covering all data (~20 windows), recompute the fold-level gates on them,
and regenerate FINAL_VERDICT.md. Full-window artifacts (sensitivity,
compounding, latency) are unaffected by fold shape and are carried over.

Also records the honest regime picture: Aug-Dec 2024 was net negative
(-13.4% over 4 months), positive thereafter; worst month 2025-12 (-11.6).
"""
import os
import sys
import json
from datetime import datetime

import numpy as np
import pandas as pd

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT_DIR = os.path.join(ROOT, "data", "results", "s15_v7")
sys.path.insert(0, ROOT)

from research.missions.s15_v7_v2fee_recheck import (
    net_pnl_v2, V2_FIXED_ROUND_TRIP, V2_BORROW_HOURLY, log, START_SOL,
)
import research.missions.s15_v5_pipeline as pipe
import research.missions.s15_v5_finish as fin

pipe.net_pnl = net_pnl_v2
fin.net_pnl = net_pnl_v2
sys.path.insert(0, os.path.join(ROOT, "research", "missions"))
import s15_v6_gap_close as v6

v6.net_pnl = net_pnl_v2

WARMUP = 250
FOLD_DAYS = 36


def equal_folds(df, tf):
    """Anchored walk-forward with EQUAL 36-day test windows, all data used."""
    from research.missions.s15_v5_pipeline import bars_per_hour
    w = int(FOLD_DAYS * 24 * bars_per_hour(tf))
    folds = []
    ts = WARMUP
    i = 0
    while ts + w <= len(df):
        folds.append({"fold_num": i, "test_start": ts, "test_end": ts + w})
        ts += w
        i += 1
    if ts < len(df):  # remainder < one window: append as short final fold
        folds.append({"fold_num": i, "test_start": ts, "test_end": len(df)})
    return folds


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    cfg = json.load(open(os.path.join(ROOT, "data", "results", "s15_v5",
                                      "winning_config.json")))
    p_long = {**cfg["params_long"], "confirm_mode": "none"}
    p_short = {**cfg["params_short"], "confirm_mode": "none"}
    df = v6.load_2yr("SOL/USDT")
    log(f"SOL 2yr: {len(df)} bars | {FOLD_DAYS}d equal folds | v2 fees")

    # --- Corrected WFA ---
    from research.missions.s15_v5_pipeline import compute_metrics, bars_per_hour
    folds = equal_folds(df, v6.TF)
    rows = []
    for f in folds:
        test_df = df.iloc[f["test_start"]:f["test_end"]]
        trips = v6.run_composite(test_df, p_long, p_short, 1.0, v6.TF)
        net = v6.net_pnl(trips, 1.0, v6.TF)
        m = compute_metrics(net, total_hours=len(test_df) / bars_per_hour(v6.TF))
        rows.append({"fold": f["fold_num"],
                     "start": str(df.index[f["test_start"]].date()),
                     "end": str(df.index[f["test_end"] - 1].date()),
                     "oos_trades": m["round_trips"],
                     "oos_pnl": m["total_pnl_pct"],
                     "oos_sharpe": m["sharpe"],
                     "long_net": sum(t["net_pnl"] for t in net if t["direction"] == 1),
                     "short_net": sum(t["net_pnl"] for t in net if t["direction"] == -1)})
    folds_df = pd.DataFrame(rows)
    folds_df.to_csv(os.path.join(OUT_DIR, "folds_blindtouch_equal.csv"), index=False)
    n = len(rows)
    sharpes = [max(-100, min(100, x["oos_sharpe"])) for x in rows]
    trades = [x["oos_trades"] for x in rows]
    cons = sum(1 for s in sharpes if s > 0) / n
    total_pnl = sum(x["oos_pnl"] for x in rows)
    long_net = sum(x["long_net"] for x in rows)
    short_net = sum(x["short_net"] for x in rows)
    med_sh = float(np.median(sharpes))
    min_tr = int(min(trades))
    log(f"\n  EQUAL-FOLD WFA: {n} folds, min trades={min_tr} "
        f"avg={np.mean(trades):.1f} pnl={total_pnl:+.1f}% cons={cons:.0%} "
        f"medsh={med_sh:.2f} long={long_net:+.1f}% short={short_net:+.1f}%")
    neg = folds_df[folds_df["oos_pnl"] <= 0]
    for _, r in neg.iterrows():
        log(f"    neg fold {r['fold']}: {r['start']}->{r['end']} "
            f"{r['oos_trades']}t {r['oos_pnl']:+.1f}%")

    # --- Leverage gate on equal folds ---
    best_lev, best_final = 1.0, -1e9
    lev_rows = []
    for lev in [1.0, 2.0, 3.0, 5.0]:
        fold_pos = sum(1 for f in folds
                       if (lambda nt: nt and sum(t["net_pnl"] for t in nt) > 0)(
                           v6.net_pnl(v6.run_composite(
                               df.iloc[f["test_start"]:f["test_end"]],
                               p_long, p_short, lev, v6.TF), lev, v6.TF)))
        c = v6.compound(df, v6.TF, p_long, p_short, leverage=lev)
        passed = (c["max_dd_pct"] <= 25 and c["liquidations"] == 0 and
                  fold_pos / n >= 0.5 and c["final_sol"] > START_SOL)
        lev_rows.append({"leverage": lev, "final_sol": c["final_sol"],
                         "max_dd_pct": c["max_dd_pct"],
                         "folds_pos": f"{fold_pos}/{n}", "passed": passed})
        log(f"  LEV {lev}x: final={c['final_sol']} dd={c['max_dd_pct']}% "
            f"folds_pos={fold_pos}/{n} {'PASS' if passed else 'fail'}")
        if passed and c["final_sol"] > best_final:
            best_final, best_lev = c["final_sol"], lev

    comp = v6.compound(df, v6.TF, p_long, p_short, leverage=best_lev)
    base5 = comp
    lat5 = v6.compound(df, v6.TF, p_long, p_short, leverage=best_lev, delay_bars=1)
    denom = base5["final_sol"] - START_SOL
    retention = (lat5["final_sol"] - START_SOL) / denom if abs(denom) > 1e-9 else 1.0
    log(f"\n  COMP @ {best_lev}x: final={comp['final_sol']} ({comp['ret_pct']:+.1f}%) "
        f"dd={comp['max_dd_pct']}% liq={comp['liquidations']} halts={comp['halts']} "
        f"trades={comp['trades']} ({comp['trades_per_day']}/day) "
        f"latency retention={retention:.0%}")

    # Sensitivity carried from v7d full-window run (fold-shape independent)
    base_net, spread, flips, robust, _ = fin.sensitivity(
        df, v6.TF, p_long, use_composite=True, params2=p_short)
    log(f"  SENS: base={base_net:+.1f}% spread={spread:.1f} flips={flips} {robust}")

    # Full-window trade floor: folds 0..n-2 are complete 36d windows; the
    # trailing fold (if any) is a short remnant and can never hold ~10
    # trades of a 0.51/day strategy — it would fail by construction, so
    # the gate applies to FULL windows only (remnant reported separately).
    w_bars = int(FOLD_DAYS * 24 * bars_per_hour(v6.TF))
    n_full = sum(1 for f in folds if (f["test_end"] - f["test_start"]) >= w_bars)
    full_min_trades = int(min(r["oos_trades"] for r in rows[:n_full]))

    gates = {
        "oos_positive_total_pnl": total_pnl > 0,
        "oos_consistency_ge_50": cons >= 0.5,
        "bidirectional_attribution_ge_0": long_net >= 0 and short_net >= 0,
        "min_trades_per_fold_ge_10": full_min_trades >= 10,
        "latency_retention_ge_80": retention >= 0.8,
        "sensitivity_robust": flips == 0 and robust == "ROBUST",
        "compounding_gt_start": comp["final_sol"] > START_SOL,
        "dd_le_25": comp["max_dd_pct"] <= 25,
        "zero_halts_zero_liq": comp["halts"] == 0 and comp["liquidations"] == 0,
        "trades_per_day_le_4": comp["trades_per_day"] <= 4,
    }
    passed = sum(gates.values())
    log(f"\n--- GATES (equal folds): {passed}/10 ---")
    for k, v in gates.items():
        log(f"  [{'PASS' if v else 'FAIL'}] {k}")

    pd.DataFrame([{"tag": "20m_blindtouch_equal_folds",
                   "gates_passed": f"{passed}/{len(gates)}",
                   **{k: bool(v) for k, v in gates.items()},
                   "num_folds": n, "min_oos_trades": min_tr,
                   "total_oos_pnl": round(total_pnl, 1),
                   "consistency": round(cons, 3),
                   "median_oos_sharpe": round(med_sh, 2),
                   "long_net": round(long_net, 1), "short_net": round(short_net, 1),
                   "best_leverage": best_lev, "final_sol": comp["final_sol"],
                   "dd": comp["max_dd_pct"],
                   "retention": round(retention, 3)}]
                 ).to_csv(os.path.join(OUT_DIR, "final_gate_matrix.csv"), index=False)

    # --- Regime stats for the verdict ---
    trips = v6.run_composite(df, p_long, p_short, 1.0, v6.TF)
    net = v6.net_pnl(trips, 1.0, v6.TF)
    ndf = pd.DataFrame(net)
    ndf["entry_time"] = df.index[ndf["entry_idx"]]
    y1 = ndf[ndf["entry_time"] < "2025-08-06"]
    y2 = ndf[ndf["entry_time"] >= "2025-08-06"]

    ts = datetime.now().isoformat()
    L = [f"# S15 FINAL VERDICT — Friend's Engine (v7e, corrected folds)\n\n"
         f"Generated: {ts}\n\n",
         "## Verdict: " + ("DEPLOYABLE" if all(gates.values()) else
                          f"CONDITIONAL — {passed}/10 gates") + "\n\n",
         "Data: SOL/USDT 20m, 2 years (2024-08-06 -> 2026-08-05), 52,530 bars\n",
         "Fee basis: MEASURED Flash v2 (2026-08-07) — 0.02% open + 0.02% close + "
         "~0.01%/side spread + 0.0004%/hr borrow (both sides) = "
         f"{V2_FIXED_ROUND_TRIP:.2f}% round-trip\n\n",
         "## Operating config (limit-at-zone execution)\n\n",
         "- Entry: blind touch (confirm_mode=none) — resting limit at the "
         "retracement zone; the order book absorbs detection latency\n",
         f"- Leverage: {best_lev}x | TF: 20m | composite long+short legs\n",
         "- Champion params: v5 winning_config.json with confirm_mode flipped "
         "none on both legs\n\n",
         f"## Gate matrix ({n_full} full 36-day anchored windows + 1 trailing "
         "6-day remnant)\n\n"]
    for k, v in gates.items():
        L.append(f"- [{'PASS' if v else 'FAIL'}] {k}\n")
    L += [f"\n## Headline numbers\n\n",
          f"- OOS PnL: {total_pnl:+.1f}% over {int(sum(trades))} trades "
          f"(avg {np.mean(trades):.1f}/fold; full-window min {full_min_trades}, "
          f"trailing 6-day remnant fold excluded from the floor: "
          f"{rows[-1]['oos_trades']}t)\n",
          f"- Consistency: {cons:.0%} ({sum(1 for s in sharpes if s > 0)}/{n} folds "
          f"positive) | median OOS Sharpe {med_sh:.2f}\n",
          f"- Attribution: long {long_net:+.1f}% / short {short_net:+.1f}%\n",
          f"- Compounded @ {best_lev}x from 2.5 SOL: {comp['final_sol']} SOL "
          f"({comp['ret_pct']:+.1f}%), DD {comp['max_dd_pct']}%, "
          f"{comp['trades_per_day']}/day, {comp['liquidations']} liq, "
          f"{comp['halts']} halts\n",
          f"- Latency retention (+1 bar): {retention:.0%}\n",
          f"- Sensitivity: {robust} (spread {spread:.1f}, {flips} sign flips)\n\n",
          "## Regime honesty\n\n",
          f"- Year 1 (2024-08 -> 2025-08): {len(y1)} trades, net "
          f"{y1['net_pnl'].sum():+.1f}% — Aug-Dec 2024 was underwater "
          f"(-13.4% across 4 months) before turning positive Jan 2025\n",
          f"- Year 2 (2025-08 -> 2026-08): {len(y2)} trades, net "
          f"{y2['net_pnl'].sum():+.1f}%\n",
          f"- Worst single month: 2025-12 (-11.6%); best: 2025-11 (+14.4%)\n",
          f"- Negative folds: {len(neg)} of {n} "
          f"({', '.join(r['start'] for _, r in neg.iterrows()) or 'none'})\n\n",
          "## Methodology correction (v7e)\n\n",
          "create_folds() absorbs all leftover bars into the LAST fold when data "
          "> num_folds x window — on 2yr data that made one 438-day mega-fold "
          "hold 212 of 370 trades and dominate the headline stats. v7e rebuilds "
          f"the walk-forward as {n_full} equal 36-day anchored windows (+ 1 "
          "trailing 6-day remnant, reported but excluded from the "
          f"{FOLD_DAYS}d trade floor — a 6-day window can never hold 10 trades "
          "of a 0.51/day strategy) and re-runs all fold-level gates on them. "
          "Full-window artifacts (sensitivity, compounding, latency) are "
          "fold-shape independent and unchanged.\n\n",
          "## Lineage\n\n",
          "1. v5: champion 20m_COMPOSITE (close_reassert cb1), 8/11 criteria, "
          "latency 48% FAIL, v1-era fees\n",
          "2. v6: 2yr re-validation falsified champion (33% cons, -13.5%) under "
          "v1-era fees (0.32%/trip) — fee artifact\n",
          "3. v7: measured v2 fees resurrect champion (8/10); gaps latency 47%, "
          "min trades 4\n",
          "4. v7b: 72d folds fix trade floor; cb2 NOT the latency absorber (-88%)\n",
          "5. v7c: blind touch (limit-at-zone) — latency 108%, min trades 31, "
          "OOS +68.4%\n",
          "6. v7d: full gate suite on blind touch — 10/10, but fold artifact "
          "discovered\n",
          "7. v7e: equal-fold re-gate (this document)\n\n",
          "## Remaining risks before live deployment\n\n",
          "- Live limit-order placement on Flash must be verified (order support, "
          "fill rate at zone price); backtest assumes fills at touch-bar close, "
          "stressed +1 bar\n",
          "- SOL-only mandate (BTC/ETH transfer negative) — waived by design\n",
          "- Aug-Dec 2024 regime was net negative: expect multi-month drawdown "
          "periods in live operation\n",
          "- Rust momentum off-by-one (flagged, separate decision)\n"]
    with open(os.path.join(OUT_DIR, "FINAL_VERDICT.md"), "w") as f:
        f.write("".join(L))
    log("\nWrote data/results/s15_v7/FINAL_VERDICT.md")


if __name__ == "__main__":
    main()
