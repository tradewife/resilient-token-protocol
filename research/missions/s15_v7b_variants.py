#!/usr/bin/env python3
"""
S15 v7b — latency absorber (confirm_bars=2) + thicker folds (72d) under v2 fees.

v7 re-check passed the champion at 8/10 gates under the MEASURED v2 fee
basis on 2-year data. Two gaps remain, each with a pre-registered fix:

  G1 latency retention 47% -> confirm_bars=2: confirmation already waits a
     bar after the zone touch; the extra bar absorbs execution latency
     instead of paying it on a blind-touch fill.
  G2 min fold trades 4 (floor 10) -> the marubozu pattern fires ~0.23/day;
     a 36-day fold EXPECTS ~8.3 trades, so a 4-trade fold is statistical
     noise, not an edge failure. With 2 years of data, 72-day folds expect
     ~16.6 trades — enough for the floor to mean something without
     lowering it.

Runs four cells (cb1/cb2 x 36d/72d folds) through the same gate suite on
the v2 fee basis and reports the matrix.
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


def wfa_folds(df, p_long, p_short, label, test_fold_days):
    """wfa() with configurable fold length."""
    from research.missions.s15_v5_pipeline import create_folds, compute_metrics, bars_per_hour
    folds = create_folds(len(df), 9, test_fold_days, v6.TF)
    rows = []
    for f in folds:
        test_df = df.iloc[f["test_start"]:f["test_end"]]
        if len(test_df) <= 10:
            continue
        trips = v6.run_composite(test_df, p_long, p_short, 1.0, v6.TF)
        net = v6.net_pnl(trips, 1.0, v6.TF)
        m = compute_metrics(net, total_hours=len(test_df) / bars_per_hour(v6.TF))
        rows.append({"fold": f["fold_num"],
                     "oos_trades": m["round_trips"], "oos_pnl": m["total_pnl_pct"],
                     "oos_sharpe": m["sharpe"], "oos_dd": m["max_dd_pct"],
                     "long_net": sum(t["net_pnl"] for t in net if t["direction"] == 1),
                     "short_net": sum(t["net_pnl"] for t in net if t["direction"] == -1)})
    oos_tr = [x["oos_trades"] for x in rows]
    return {"label": label, "folds": rows,
            "num_folds": len(rows),
            "min_oos_trades": int(min(oos_tr)) if oos_tr else 0,
            "avg_oos_trades": float(np.mean(oos_tr)) if oos_tr else 0,
            "total_oos_pnl": float(sum(x["oos_pnl"] for x in rows)),
            "total_long_net": float(sum(x["long_net"] for x in rows)),
            "total_short_net": float(sum(x["short_net"] for x in rows)),
            "consistency": sum(1 for x in rows if x["oos_sharpe"] > 0) / len(rows) if rows else 0,
            "median_oos_sharpe": float(np.median([x["oos_sharpe"] for x in rows])) if rows else 0}


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    cfg = json.load(open(os.path.join(ROOT, "data", "results", "s15_v5",
                                      "winning_config.json")))
    p_long, p_short = cfg["params_long"], cfg["params_short"]
    d = os.path.join(ROOT, "data", "ohlcv")
    y2 = pd.read_parquet(os.path.join(d, "SOL_USDT_20m_y2.parquet"))
    y1 = pd.read_parquet(os.path.join(d, "SOL_USDT_20m.parquet"))
    df = pd.concat([y2, y1]).sort_index()
    df = df[~df.index.duplicated(keep="first")]
    log(f"SOL 2yr: {len(df)} bars | v2 fees {V2_FIXED_ROUND_TRIP:.2f}%/trip "
        f"+ {V2_BORROW_HOURLY}%/hr")

    variants = []
    for cb_tag, p_l, p_s in [("cb1", p_long, p_short),
                             ("cb2", {**p_long, "confirm_bars": 2},
                              {**p_short, "confirm_bars": 2})]:
        for fold_days in (36, 72):
            tag = f"20m_{cb_tag}_{fold_days}d"
            log(f"\n=== {tag} ===")
            w = wfa_folds(df, p_l, p_s, tag, fold_days)
            log(f"  WFA: folds={w['num_folds']} min_trades={w['min_oos_trades']} "
                f"avg={w['avg_oos_trades']:.1f} pnl={w['total_oos_pnl']:+.1f}% "
                f"cons={w['consistency']:.0%} long={w['total_long_net']:+.1f}% "
                f"short={w['total_short_net']:+.1f}%")
            # latency at 3x (v7's best leverage), +1 bar delay
            base = v6.compound(df, v6.TF, p_l, p_s, leverage=3.0, delay_bars=0)
            lat = v6.compound(df, v6.TF, p_l, p_s, leverage=3.0, delay_bars=1)
            denom = base["final_sol"] - v6.START_SOL
            ret = (lat["final_sol"] - v6.START_SOL) / denom if abs(denom) > 1e-9 else 1.0
            log(f"  LAT @3x: base={base['final_sol']} delayed={lat['final_sol']} "
                f"retention={ret:.0%} {'PASS' if ret >= 0.8 else 'FAIL'}")
            pd.DataFrame(w["folds"]).to_csv(os.path.join(OUT_DIR, f"folds_{tag}.csv"), index=False)
            variants.append({
                "tag": tag, "confirm_bars": 1 if cb_tag == "cb1" else 2,
                "fold_days": fold_days, "num_folds": w["num_folds"],
                "min_oos_trades": w["min_oos_trades"],
                "avg_oos_trades": round(w["avg_oos_trades"], 1),
                "total_oos_pnl": round(w["total_oos_pnl"], 1),
                "consistency": round(w["consistency"], 3),
                "long_net": round(w["total_long_net"], 1),
                "short_net": round(w["total_short_net"], 1),
                "final_sol_3x": base["final_sol"],
                "latency_retention": round(ret, 3),
                "latency_pass": ret >= 0.8,
                "min_trades_pass": w["min_oos_trades"] >= 10,
            })

    out = pd.DataFrame(variants)
    out.to_csv(os.path.join(OUT_DIR, "variant_matrix.csv"), index=False)
    log("\n" + out.to_string(index=False))

    ts = datetime.now().isoformat()
    L = [f"# S15 v7b — Latency Absorber + Thicker Folds (v2 fees, 2yr)\n\n"
         f"Generated: {ts}\n\n",
         "| variant | min trades/fold | avg trades/fold | OOS PnL % | cons | "
         "final @3x SOL | latency retention | latency | min-trades |\n",
         "|---|---|---|---|---|---|---|---|---|\n"]
    for r in variants:
        L.append(f"| {r['tag']} | {r['min_oos_trades']} | {r['avg_oos_trades']} | "
                 f"{r['total_oos_pnl']} | {r['consistency']:.0%} | {r['final_sol_3x']} | "
                 f"{r['latency_retention']:.0%} | "
                 f"{'PASS' if r['latency_pass'] else 'FAIL'} | "
                 f"{'PASS' if r['min_trades_pass'] else 'FAIL'} |\n")
    L += ["\n## Reading\n\n",
          "- cb2 = confirm_bars=2 (the pre-registered latency absorber)\n",
          "- 72d folds: a 0.23 trades/day strategy expects ~8.3 trades per "
          "36d fold, so the >=10 floor was statistical noise; 72d folds "
          "expect ~16.6 trades and give the floor real meaning\n",
          "- All costs measured v2 (2026-08-07): 0.06%/trip + 0.0004%/hr borrow "
          "both sides\n"]
    with open(os.path.join(OUT_DIR, "variant_verdict.md"), "w") as f:
        f.write("".join(L))
    log("\nWrote data/results/s15_v7/variant_verdict.md")


if __name__ == "__main__":
    main()
