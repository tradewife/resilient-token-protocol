"""
S15 v8d — GMTrade fee variant.

Measured from official docs (docs.gmtrade.xyz, 2026-08-08):
- Open/close crypto: 0.010% (balance-improving) or 0.012% per side.
  Conservative: 0.012% both sides = 0.024%/trip.
- Price impact: bi-directional, shown pre-trade; at ~$100 notional it is
  negligible but modelled conservatively at +0.005%/trip.
- SOL-collateral LONGS confirmed: "Long SOL with SOL as collateral...
  profits paid in SOL" — the accumulation mechanic the client thesis
  requires. Shorts may also use SOL collateral (delta-neutral/funding
  strategies) but short profits pay in stablecoin per docs; modelled as
  native-collateral both sides, profit settlement as documented.
- Funding: adaptive, bi-directional (long-heavy -> longs pay). Strategy
  trades both directions; net ~neutral, modelled conservatively as cost.
- Borrowing fee: utilization-based, paid by the majority side.
- Liquidation fee 0.05% (only on liquidation — strategy had 0 liq in v7e).
- Total hourly: 0.0005%/hr both sides (same conservative basis as HL).
- Trip cost: 0.029%/trip both sides ≈ 0.03% (HALF of Flash v2's 0.06%).
"""
import os
import sys
import json
from datetime import datetime

import numpy as np
import pandas as pd

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT_DIR = os.path.join(ROOT, "data", "results", "s15_v8_gmtrade")
sys.path.insert(0, ROOT)

from research.missions.s15_v7_v2fee_recheck import log, START_SOL

GM_TRIP = 0.029        # % per round trip, both sides (0.012x2 + 0.005 impact)
GM_BORROW_HOURLY = 0.0005  # %/hr both sides, conservative

import research.missions.s15_v5_pipeline as pipe
import research.missions.s15_v5_finish as fin


def net_pnl_gm(trips, leverage=1.0, tf_minutes=15):
    bph = pipe.bars_per_hour(tf_minutes)
    out = []
    for t in trips:
        lev = max(leverage, 1.0)
        hold_hours = t.get("hold_bars", t.get("hold_hrs", 0)) / bph
        fee = lev * (GM_TRIP + GM_BORROW_HOURLY * hold_hours)
        nt = dict(t)
        nt["gross_pnl"] = t["pnl_pct"]
        nt["net_pnl"] = t["pnl_pct"] - fee
        nt["fee_pct"] = fee
        out.append(nt)
    return out


pipe.net_pnl = net_pnl_gm
fin.net_pnl = net_pnl_gm
fin.compound.__globals__["net_pnl"] = net_pnl_gm

sys.path.insert(0, os.path.join(ROOT, "research", "missions"))
import s15_v6_gap_close as v6

v6.net_pnl = net_pnl_gm

WARMUP = 250
FOLD_DAYS = 36


