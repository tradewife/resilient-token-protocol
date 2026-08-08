"""
S15 v8 — Jupiter Perps fee re-check (2-year data).

Flash Trade is winding down (Aug 7, 2026). The friend's engine (v7e,
DEPLOYABLE 10/10 under measured Flash v2 fees of ~0.06%/trip) must be
re-validated on the next venue's measured costs before any capital decision.

Measured Jupiter Perps costs (2026-08-08, on-chain SOL custody
7xS2gz… + docs.jup.ag fee reference):
- Base fee: 6 bps open + 6 bps close = 0.12%/trip (both sides)
- Linear price impact: scalar 3.75e11 USD → ~0.0005%/trip at $100 notional
- Additive imbalance penalty: OI imbalance $7.98M > threshold $1.50M,
  feeFactor=1, exponent=1 → ~5.3 bps (0.053%) on the IMBALANCE-WORSENING
  side (longs today); capped at 32 bps/side. Applied conservatively to
  longs only, on every trip.
- Swap fee: shorts are USDC-collateral → SOL→USDC on entry and USDC→SOL
  on exit each incur the 10 bps non-stable swap fee = +0.20%/trip on
  shorts. Longs are SOL-collateral: no swap.
- Borrow: jump curve, current 0.00146%/hr at 8.9% utilization; modelled
  at 0.002%/hr (conservative — rate rises with utilization).

Side-aware trip costs (per trip, % of notional, before leverage):
  LONG : 0.12 + 0.053 + 0.001 (linear) ≈ 0.18%  → model 0.18%
  SHORT: 0.12 + 0.20  + 0.001          ≈ 0.33%  → model 0.33%
  + borrow 0.002%/hr on both sides (vs Flash v2's 0.0004%/hr).

This script re-runs the EXACT v7e gate suite (equal 36-day folds, blind
touch composite) with the Jupiter cost basis swapped in.
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

# Measured Jupiter Perps fee model (2026-08-08, see module docstring)
JUP_LONG_TRIP = 0.18     # % of notional per round trip (base + additive + linear)
JUP_SHORT_TRIP = 0.33    # % of notional per round trip (base + 2x swap + linear)
JUP_BORROW_HOURLY = 0.002  # % of notional per hour, both sides (conservative)

import research.missions.s15_v5_pipeline as pipe
import research.missions.s15_v5_finish as fin


def net_pnl_jupiter(trips, leverage=1.0, tf_minutes=15):
    """Jupiter cost basis: side-aware trip fee + 0.002%/hr borrow."""
    bph = pipe.bars_per_hour(tf_minutes)
    net_trips = []
    for t in trips:
        lev = max(leverage, 1.0)
        hold_hours = t.get("hold_bars", t.get("hold_hrs", 0)) / bph
        is_short = t.get("direction", 1) == -1
        trip_fee = JUP_SHORT_TRIP if is_short else JUP_LONG_TRIP
        borrow = JUP_BORROW_HOURLY * hold_hours
        fee = lev * (trip_fee + borrow)
        net = t["pnl_pct"] - fee
        nt = dict(t)
        nt["gross_pnl"] = t["pnl_pct"]
        nt["net_pnl"] = net
        nt["fee_pct"] = fee
        net_trips.append(nt)
    return net_trips


pipe.net_pnl = net_pnl_jupiter
fin.net_pnl = net_pnl_jupiter
fin.compound.__globals__["net_pnl"] = net_pnl_jupiter

sys.path.insert(0, os.path.join(ROOT, "research", "missions"))
import s15_v6_gap_close as v6

v6.net_pnl = net_pnl_jupiter

WARMUP = 250
FOLD_DAYS = 36


def equal_folds(df, tf):
    """Anchored walk-forward with EQUAL 36-day test windows (v7e method)."""
    from research.missions.s15_v5_pipeline import bars_per_hour
    w = int(FOLD_DAYS * 24 * bars_per_hour(tf))
    folds = []
    ts = WARMUP
    i = 0
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
    log(f"SOL 2yr: {len(df)} bars | {FOLD_DAYS}d equal folds | JUPITER fees")
    log(f"Fee basis: long {JUP_LONG_TRIP}%/trip, short {JUP_SHORT_TRIP}%/trip, "
        f"borrow {JUP_BORROW_HOURLY}%/hr")

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
    folds_df.to_csv(os.path.join(OUT_DIR, "folds_jupiter.csv"), index=False)
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
    lat5 = v6.compound(df, v6.TF, p_long, p_short, leverage=best_lev, delay_bars=1)
    denom = comp["final_sol"] - START_SOL
    retention = (lat5["final_sol"] - START_SOL) / denom if abs(denom) > 1e-9 else 1.0
    log(f"\n  COMP @ {best_lev}x: final={comp['final_sol']} ({comp['ret_pct']:+.1f}%) "
        f"dd={comp['max_dd_pct']}% liq={comp['liquidations']} halts={comp['halts']} "
        f"trades={comp['trades']} ({comp['trades_per_day']}/day) "
        f"latency retention={retention:.0%}")

    base_net, spread, flips, robust, _ = fin.sensitivity(
        df, v6.TF, p_long, use_composite=True, params2=p_short)
    log(f"  SENS: base={base_net:+.1f}% spread={spread:.1f} flips={flips} {robust}")

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
    log(f"\n--- GATES (Jupiter fees): {passed}/10 ---")
    for k, v in gates.items():
        log(f"  [{'PASS' if v else 'FAIL'}] {k}")

    pd.DataFrame([{"tag": "20m_blindtouch_jupiter_fees",
                   "gates_passed": f"{passed}/{len(gates)}",
                   **{k: bool(v) for k, v in gates.items()},
                   "num_folds": n, "min_oos_trades": min_tr,
                   "full_window_min_trades": full_min_trades,
                   "total_oos_pnl": round(total_pnl, 1),
                   "consistency": round(cons, 3),
                   "median_oos_sharpe": round(med_sh, 2),
                   "long_net": round(long_net, 1), "short_net": round(short_net, 1),
                   "best_leverage": best_lev, "final_sol": comp["final_sol"],
                   "dd": comp["max_dd_pct"],
                   "retention": round(retention, 3)}]
                 ).to_csv(os.path.join(OUT_DIR, "jupiter_gate_matrix.csv"), index=False)

    # v7e comparison line for the verdict
    log("\n--- v7e (Flash v2 fees) reference: 10/10 gates, OOS +49.8%, "
        "cons 67%, 2.5->3.19 SOL @5x, dd 20.56%, retention 105% ---")
    log("Done. Results in data/results/s15_v8_jupiter/")


if __name__ == "__main__":
    main()
