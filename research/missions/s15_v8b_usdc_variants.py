"""
S15 v8b — Jupiter USDC-accounting variants.

Grok hypothesis under test (2026-08-08): "2.69 was not tested on a SOL
model; pure USDC trading would almost certainly perform better on Jupiter."

Facts from measured Jupiter mechanics (2026-08-08, SOL custody on-chain):
- SOL LONG native collateral is SOL. USDC-funded longs swap USDC->SOL on
  entry and SOL->USDC on exit: +2x 10bps swap fee.
- SOL SHORT native collateral is USDC: no swap fee under USDC accounting.
- Additive imbalance penalty: OI imbalance is LONG-heavy ($26.4M vs
  $18.4M; threshold $1.5M, factor=1, exp=1, cap 32bps). Longs worsen it
  -> pay ~5.3bps; shorts reduce it -> pay ZERO until crossover.
- Base fee 6bps each side; borrow 0.002%/hr conservative.

Variants:
  A "hybrid_native"  — SOL-collateral longs (0.18%/trip) + USDC shorts
                        (0.13%/trip). This is what Jupiter natively offers.
  B "pure_usdc"      — everything funded in USDC: longs pay entry+exit
                        swaps (0.12 + 0.20 + 0.053 = 0.373%/trip), shorts
                        unchanged (0.13%/trip). Grok's proposal.
Reference: v7e Flash v2 = 0.06%/trip both sides (10/10 gates, +49.8%).
"""
import os
import sys
import json
from datetime import datetime

import numpy as np
import pandas as pd

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT_DIR = os.path.join(ROOT, "data", "results", "s15_v8_jupiter")
sys.path.insert(0, ROOT)

from research.missions.s15_v7_v2fee_recheck import log, START_SOL

JUP_BORROW_HOURLY = 0.002  # %/hr both sides, conservative

VARIANTS = {
    "A_hybrid_native": {"long_trip": 0.180, "short_trip": 0.130},
    "B_pure_usdc":     {"long_trip": 0.373, "short_trip": 0.130},
}

import research.missions.s15_v5_pipeline as pipe
import research.missions.s15_v5_finish as fin


def make_net_pnl(long_trip, short_trip):
    def net_pnl_variant(trips, leverage=1.0, tf_minutes=15):
        bph = pipe.bars_per_hour(tf_minutes)
        out = []
        for t in trips:
            lev = max(leverage, 1.0)
            hold_hours = t.get("hold_bars", t.get("hold_hrs", 0)) / bph
            trip = short_trip if t.get("direction", 1) == -1 else long_trip
            fee = lev * (trip + JUP_BORROW_HOURLY * hold_hours)
            nt = dict(t)
            nt["gross_pnl"] = t["pnl_pct"]
            nt["net_pnl"] = t["pnl_pct"] - fee
            nt["fee_pct"] = fee
            out.append(nt)
        return out
    return net_pnl_variant


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


def run_variant(tag, df, p_long, p_short, long_trip, short_trip):
    import s15_v6_gap_close as v6
    from research.missions.s15_v5_pipeline import compute_metrics, bars_per_hour

    npnl = make_net_pnl(long_trip, short_trip)
    pipe.net_pnl = npnl
    fin.net_pnl = npnl
    fin.compound.__globals__["net_pnl"] = npnl
    v6.net_pnl = npnl

    log(f"\n========== VARIANT {tag}: long {long_trip}%/trip, "
        f"short {short_trip}%/trip ==========")
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
    log(f"  WFA {n} folds: pnl={total_pnl:+.1f}% cons={cons:.0%} "
        f"medsh={med_sh:.2f} long={long_net:+.1f}% short={short_net:+.1f}%")

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
    log(f"  --- GATES {tag}: {passed}/10 ---")
    for k, v in gates.items():
        log(f"    [{'PASS' if v else 'FAIL'}] {k}")

    return {"tag": tag, "gates_passed": f"{passed}/10",
            **{k: bool(v) for k, v in gates.items()},
            "total_oos_pnl": round(total_pnl, 1),
            "consistency": round(cons, 3),
            "median_oos_sharpe": round(med_sh, 2),
            "long_net": round(long_net, 1), "short_net": round(short_net, 1),
            "min_oos_trades": min_tr,
            "best_leverage": best_lev, "final_sol": comp["final_sol"],
            "dd": comp["max_dd_pct"], "retention": round(retention, 3),
            "long_trip": long_trip, "short_trip": short_trip}


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    cfg = json.load(open(os.path.join(ROOT, "data", "results", "s15_v5",
                                      "winning_config.json")))
    p_long = {**cfg["params_long"], "confirm_mode": "none"}
    p_short = {**cfg["params_short"], "confirm_mode": "none"}
    import s15_v6_gap_close as v6
    df = v6.load_2yr("SOL/USDT")
    log(f"SOL 2yr: {len(df)} bars | equal {FOLD_DAYS}d folds | borrow "
        f"{JUP_BORROW_HOURLY}%/hr")

    results = []
    for tag, v in VARIANTS.items():
        results.append(run_variant(tag, df, p_long, p_short,
                                   v["long_trip"], v["short_trip"]))

    pd.DataFrame(results).to_csv(
        os.path.join(OUT_DIR, "usdc_variant_matrix.csv"), index=False)

    log("\n--- SUMMARY ---")
    log("ref v7e Flash 0.06%/trip:      10/10 | +49.8% | long +12.0 short +37.8")
    log("ref v8  Flash-style SOL acct:   4/10 | -28.5% | long  -1.0 short -27.5")
    for r in results:
        log(f"    {r['tag']}: {r['gates_passed']} | OOS {r['total_oos_pnl']:+.1f}% | "
            f"long {r['long_net']:+.1f} short {r['short_net']:+.1f} | "
            f"final {r['final_sol']} SOL @{r['best_leverage']}x dd {r['dd']}%")


if __name__ == "__main__":
    main()