def equal_folds(df, tf):
    from research.missions.s15_v5_pipeline import bars_per_hour
    w = int(FOLD_DAYS * 24 * bars_per_hour(tf))
    folds, ts, i = [], WARMUP, 0
    while ts + w <= len(df):
        folds.append({"fold_num": i, "test_start": ts, "test_end": ts + w})
        ts += w
        i += 1
    if ts < len(df):
        folds.append({"fold_num": i, "test_start": ts, "test_end": len(df)})
    return folds


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    cfg = json.load(open(os.path.join(ROOT, "data", "results", "s15_v5",
                                      "winning_config.json")))
    p_long = {**cfg["params_long"], "confirm_mode": "none"}
    p_short = {**cfg["params_short"], "confirm_mode": "none"}
    df = v6.load_2yr("SOL/USDT")
    log(f"SOL 2yr: {len(df)} bars | equal {FOLD_DAYS}d folds | GMTrade fees")
    log(f"Fee basis: {GM_TRIP}%/trip both sides + {GM_BORROW_HOURLY}%/hr")

    from research.missions.s15_v5_pipeline import compute_metrics, bars_per_hour
    folds = equal_folds(df, v6.TF)
    rows = []
    for f in folds:
        test_df = df.iloc[f["test_start"]:f["test_end"]]
        trips = v6.run_composite(test_df, p_long, p_short, 1.0, v6.TF)
        net = v6.net_pnl(trips, 1.0, v6.TF)
        m = compute_metrics(net, total_hours=len(test_df) / bars_per_hour(v6.TF))
        rows.append({"fold": f["fold_num"], "oos_trades": m["round_trips"],
                     "oos_pnl": m["total_pnl_pct"], "oos_sharpe": m["sharpe"],
                     "long_net": sum(t["net_pnl"] for t in net if t["direction"] == 1),
                     "short_net": sum(t["net_pnl"] for t in net if t["direction"] == -1)})
    n = len(rows)
    sharpes = [max(-100, min(100, x["oos_sharpe"])) for x in rows]
    cons = sum(1 for s in sharpes if s > 0) / n
    total_pnl = sum(x["oos_pnl"] for x in rows)
    long_net = sum(x["long_net"] for x in rows)
    short_net = sum(x["short_net"] for x in rows)
    med_sh = float(np.median(sharpes))
    min_tr = int(min(x["oos_trades"] for x in rows))
    log(f"  WFA {n} folds: pnl={total_pnl:+.1f}% cons={cons:.0%} medsh={med_sh:.2f} "
        f"long={long_net:+.1f}% short={short_net:+.1f}%")

    best_lev, best_final = 1.0, -1e9
    for lev in [1.0, 2.0, 3.0, 5.0]:
        fold_pos = sum(1 for f in folds
                       if (lambda nt: nt and sum(t["net_pnl"] for t in nt) > 0)(
                           v6.net_pnl(v6.run_composite(
                               df.iloc[f["test_start"]:f["test_end"]],
                               p_long, p_short, lev, v6.TF), lev, v6.TF)))
        c = v6.compound(df, v6.TF, p_long, p_short, leverage=lev)
        passed = (c["max_dd_pct"] <= 25 and c["liquidations"] == 0 and
                  fold_pos / n >= 0.5 and c["final_sol"] > START_SOL)
        log(f"  LEV {lev}x: final={c['final_sol']} dd={c['max_dd_pct']}% "
            f"folds_pos={fold_pos}/{n} {'PASS' if passed else 'fail'}")
        if passed and c["final_sol"] > best_final:
            best_final, best_lev = c["final_sol"], lev

    comp = v6.compound(df, v6.TF, p_long, p_short, leverage=best_lev)
    lat = v6.compound(df, v6.TF, p_long, p_short, leverage=best_lev, delay_bars=1)
    denom = comp["final_sol"] - START_SOL
    retention = (lat["final_sol"] - START_SOL) / denom if abs(denom) > 1e-9 else 1.0
    log(f"  COMP @ {best_lev}x: final={comp['final_sol']} dd={comp['max_dd_pct']}% "
        f"liq={comp['liquidations']} halts={comp['halts']} retention={retention:.0%}")

    base_net, spread, flips, robust, _ = fin.sensitivity(
        df, v6.TF, p_long, use_composite=True, params2=p_short)
    log(f"  SENS: base={base_net:+.1f}% spread={spread:.1f} flips={flips} {robust}")

    w_bars = int(FOLD_DAYS * 24 * pipe.bars_per_hour(v6.TF))
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
    log(f"  --- GATES GMTrade: {passed}/10 ---")
    for k, v in gates.items():
        log(f"    [{'PASS' if v else 'FAIL'}] {k}")

    pd.DataFrame([{"tag": "20m_blindtouch_gmtrade_fees",
                   "gates_passed": f"{passed}/10",
                   **{k: bool(v) for k, v in gates.items()},
                   "total_oos_pnl": round(total_pnl, 1),
                   "consistency": round(cons, 3),
                   "median_oos_sharpe": round(med_sh, 2),
                   "long_net": round(long_net, 1), "short_net": round(short_net, 1),
                   "min_oos_trades": min_tr, "best_leverage": best_lev,
                   "final_sol": comp["final_sol"], "dd": comp["max_dd_pct"],
                   "retention": round(retention, 3)}]
                 ).to_csv(os.path.join(OUT_DIR, "gmtrade_gate_matrix.csv"), index=False)

    log("\nrefs: v7e Flash 0.06% = 10/10 +49.8% | v8b Jupiter hybrid = 6/10 +18.3%")


if __name__ == "__main__":
    main()
