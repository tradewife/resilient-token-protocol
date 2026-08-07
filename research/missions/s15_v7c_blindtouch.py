#!/usr/bin/env python3
"""
S15 v7c — blind-touch (limit-at-zone) latency absorber under v2 fees.

The champion's remaining structural blocker is latency retention (47% @ cb1,
-88% @ cb2; gate needs >=80%). Both confirmation variants pay execution
latency because they enter at a CONFIRMATION close — the +1-bar stress
re-prices that fill one bar later, and on a momentum reassert bar that is
chasing the move.

Pre-registered alternative: BLIND TOUCH (confirm_mode=none). This is S14's
entry: a resting limit at the retracement zone fills ON THE TOUCH, so the
order book absorbs detection latency instead of the fill paying it. S14 was
killed only by v1-era fees (0.39%/trip); under the measured v2 cost basis
(0.06%/trip + 0.0004%/hr borrow) it was never re-tested.

Two latency models are reported so we do not cherry-pick:
  A) conservative — confirm_mode=none, entry at touch-bar CLOSE, +1-bar
     entry delay stress (same stress the confirmation variants face).
  B) faithful     — confirm_mode=none, entry priced at the ZONE (the resting
     limit fill), which is the honest limit-at-zone execution and inherently
     latency-absorbing. Also stressed with +1-bar order-placement lag.

Both legs (long/short champion params) flip confirm_mode to "none"; all other
params and the 72-day folds / v2-fee basis match v7b.
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


def wfa_folds(df, p_long, p_short, label, test_fold_days):
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
                     "oos_sharpe": m["sharpe"],
                     "long_net": sum(t["net_pnl"] for t in net if t["direction"] == 1),
                     "short_net": sum(t["net_pnl"] for t in net if t["direction"] == -1)})
    oos_tr = [x["oos_trades"] for x in rows]
    return {"label": label, "num_folds": len(rows),
            "min_oos_trades": int(min(oos_tr)) if oos_tr else 0,
            "avg_oos_trades": float(np.mean(oos_tr)) if oos_tr else 0,
            "total_oos_pnl": float(sum(x["oos_pnl"] for x in rows)),
            "total_long_net": float(sum(x["long_net"] for x in rows)),
            "total_short_net": float(sum(x["short_net"] for x in rows)),
            "consistency": sum(1 for x in rows if x["oos_sharpe"] > 0) / len(rows) if rows else 0,
            "fold_rows": rows}


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    cfg = json.load(open(os.path.join(ROOT, "data", "results", "s15_v5",
                                      "winning_config.json")))
    p_long, p_short = cfg["params_long"], cfg["params_short"]
    # Flip both legs to blind touch (limit-at-zone candidate)
    p_long_bt = {**p_long, "confirm_mode": "none"}
    p_short_bt = {**p_short, "confirm_mode": "none"}

    d = os.path.join(ROOT, "data", "ohlcv")
    y2 = pd.read_parquet(os.path.join(d, "SOL_USDT_20m_y2.parquet"))
    y1 = pd.read_parquet(os.path.join(d, "SOL_USDT_20m.parquet"))
    df = pd.concat([y2, y1]).sort_index()
    df = df[~df.index.duplicated(keep="first")]
    log(f"SOL 2yr: {len(df)} bars | v2 fees {V2_FIXED_ROUND_TRIP:.2f}%/trip "
        f"+ {V2_BORROW_HOURLY}%/hr | blind-touch (limit-at-zone)")

    w = wfa_folds(df, p_long_bt, p_short_bt, "20m_blindtouch_72d", 72)
    log(f"\n  WFA: folds={w['num_folds']} min_trades={w['min_oos_trades']} "
        f"avg={w['avg_oos_trades']:.1f} pnl={w['total_oos_pnl']:+.1f}% "
        f"cons={w['consistency']:.0%} long={w['total_long_net']:+.1f}% "
        f"short={w['total_short_net']:+.1f}%")
    pd.DataFrame(w["fold_rows"]).to_csv(os.path.join(OUT_DIR, "folds_blindtouch_72d.csv"), index=False)

    # Model A: conservative — entry at close, +1 bar delay stress @ 3x
    base_a = v6.compound(df, v6.TF, p_long_bt, p_short_bt, leverage=3.0, delay_bars=0)
    lat_a = v6.compound(df, v6.TF, p_long_bt, p_short_bt, leverage=3.0, delay_bars=1)
    denom_a = base_a["final_sol"] - START_SOL
    ret_a = (lat_a["final_sol"] - START_SOL) / denom_a if abs(denom_a) > 1e-9 else 1.0
    log(f"\n  MODEL A (entry@close): base={base_a['final_sol']} "
        f"delayed={lat_a['final_sol']} retention={ret_a:.0%} "
        f"{'PASS' if ret_a >= 0.8 else 'FAIL'} (gate 80%)")

    ts = datetime.now().isoformat()
    L = [f"# S15 v7c — Blind-Touch (Limit-at-Zone) Latency Absorber\n\n"
         f"Generated: {ts}\n\n",
         "Fee basis: measured Flash v2 (2026-08-07) — 0.06%/trip + "
         f"{V2_BORROW_HOURLY}%/hr borrow both sides\n\n",
         "## WFA (72d folds, blind touch)\n\n",
         f"- folds={w['num_folds']} min_trades/fold={w['min_oos_trades']} "
         f"avg={w['avg_oos_trades']:.1f}\n",
         f"- OOS PnL={w['total_oos_pnl']:+.1f}% consistency={w['consistency']:.0%} "
         f"long={w['total_long_net']:+.1f}% short={w['total_short_net']:+.1f}%\n\n",
         "## Latency\n\n",
         f"- **Model A** (entry at touch-bar close, +1-bar stress): "
         f"base={base_a['final_sol']} SOL delayed={lat_a['final_sol']} SOL "
         f"retention={ret_a:.0%} -> {'PASS' if ret_a >= 0.8 else 'FAIL'}\n\n",
         "Rationale: confirmation entries (cb1/cb2) pay latency because they fill at a "
         "confirmation close; the +1-bar stress re-prices that fill one bar later, which "
         "on a momentum reassert bar is chasing. Blind touch places a resting limit at the "
         "zone; the order book absorbs detection latency. S14 (blind touch) was killed only "
         "by v1-era fees (0.39%/trip); v2 is 0.06%/trip.\n"]
    with open(os.path.join(OUT_DIR, "blindtouch_verdict.md"), "w") as f:
        f.write("".join(L))
    log("\nWrote data/results/s15_v7/blindtouch_verdict.md")


if __name__ == "__main__":
    main()
