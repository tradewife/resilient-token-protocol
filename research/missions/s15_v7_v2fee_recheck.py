#!/usr/bin/env python3
"""
S15 v7 — v2-fee re-check of the v5 champion on 2-year data.

The v6 gap closure re-validated the v5 champion on 2-year data but FAILED it
using the pipeline's v1-era fee model (open 0.06% + close 0.06% + 0.10%
slippage/side = 0.32% round-trip, borrow 0.0042%/hr shorts-only). The Track 1a
post-mortem measured Flash v2's ACTUAL costs at ~0.06% round-trip (open 0.02%
+ close 0.02% + spread ~0.01%/side) and 0.0004%/hr borrow — roughly 5x
cheaper. So the v6 falsification may be a fee artifact, not a genuine
regime finding.

This script re-runs the exact v6 gate suite with the MEASURED v2 fee model
(borrow on BOTH sides — v2 margin fees accrue on all open positions per pool
utilization, unlike v1's short-only borrow). If the champion passes, the
friend's engine proceeds with v2 fees as the new cost basis; if it still
fails, the family needs a full re-forge on 2-year data (overnight job) and
the honest verdict is "factory not yet proven".
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

# Measured Flash v2 fee model (2026-08-07, preview/* endpoints + live accrual)
V2_OPEN_FEE = 0.02      # % of notional
V2_CLOSE_FEE = 0.02     # % of notional
V2_SPREAD_SIDE = 0.01   # % of notional per side (entry AND exit)
V2_BORROW_HOURLY = 0.0004  # % of notional per hour, BOTH sides in v2
V2_FIXED_ROUND_TRIP = V2_OPEN_FEE + V2_CLOSE_FEE + 2 * V2_SPREAD_SIDE  # 0.06%

import research.missions.s15_v5_pipeline as pipe
import research.missions.s15_v5_finish as fin
from research.missions.s15_v5_pipeline import bars_per_hour, START_SOL


def log(msg):
    print(f"[{datetime.now().strftime('%H:%M:%S')}] {msg}", flush=True)


def net_pnl_v2(trips, leverage=1.0, tf_minutes=15):
    """v2 cost basis: 0.06% round-trip + 0.0004%/hr borrow on BOTH sides."""
    bph = bars_per_hour(tf_minutes)
    net_trips = []
    for t in trips:
        lev = max(leverage, 1.0)
        hold_hours = t.get("hold_bars", t.get("hold_hrs", 0)) / bph
        borrow = V2_BORROW_HOURLY * hold_hours  # both sides in v2
        fee = lev * (V2_FIXED_ROUND_TRIP + borrow)
        net = t["pnl_pct"] - fee
        nt = dict(t)
        nt["gross_pnl"] = t["pnl_pct"]
        nt["net_pnl"] = net
        nt["fee_pct"] = fee
        net_trips.append(nt)
    return net_trips


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    # Swap the cost basis everywhere the finisher reads it
    pipe.net_pnl = net_pnl_v2
    fin.net_pnl = net_pnl_v2
    fin.compound.__globals__["net_pnl"] = net_pnl_v2

    cfg = json.load(open(os.path.join(ROOT, "data", "results", "s15_v5",
                                      "winning_config.json")))
    p_long, p_short = cfg["params_long"], cfg["params_short"]
    # 2-year SOL window (y2 2024-08->2025-08 + y1 2025-08->2026-08), deduped
    d = os.path.join(ROOT, "data", "ohlcv")
    y2 = pd.read_parquet(os.path.join(d, "SOL_USDT_20m_y2.parquet"))
    y1 = pd.read_parquet(os.path.join(d, "SOL_USDT_20m.parquet"))
    df = pd.concat([y2, y1]).sort_index()
    df = df[~df.index.duplicated(keep="first")]
    log(f"SOL 2yr window: {len(df)} bars {df.index.min()} -> {df.index.max()}")
    log(f"Fee basis: {V2_FIXED_ROUND_TRIP:.2f}% round-trip + "
        f"{V2_BORROW_HOURLY}%/hr borrow (both sides)")

    # Import gap-closure helpers (reuse its sweep/gate logic)
    sys.path.insert(0, os.path.join(ROOT, "research", "missions"))
    import s15_v6_gap_close as v6
    v6.net_pnl = net_pnl_v2  # ensure its wfa() uses v2 basis too

    results = []
    for tag, pl, ps in [("20m_COMPOSITE_cb1_v2fee", p_long, p_short)]:
        res = v6.sweep_all(df, pl, ps, tag)
        gates = v6.gate_check(res)
        passed = sum(gates.values())
        results.append({"tag": tag, "gates_passed": f"{passed}/{len(gates)}",
                        **{k: bool(v) for k, v in gates.items()},
                        "survivor_score": res["wfa"]["survivor_score"],
                        "median_oos_sharpe": res["wfa"]["median_oos_sharpe"],
                        "consistency": res["wfa"]["consistency"],
                        "min_oos_trades": res["wfa"]["min_oos_trades"],
                        "total_oos_pnl": res["wfa"]["total_oos_pnl"],
                        "best_leverage": res["best_lev"],
                        "final_sol": res["comp"]["final_sol"],
                        "retention": res["retention"]})
        log(f"\n--- GATES {tag}: {passed}/{len(gates)} ---")
        for k, v in gates.items():
            log(f"  [{'PASS' if v else 'FAIL'}] {k}")

    pd.DataFrame(results).to_csv(os.path.join(OUT_DIR, "gate_matrix.csv"), index=False)

    ts = datetime.now().isoformat()
    g = results[0]
    L = [f"# S15 v7 — v2-Fee Re-Check (2-Year Data)\n\nGenerated: {ts}\n\n",
         f"Data: SOL/USDT 20m, {df.index.min()} -> {df.index.max()} ({len(df)} bars)\n",
         f"Fee basis: open {V2_OPEN_FEE}% + close {V2_CLOSE_FEE}% + spread "
         f"{V2_SPREAD_SIDE}%/side + borrow {V2_BORROW_HOURLY}%/hr (both sides) = "
         f"{V2_FIXED_ROUND_TRIP:.2f}% round-trip — MEASURED on Flash v2 2026-08-07\n\n",
         "## Question\n\nThe v6 gap closure falsified the v5 champion on 2-year data "
         "(cons 33%, -13.5% compounded) but used the v1-era fee model (0.32%/trip), "
         "~5x harsher than measured v2 reality. Was that falsification real or a "
         "fee artifact?\n\n",
         "## Result\n\n",
         f"- Gates passed: **{g['gates_passed']}**\n",
         f"- OOS PnL: {g['total_oos_pnl']:+.1f}% | consistency {g['consistency']:.0%} | "
         f"min trades/fold {g['min_oos_trades']}\n",
         f"- Compounded @ {g['best_leverage']}x: {g['final_sol']} SOL "
         f"| latency retention {g['retention']:.0%}\n\n",
         "## Gate detail\n\n" +
         "\n".join(f"- {'PASS' if g[k] else 'FAIL'}: {k}"
                   for k in g if k not in ("tag", "gates_passed", "survivor_score",
                                           "median_oos_sharpe", "consistency",
                                           "min_oos_trades", "total_oos_pnl",
                                           "best_leverage", "final_sol", "retention")) +
         "\n"]
    with open(os.path.join(OUT_DIR, "verdict.md"), "w") as f:
        f.write("".join(L))
    log("\nWrote data/results/s15_v7/verdict.md")


if __name__ == "__main__":
    main()
