"""
S15 v8e — GMTrade MEASURED cost basis (supersedes v8d docs-basis).

Measured on-chain via gmsol-sdk mainnet probe (2026-08-08), program
Gmso1uvJnLbawvw7yezdfCDcPydwW2s2iqG3w6MDucLo (open-source gmx-solana):

Target market: SOL/USD[WSOL-USDC] (3M4vW1u8…) — SOL-collateral longs,
profits paid in SOL (RB accumulation mechanic). OI $47,990 long / $2,000
short; pool $67,960 WSOL / $5.3M USDC.

- Order fees: 0.010% (balance-improving) / 0.012% per side, of position
  size USD. Conservative: 0.012% both sides + 0.005% impact buffer.
- skip_borrow_for_smaller_side = TRUE (market flag raw=1): the minority
  OI side pays ZERO borrowing fee. Currently shorts are minority -> 0.
- Longs (majority): borrow = usage x base_factor. Measured usage 0.706,
  base 1.43e-8/s -> 0.0036%/hr of POSITION SIZE. Kink-max (above optimal
  usage 0.75): 3.17e-8/s = 0.0114%/hr.
- Funding: adaptive, cap 2.378e-8/s = 0.0036%/hr; cumulative long/short
  funding on this market tiny (0.00028 / 0.00043). Modelled 0.0005%/hr
  both sides (conservative floor).
- Liquidation fee 0.05% (strategy had 0 liquidations in v7e).
- Borrow applies to position size (leverage-inclusive), charged to the
  collateral token. As % of collateral = leverage x rate x hours.

Variants:
  E1 measured_now   : long borrow 0.0036%/hr, short 0
  E2 stress_kink    : long borrow 0.0114%/hr (kink max), short 0
  E3 low_usage      : long borrow 0.0020%/hr (usage ~0.4), short 0
All: trip 0.029% (0.024 fees + 0.005 impact buffer), both sides, x lev.
"""
import os
import sys
import json

import numpy as np
import pandas as pd

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT_DIR = os.path.join(ROOT, "data", "results", "s15_v8_gmtrade")
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "research", "missions"))

from research.missions.s15_v7_v2fee_recheck import log, START_SOL

TRIP = 0.029  # % per round trip both sides (fees + impact buffer)
FUNDING_HOURLY = 0.0005  # %/hr both sides (conservative floor vs 0.0036 cap)

VARIANTS = {
    "E1_measured_now": {"long_borrow": 0.0036, "short_borrow": 0.0},
    "E2_stress_kink":  {"long_borrow": 0.0114, "short_borrow": 0.0},
    "E3_low_usage":    {"long_borrow": 0.0020, "short_borrow": 0.0},
}

import research.missions.s15_v5_pipeline as pipe
import research.missions.s15_v5_finish as fin


def make_net_pnl(long_borrow, short_borrow):
    def net_pnl_measured(trips, leverage=1.0, tf_minutes=15):
        bph = pipe.bars_per_hour(tf_minutes)
        out = []
        for t in trips:
            lev = max(leverage, 1.0)
            hold_hours = t.get("hold_bars", t.get("hold_hrs", 0)) / bph
            is_long = t.get("direction", 1) == 1
            borrow = long_borrow if is_long else short_borrow
            fee = lev * (TRIP + FUNDING_HOURLY * hold_hours + borrow * hold_hours)
            nt = dict(t)
            nt["gross_pnl"] = t["pnl_pct"]
            nt["net_pnl"] = t["pnl_pct"] - fee
            nt["fee_pct"] = fee
            out.append(nt)
        return out
    return net_pnl_measured


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


def run_variant(tag, df, p_long, p_short, long_borrow, short_borrow):
    import s15_v6_gap_close as v6
    from research.missions.s15_v5_pipeline import compute_metrics, bars_per_hour

    npnl = make_net_pnl(long_borrow, short_borrow)
    pipe.net_pnl = npnl
    fin.net_pnl = npnl
    fin.compound.__globals__["net_pnl"] = npnl
    v6.net_pnl = npnl

    log(f"\n========== VARIANT {tag}: long {long_borrow}%/hr short {short_borrow}%/hr ==========")
    folds = equal_folds(df, v6.TF)
    rows = []
    all_trips = []
    for f in folds:
        test_df = df.iloc[f["test_start"]:f["test_end"]]
        trips = v6.run_composite(test_df, p_long, p_short, 1.0, v6.TF)
        net = v6.net_pnl(trips, 1.0, v6.TF)
        all_trips.extend(net)
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
    longs = [t for t in all_trips if t["direction"] == 1]
    shorts = [t for t in all_trips if t["direction"] == -1]
    if longs:
        avg_hold = np.mean([t.get("hold_bars", t.get("hold_hrs", 0)) for t in longs]) / pipe.bars_per_hour(v6.TF)
        log(f"  avg long hold {avg_hold:.1f}h -> borrow/trip @{5}x ≈ {5*long_borrow*avg_hold:.2f}% of collateral")
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
    log(f"  --- GATES {tag}: {passed}/10 ---")
    for k, v in gates.items():
        log(f"    [{'PASS' if v else 'FAIL'}] {k}")

    return {"tag": tag, "gates_passed": f"{passed}/10",
            **{k: bool(v) for k, v in gates.items()},
            "total_oos_pnl": round(total_pnl, 1),
            "consistency": round(cons, 3),
            "median_oos_sharpe": round(med_sh, 2),
            "long_net": round(long_net, 1), "short_net": round(short_net, 1),
            "min_oos_trades": min_tr, "best_leverage": best_lev,
            "final_sol": comp["final_sol"], "dd": comp["max_dd_pct"],
            "retention": round(retention, 3),
            "long_borrow_hr": long_borrow, "short_borrow_hr": short_borrow}


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    cfg = json.load(open(os.path.join(ROOT, "data", "results", "s15_v5",
                                      "winning_config.json")))
    p_long = {**cfg["params_long"], "confirm_mode": "none"}
    p_short = {**cfg["params_short"], "confirm_mode": "none"}
    import s15_v6_gap_close as v6
    df = v6.load_2yr("SOL/USDT")
    log(f"SOL 2yr: {len(df)} bars | equal {FOLD_DAYS}d folds | GMTrade MEASURED costs")
    log(f"Trip: {TRIP}%/trip both sides x lev | funding {FUNDING_HOURLY}%/hr both sides")

    results = []
    for tag, v in VARIANTS.items():
        results.append(run_variant(tag, df, p_long, p_short,
                                   v["long_borrow"], v["short_borrow"]))

    pd.DataFrame(results).to_csv(
        os.path.join(OUT_DIR, "gmtrade_measured_matrix.csv"), index=False)

    log("\n--- SUMMARY (measured-cost basis) ---")
    log("ref v8d docs-basis 0.0005%/hr both sides: 10/10 | +59.9% | 3.63 SOL @5x")
    for r in results:
        log(f"    {r['tag']}: {r['gates_passed']} | OOS {r['total_oos_pnl']:+.1f}% | "
            f"long {r['long_net']:+.1f} short {r['short_net']:+.1f} | "
            f"final {r['final_sol']} @{r['best_leverage']}x dd {r['dd']}%")


if __name__ == "__main__":
    main()
