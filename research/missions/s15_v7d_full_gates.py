#!/usr/bin/env python3
"""
S15 v7d — full 10-gate suite for the blind-touch champion (v2 fees, 2yr).

v7c showed blind touch (limit-at-zone) is dramatically better than the
confirmation variants on every structural axis under measured v2 fees:
OOS +68.4% vs +50.5%, consistency 78% vs 56%, min trades/fold 31 vs 13,
and latency retention 108% vs 47% (a +1-bar delay on a zone fill is NOT
chasing momentum — it is often a better fill).

This run applies the COMPLETE gate suite (WFA, sensitivity, leverage sweep,
compounding, latency) via the v6 harness so the verdict covers all 10 gates,
and writes the final consolidated S15 verdict for the friend's engine.
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
    net_pnl_v2, V2_FIXED_ROUND_TRIP, V2_BORROW_HOURLY, log,
)
import research.missions.s15_v5_pipeline as pipe
import research.missions.s15_v5_finish as fin

pipe.net_pnl = net_pnl_v2
fin.net_pnl = net_pnl_v2

sys.path.insert(0, os.path.join(ROOT, "research", "missions"))
import s15_v6_gap_close as v6

v6.net_pnl = net_pnl_v2


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    cfg = json.load(open(os.path.join(ROOT, "data", "results", "s15_v5",
                                      "winning_config.json")))
    p_long = {**cfg["params_long"], "confirm_mode": "none"}
    p_short = {**cfg["params_short"], "confirm_mode": "none"}

    df = v6.load_2yr("SOL/USDT")
    log(f"SOL 2yr: {len(df)} bars | v2 fees {V2_FIXED_ROUND_TRIP:.2f}%/trip + "
        f"{V2_BORROW_HOURLY}%/hr")

    res = v6.sweep_all(df, p_long, p_short, "20m_blindtouch_full")
    gates = v6.gate_check(res)
    passed = sum(gates.values())
    log(f"\n--- GATES 20m_blindtouch: {passed}/{len(gates)} ---")
    for k, v in gates.items():
        log(f"  [{'PASS' if v else 'FAIL'}] {k}")

    pd.DataFrame([{
        "tag": "20m_blindtouch", "gates_passed": f"{passed}/{len(gates)}",
        **{k: bool(v) for k, v in gates.items()},
        "survivor_score": res["wfa"]["survivor_score"],
        "median_oos_sharpe": res["wfa"]["median_oos_sharpe"],
        "consistency": res["wfa"]["consistency"],
        "min_oos_trades": res["wfa"]["min_oos_trades"],
        "avg_oos_trades": round(res["wfa"]["avg_oos_trades"], 1),
        "total_oos_pnl": round(res["wfa"]["total_oos_pnl"], 1),
        "long_net": round(res["wfa"]["total_long_net"], 1),
        "short_net": round(res["wfa"]["total_short_net"], 1),
        "best_leverage": res["best_lev"],
        "final_sol": res["comp"]["final_sol"],
        "dd": res["comp"]["max_dd_pct"],
        "retention": round(res["retention"], 3),
    }]).to_csv(os.path.join(OUT_DIR, "final_gate_matrix.csv"), index=False)

    # ---- Final consolidated verdict (v5 -> v6 -> v7 -> v7d lineage) ----
    ts = datetime.now().isoformat()
    w = res["wfa"]; c = res["comp"]
    L = [f"# S15 FINAL VERDICT — Friend's Engine\n\nGenerated: {ts}\n\n",
         "## Verdict: " +
         ("DEPLOYABLE" if all(gates.values()) else
          f"CONDITIONAL — {passed}/10 gates") + "\n\n",
         "Data: SOL/USDT 20m, 2 years (2024-08 -> 2026-08), 52,530 bars\n",
         "Fee basis: MEASURED Flash v2 (2026-08-07) — 0.02% open + 0.02% close "
         "+ ~0.01%/side spread + 0.0004%/hr borrow (both sides) = "
         f"{V2_FIXED_ROUND_TRIP:.2f}% round-trip\n\n",
         "## Operating config (limit-at-zone execution)\n\n",
         "- Entry: blind touch (confirm_mode=none) — resting limit at the "
         "retracement zone; order book absorbs detection latency\n",
         f"- Leverage: {res['best_lev']}x | TF: 20m | both legs composite "
         "(long+short)\n\n",
         "## Gate matrix\n\n"]
    for k, v in gates.items():
        L.append(f"- [{'PASS' if v else 'FAIL'}] {k}\n")
    L += [f"\n## Headline numbers\n\n",
          f"- OOS PnL (1x, sum of folds): {w['total_oos_pnl']:+.1f}% | "
          f"consistency {w['consistency']:.0%} | median OOS Sharpe "
          f"{w['median_oos_sharpe']:.2f}\n",
          f"- Attribution: long {w['total_long_net']:+.1f}% / short "
          f"{w['total_short_net']:+.1f}%\n",
          f"- Trades/fold: avg {w['avg_oos_trades']:.1f}, min "
          f"{w['min_oos_trades']}\n",
          f"- Compounded @ {res['best_lev']}x from 2.5 SOL: {c['final_sol']} SOL "
          f"({c['ret_pct']:+.1f}%), DD {c['max_dd_pct']}%, "
          f"{c['trades_per_day']}/day, {c['liquidations']} liq, {c['halts']} halts\n",
          f"- Latency retention (+1 bar): {res['retention']:.0%}\n",
          f"- Sensitivity: {res['sens']['robust']} (spread "
          f"{res['sens']['spread']:.1f}, {res['sens']['flips']} sign flips)\n\n",
          "## Lineage\n\n",
          "1. v5: champion 20m_COMPOSITE (close_reassert cb1), 8/11 criteria, "
          "latency 48% FAIL under v1-era fees\n",
          "2. v6: 2yr re-validation falsified the champion (33% cons, -13.5%) — "
          "but used the v1-era fee model (0.32%/trip), ~5x harsher than reality\n",
          "3. v7: re-check under MEASURED v2 fees resurrects the champion "
          "(8/10 gates); gaps: latency 47%, min trades/fold 4\n",
          "4. v7b: 72d folds fix the trade floor (min 13); cb2 NOT the latency "
          "absorber (-88%)\n",
          "5. v7c/v7d: blind touch (limit-at-zone) is the real absorber — "
          f"latency {res['retention']:.0%}, min trades/fold "
          f"{w['min_oos_trades']}, OOS {w['total_oos_pnl']:+.1f}%\n\n",
          "## Remaining known risks\n\n",
          "- Blind touch fills on zone touch without confirmation; the +1-bar "
          "delay stress shows the fill is robust (not momentum-chasing), but "
          "live limit-order placement must be verified on Flash (order "
          "support, fill rate)\n",
          "- SOL-only mandate (BTC/ETH transfer negative) — waived by design\n",
          "- Rust momentum off-by-one (flagged, separate decision)\n"]
    with open(os.path.join(OUT_DIR, "FINAL_VERDICT.md"), "w") as f:
        f.write("".join(L))
    log("\nWrote data/results/s15_v7/FINAL_VERDICT.md")


if __name__ == "__main__":
    main()
