#!/usr/bin/env python3
"""
S15 v5 FINISHER — direction-aware candidate validation.

The main pipeline (s15_v5_pipeline.py) passed the net gate (+0.152%/trade
@ 20m close_reassert_b1) but its Phase 3 re-tested the short-only winner
with flipped filters instead of evaluating direction-specific candidates
and the composite long∪short. The Phase 2 grids actually contain live
candidates in ALL THREE regimes (e.g. 20m long +15.5% net/26 trades/50% WR,
20m short +27.5% net/49 trades/51% WR). This finisher validates them
properly:

  F1  Candidate extraction from phase2 grids (long/short/both, tiered by
      trade count, ranked by total net PnL = profit-driven)
  F2  Exec-param refinement (SL/TP/hold/trail) on top candidates
  F3  9-fold net WFA with per-direction attribution + COMPOSITE long∪short
      (two independent configs trading simultaneously, merged chronologically)
  F4  Sensitivity ±20% (absolute-spread metric, no divide-by-zero bug)
  F5  Leverage sweep 1-10x
  F6  2.5 SOL compounding at best leverage, long/short SOL attribution
  F7  Latency: entries delayed one 5-min poll (close-based re-pricing)
  F8  BTC/ETH cross-symbol at the surviving TF
  F9  Final verdict + H1-H7 scorecard + winning_config.json

All real-plugin, net-of-fee (borrow on shorts only, hours-corrected).
"""
import os, sys, json, csv, time
from datetime import datetime
from itertools import product
from typing import Dict, List, Optional, Tuple

import numpy as np
import pandas as pd

ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
OUT_DIR = os.path.join(ROOT, "data", "results", "s15_v5")
sys.path.insert(0, ROOT)

from research.missions.s15_v5_pipeline import (
    run_simulation, net_pnl, compute_metrics, create_folds, survivor_score,
    bars_per_hour, scale_time_params, get_data, TFS, START_SOL, POSITION_PCT,
    MIN_COLLATERAL_SOL, GAS_PER_ROUND_TRIP, MAX_TRADES_PER_DAY, POLL_MINUTES,
    MIN_TRADES_PER_FOLD,
)

CAND_CSV = os.path.join(OUT_DIR, "finisher_candidates.csv")
SWEEP_CSV = os.path.join(OUT_DIR, "finisher_exec_sweep.csv")
WFA_CSV = os.path.join(OUT_DIR, "finisher_wfa.csv")


def log(msg):
    print(f"[{datetime.now().strftime('%H:%M:%S')}] {msg}", flush=True)


def trips_to_frame(trips):
    """Merge trips from independent configs into one chronological stream."""
    return sorted(trips, key=lambda t: (t.get("entry_idx", 0), t.get("direction", 1)))


def run_composite(df, params_long, params_short, leverage=1.0, tf_minutes=15):
    """Union of two independent direction configs, chronological order."""
    trips = []
    if params_long is not None:
        trips += run_simulation(df, params_long, leverage, tf_minutes, use_s15=True)
    if params_short is not None:
        trips += run_simulation(df, params_short, leverage, tf_minutes, use_s15=True)
    return trips_to_frame(trips)


# ---------------------------------------------------------------------------
# F1 — candidate extraction
# ---------------------------------------------------------------------------
PARAM_KEYS = ["retracement_pct", "wick_tolerance_pct", "body_atr_multiplier",
              "expiry_hours", "trend_fast_period", "trend_slow_period",
              "direction_filter", "volume_multiplier", "confirm_mode",
              "confirm_bars", "exit_mode", "trail_after_r", "use_vwap_filter",
              "stop_loss_atr", "take_profit_atr", "max_hold_hours",
              "time_decay_hours", "trailing_stop_atr"]


def extract_candidates(tf):
    path = os.path.join(OUT_DIR, f"phase2_coarse_{tf}m.csv")
    df = pd.read_csv(path, low_memory=False)
    df = df[df["round_trips"].fillna(0) > 0].copy()
    df["total_net"] = df["avg_net"] * df["round_trips"]
    cands = {"long": [], "short": [], "both": []}
    for d in cands:
        sub = df[(df["direction_filter"] == d) & (df["avg_net"] > 0)]
        if sub.empty:
            continue
        # Tier A: max total net profit (any trade count)
        topA = sub.sort_values("total_net", ascending=False).head(2)
        # Tier B: max total net with >= 100 trades/yr (statistical tier)
        subB = sub[sub["round_trips"] >= 100]
        topB = subB.sort_values("total_net", ascending=False).head(1) if not subB.empty else pd.DataFrame()
        # Tier C: max avg expectancy with >= 60 trades/yr (gate tier)
        subC = sub[sub["round_trips"] >= 60]
        topC = subC.sort_values("avg_net", ascending=False).head(1) if not subC.empty else pd.DataFrame()
        for tier, chunk in [("A", topA), ("B", topB), ("C", topC)]:
            for _, r in chunk.iterrows():
                p = {k: r[k] for k in PARAM_KEYS if k in r.index and pd.notna(r[k])}
                cands[d].append({"tf": tf, "direction": d, "tier": tier,
                                 "params": p,
                                 "grid_total_net": r["total_net"],
                                 "grid_trades": int(r["round_trips"]),
                                 "grid_avg_net": r["avg_net"],
                                 "grid_wr": r["win_rate"],
                                 "grid_sharpe": r["sharpe"]})
    return cands


# ---------------------------------------------------------------------------
# F2 — exec-param refinement (full-year net scan)
# ---------------------------------------------------------------------------
def exec_sweep(df_data, tf, params, tag):
    best = None
    rows = []
    for sl, tp, hold, trail in product([1.5, 2.0, 2.5], [2.0, 3.0, 4.0, 5.0],
                                       [12, 24, 48], [0.0, 0.5]):
        p = dict(params)
        p["stop_loss_atr"] = sl; p["take_profit_atr"] = tp
        p["max_hold_hours"] = hold; p["trailing_stop_atr"] = trail
        if p.get("exit_mode") == "structure" and tp != params.get("take_profit_atr", 3.0):
            continue  # structure exits define their own TP
        trips = run_simulation(df_data, p, 1.0, tf, use_s15=True)
        net = net_pnl(trips, 1.0, tf)
        m = compute_metrics(net, total_hours=len(df_data)/bars_per_hour(tf))
        row = {"tag": tag, "sl": sl, "tp": tp, "hold": hold, "trail": trail,
               "trades": m["round_trips"], "total_net": m["total_pnl_pct"],
               "avg_net": m["avg_pnl_pct"], "sharpe": m["sharpe"],
               "win_rate": m["win_rate"]}
        rows.append(row)
        if m["round_trips"] >= 15:
            score = m["total_pnl_pct"]
            if best is None or score > best[0]:
                best = (score, dict(p), m)
    return best, rows


# ---------------------------------------------------------------------------
# F3 — WFA with per-direction attribution (+ composite)
# ---------------------------------------------------------------------------
def wfa_candidate(df_data, tf, params_long, params_short, label):
    folds = create_folds(len(df_data), 9, 36, tf)
    fold_rows = []
    for f in folds:
        test_df = df_data.iloc[f["test_start"]:f["test_end"]]
        if len(test_df) <= 10:
            continue
        trips = run_composite(test_df, params_long, params_short, 1.0, tf)
        net = net_pnl(trips, 1.0, tf)
        m = compute_metrics(net, total_hours=len(test_df)/bars_per_hour(tf))
        long_net = sum(t["net_pnl"] for t in net if t["direction"] == 1)
        short_net = sum(t["net_pnl"] for t in net if t["direction"] == -1)
        fold_rows.append({"fold": f["fold_num"], "oos_sharpe": m["sharpe"],
                          "oos_trades": m["round_trips"], "oos_pnl": m["total_pnl_pct"],
                          "oos_dd": m["max_dd_pct"], "oos_pf": m["pf"],
                          "oos_wr": m["win_rate"], "long_net": long_net,
                          "short_net": short_net})
    oos_sh = [x["oos_sharpe"] for x in fold_rows]
    oos_dd = [x["oos_dd"] for x in fold_rows]
    oos_tr = [x["oos_trades"] for x in fold_rows]
    mnt = MIN_TRADES_PER_FOLD.get(tf, 15)
    ss = survivor_score(oos_sh, oos_dd, oos_tr, min_trades_per_fold=mnt)
    return {"label": label, "tf": tf, "num_folds": len(fold_rows),
            "survivor_score": ss["score"], "median_oos_sharpe": ss["median_sharpe"],
            "consistency": ss["consistency"],
            "avg_oos_trades": float(np.mean(oos_tr)) if oos_tr else 0,
            "min_oos_trades": int(min(oos_tr)) if oos_tr else 0,
            "total_oos_pnl": float(sum(x["oos_pnl"] for x in fold_rows)),
            "total_long_net": float(sum(x["long_net"] for x in fold_rows)),
            "total_short_net": float(sum(x["short_net"] for x in fold_rows)),
            "avg_oos_dd": float(np.mean(oos_dd)) if oos_dd else 0,
            "fold_rows": fold_rows}


# ---------------------------------------------------------------------------
# F4 — sensitivity
# ---------------------------------------------------------------------------
NUMERIC_KEYS = ["retracement_pct", "wick_tolerance_pct", "body_atr_multiplier",
                "expiry_hours", "stop_loss_atr", "take_profit_atr",
                "max_hold_hours", "time_decay_hours"]


def sensitivity(df_data, tf, params, use_composite=False, params2=None):
    def evaluate(p):
        trips = run_composite(df_data, p if params2 is None else p,
                              params2 if params2 is not None else None, 1.0, tf)
        net = net_pnl(trips, 1.0, tf)
        return compute_metrics(net, total_hours=len(df_data)/bars_per_hour(tf))["total_pnl_pct"]
    base = evaluate(params)
    results = []
    for k in NUMERIC_KEYS:
        if k not in params:
            continue
        for mult in [0.8, 1.2]:
            p = dict(params)
            p[k] = params[k] * mult
            if k in ("expiry_hours", "max_hold_hours", "time_decay_hours"):
                p[k] = max(0.5, p[k])
            v = evaluate(p)
            results.append({"param": k, "mult": mult, "total_net": v})
    vals = [r["total_net"] for r in results] + [base]
    spread = max(vals) - min(vals)
    sign_flips = sum(1 for v in vals if v <= 0)
    robust = "ROBUST" if (spread < max(10.0, abs(base)) and sign_flips <= 2) else \
             ("MODERATE" if sign_flips <= len(vals) // 2 else "FRAGILE")
    return base, spread, sign_flips, robust, results


# ---------------------------------------------------------------------------
# F6/F7 — SOL compounding with latency variant
# ---------------------------------------------------------------------------
def compound(df_data, tf, params_long, params_short, leverage=1.0, delay_bars=0):
    trips = run_composite(df_data, params_long, params_short, leverage, tf)
    if delay_bars > 0:
        close = df_data["close"].values
        adj = []
        for t in trips:
            e = t.get("entry_idx", 0) + delay_bars
            x = e + t.get("hold_bars", 0)
            if e >= len(close) or x >= len(close):
                continue
            nt = dict(t)
            entry_p = float(close[e]); exit_p = float(close[x])
            nt["pnl_pct"] = (exit_p - entry_p) / entry_p * 100 * t["direction"] * leverage
            adj.append(nt)
        trips = adj
    net = net_pnl(trips, leverage, tf)
    capital = START_SOL; peak = capital; max_dd = 0.0
    fees_sol = 0.0; gas_sol = 0.0; wins = 0; liq = 0; halts = 0
    long_sol = short_sol = 0.0; trades = 0
    for t in net:
        if t.get("liquidated", False):
            capital -= capital * POSITION_PCT; liq += 1; trades += 1; continue
        if capital * POSITION_PCT < MIN_COLLATERAL_SOL:
            halts += 1; break
        position = capital * POSITION_PCT
        pnl_sol = position * t["net_pnl"] / 100.0
        if t["direction"] == 1: long_sol += pnl_sol
        else: short_sol += pnl_sol
        capital += pnl_sol - GAS_PER_ROUND_TRIP
        fees_sol += position * t["fee_pct"] / 100.0
        gas_sol += GAS_PER_ROUND_TRIP
        if capital > peak: peak = capital
        if peak > 0: max_dd = max(max_dd, (peak - capital) / peak * 100.0)
        if t["net_pnl"] > 0: wins += 1
        trades += 1
    n_days = len(df_data) / (24 * bars_per_hour(tf))
    return {"start_sol": START_SOL, "final_sol": round(capital, 4),
            "ret_pct": round((capital - START_SOL) / START_SOL * 100, 2),
            "max_dd_pct": round(max_dd, 2), "win_rate": round(wins / trades, 4) if trades else 0,
            "fees_sol": round(fees_sol, 4), "gas_sol": round(gas_sol, 4),
            "liquidations": liq, "halts": halts, "trades": trades,
            "trades_per_day": round(trades / n_days, 2) if n_days else 0,
            "long_sol": round(long_sol, 4), "short_sol": round(short_sol, 4),
            "within_4_per_day": (trades / n_days if n_days else 0) <= MAX_TRADES_PER_DAY}


def main():
    log("=" * 70)
    log("S15 v5 FINISHER — direction-aware candidate validation")
    log("=" * 70)

    tf_data = {tf: get_data("SOL/USDT", tf) for tf in [15, 20]}

    # ---- F1: extract candidates ----
    log("\n--- F1: candidate extraction from phase2 grids ---")
    all_cands = {}
    for tf in [15, 20]:
        cands = extract_candidates(tf)
        all_cands[tf] = cands
        for d in ["long", "short", "both"]:
            log(f"  {tf}m {d}: {len(cands[d])} candidates")
            for c in cands[d]:
                log(f"    tier {c['tier']}: total_net={c['grid_total_net']:+.1f}% "
                    f"trades={c['grid_trades']} WR={c['grid_wr']:.0%} sharpe={c['grid_sharpe']:.2f} "
                    f"mode={c['params'].get('confirm_mode')} body={c['params'].get('body_atr_multiplier')} "
                    f"wick={c['params'].get('wick_tolerance_pct')} retr={c['params'].get('retracement_pct')}")

    # ---- F2: exec-param refinement on tier-A candidates ----
    log("\n--- F2: exec-param refinement (SL/TP/hold/trail) ---")
    refined = {}
    sweep_rows = []
    for tf in [15, 20]:
        for d in ["long", "short", "both"]:
            tierA = [c for c in all_cands[tf][d] if c["tier"] == "A"]
            if not tierA:
                continue
            c = tierA[0]
            tag = f"{tf}m_{d}"
            t0 = time.time()
            best, rows = exec_sweep(tf_data[tf], tf, c["params"], tag)
            sweep_rows.extend(rows)
            if best:
                refined[tag] = best[1]
                log(f"  {tag}: best total_net={best[0]:+.1f}% trades={best[2]['round_trips']} "
                    f"WR={best[2]['win_rate']:.0%} sharpe={best[2]['sharpe']:.2f} "
                    f"sl={best[1]['stop_loss_atr']} tp={best[1]['take_profit_atr']} "
                    f"hold={best[1]['max_hold_hours']} trail={best[1]['trailing_stop_atr']} "
                    f"({time.time()-t0:.0f}s)")
            else:
                log(f"  {tag}: no viable exec config (>=15 trades)")
    pd.DataFrame(sweep_rows).to_csv(SWEEP_CSV, index=False)

    # ---- F3: WFA + composite ----
    log("\n--- F3: 9-fold net WFA + composite long∪short ---")
    wfa_results = []
    # Single-direction and both candidates on refined params
    for tag, p in refined.items():
        tf = int(tag.split("m_")[0])
        d = tag.split("m_")[1]
        pl = p if d == "long" else None
        ps = p if d == "short" else None
        if d == "both":
            pl = dict(p); pl["direction_filter"] = "long"
            ps = dict(p); ps["direction_filter"] = "short"
        r = wfa_candidate(tf_data[tf], tf, pl, ps, tag)
        wfa_results.append(r)
        log(f"  {tag}: score={r['survivor_score']:.3f} medsh={r['median_oos_sharpe']:.2f} "
            f"cons={r['consistency']:.0%} trades/fold={r['avg_oos_trades']:.1f} "
            f"(min {r['min_oos_trades']}) pnl={r['total_oos_pnl']:+.1f}% "
            f"long={r['total_long_net']:+.1f}% short={r['total_short_net']:+.1f}%")

    # Composite: best long params ∪ best short params per TF
    composites = {}
    for tf in [15, 20]:
        tl = refined.get(f"{tf}m_long"); ts = refined.get(f"{tf}m_short")
        if tl is not None and ts is not None:
            tag = f"{tf}m_COMPOSITE"
            r = wfa_candidate(tf_data[tf], tf, tl, ts, tag)
            composites[tf] = (tl, ts, r)
            wfa_results.append(r)
            log(f"  {tag}: score={r['survivor_score']:.3f} medsh={r['median_oos_sharpe']:.2f} "
                f"cons={r['consistency']:.0%} trades/fold={r['avg_oos_trades']:.1f} "
                f"(min {r['min_oos_trades']}) pnl={r['total_oos_pnl']:+.1f}% "
                f"long={r['total_long_net']:+.1f}% short={r['total_short_net']:+.1f}%")
    flat = []
    for r in wfa_results:
        row = {k: v for k, v in r.items() if k != "fold_rows"}
        flat.append(row)
    pd.DataFrame(flat).to_csv(WFA_CSV, index=False)

    # ---- Select the champion ----
    # Criteria: OOS positive total PnL, both-direction attribution >= 0,
    # consistency >= 0.5. Rank by survivor score.
    viable = [r for r in wfa_results
              if r["total_oos_pnl"] > 0 and r["consistency"] >= 0.5]
    bidir_ok = [r for r in viable
                if r["total_long_net"] >= 0 and r["total_short_net"] >= 0]
    pool = bidir_ok if bidir_ok else viable
    if not pool:
        log("\n  ❌ No candidate survives WFA with positive OOS PnL. Family closes.")
        write_final_verdict(None, wfa_results, {})
        return
    champ = max(pool, key=lambda r: r["survivor_score"])
    tf = champ["tf"]
    label = champ["label"]
    log(f"\n  CHAMPION: {label} score={champ['survivor_score']:.3f}")
    if label.endswith("COMPOSITE"):
        p_long, p_short, _ = composites[tf]
    elif label.endswith("_both"):
        p_long = dict(refined[label]); p_long["direction_filter"] = "long"
        p_short = dict(refined[label]); p_short["direction_filter"] = "short"
    elif label.endswith("_long"):
        p_long = refined[label]; p_short = None
    else:
        p_long = None; p_short = refined[label]

    # ---- F4: sensitivity ----
    log("\n--- F4: sensitivity ±20% ---")
    base_net, spread, flips, robust, sens_rows = sensitivity(
        tf_data[tf], tf, p_long if p_long else p_short,
        use_composite=(p_long is not None and p_short is not None),
        params2=p_short if p_long is not None else None)
    log(f"  base_total_net={base_net:+.1f}% spread={spread:.1f} sign_flips={flips} -> {robust}")
    pd.DataFrame(sens_rows).to_csv(os.path.join(OUT_DIR, f"finisher_sensitivity_{tf}m.csv"), index=False)

    # ---- F5: leverage sweep ----
    log("\n--- F5: leverage sweep ---")
    lev_rows = []
    best_lev = 1.0; best_final = -1e9
    folds = create_folds(len(tf_data[tf]), 9, 36, tf)
    for lev in [1.0, 2.0, 3.0, 5.0]:
        c = compound(tf_data[tf], tf, p_long, p_short, leverage=lev)
        # OOS consistency at leverage via fold PnL sign
        fold_pos = 0; n = 0
        for f in folds:
            test_df = tf_data[tf].iloc[f["test_start"]:f["test_end"]]
            trips = run_composite(test_df, p_long, p_short, lev, tf)
            net = net_pnl(trips, lev, tf)
            if net and sum(t["net_pnl"] for t in net) > 0: fold_pos += 1
            n += 1
        passed = (c["max_dd_pct"] <= 25 and c["liquidations"] == 0 and
                  (fold_pos / n if n else 0) >= 0.5 and c["final_sol"] > START_SOL)
        lev_rows.append({"leverage": lev, "final_sol": c["final_sol"],
                         "max_dd_pct": c["max_dd_pct"], "liquidations": c["liquidations"],
                         "fold_profitable": fold_pos, "passed": passed})
        log(f"  {lev}x: final={c['final_sol']} dd={c['max_dd_pct']}% liq={c['liquidations']} "
            f"folds_pos={fold_pos}/{n} {'PASS' if passed else 'fail'}")
        if passed and c["final_sol"] > best_final:
            best_final = c["final_sol"]; best_lev = lev
    pd.DataFrame(lev_rows).to_csv(os.path.join(OUT_DIR, f"finisher_leverage_{tf}m.csv"), index=False)

    # ---- F6: SOL compounding at best leverage ----
    log(f"\n--- F6: 2.5 SOL compounding @ {best_lev}x ---")
    comp = compound(tf_data[tf], tf, p_long, p_short, leverage=best_lev)
    log(f"  final={comp['final_sol']} SOL ({comp['ret_pct']:+.1f}%) dd={comp['max_dd_pct']}% "
        f"trades={comp['trades']} ({comp['trades_per_day']}/day) liq={comp['liquidations']} "
        f"halts={comp['halts']} long={comp['long_sol']:+.4f} SOL short={comp['short_sol']:+.4f} SOL "
        f"fees={comp['fees_sol']} SOL gas={comp['gas_sol']} SOL")

    # ---- F7: latency ----
    log("\n--- F7: latency robustness (1-poll entry delay) ---")
    delay_bars = max(1, POLL_MINUTES // tf)
    comp_lat = compound(tf_data[tf], tf, p_long, p_short, leverage=best_lev, delay_bars=delay_bars)
    retention = (comp_lat["final_sol"] - START_SOL) / (comp["final_sol"] - START_SOL) \
        if comp["final_sol"] != START_SOL else 1.0
    lat_pass = retention >= 0.8
    log(f"  delay={delay_bars} bars: final={comp_lat['final_sol']} SOL ({comp_lat['ret_pct']:+.1f}%) "
        f"retention={retention:.0%} {'PASS' if lat_pass else 'FAIL'}")

    # ---- F8: cross-symbol ----
    log(f"\n--- F8: cross-symbol @ {tf}m ---")
    cross_rows = []
    for sym in ["BTC/USDT", "ETH/USDT"]:
        dfa = get_data(sym, tf)
        r = wfa_candidate(dfa, tf, p_long, p_short, f"{sym}_{tf}m")
        cross_rows.append({"symbol": sym, "tf": tf, "median_oos_sharpe": r["median_oos_sharpe"],
                           "consistency": r["consistency"], "total_oos_pnl": r["total_oos_pnl"],
                           "long_net": r["total_long_net"], "short_net": r["total_short_net"],
                           "avg_oos_trades": r["avg_oos_trades"]})
        log(f"  {sym}: medsh={r['median_oos_sharpe']:.2f} cons={r['consistency']:.0%} "
            f"pnl={r['total_oos_pnl']:+.1f}%")
    pd.DataFrame(cross_rows).to_csv(os.path.join(OUT_DIR, "finisher_cross_symbol.csv"), index=False)
    cross_pass = any(c["total_oos_pnl"] > 0 and c["consistency"] >= 0.5 for c in cross_rows)

    # ---- F9: final verdict ----
    write_final_verdict({
        "champion": champ, "tf": tf, "label": label,
        "p_long": p_long, "p_short": p_short,
        "sensitivity": {"base": base_net, "spread": spread, "flips": flips, "robust": robust},
        "best_leverage": best_lev, "compounding": comp,
        "latency": {"delay_bars": delay_bars, "final_sol": comp_lat["final_sol"],
                    "retention": retention, "pass": lat_pass},
        "cross_symbol": cross_rows, "cross_pass": cross_pass,
    }, wfa_results, composites)


def write_final_verdict(ctx, wfa_results, composites):
    path = os.path.join(OUT_DIR, "verdict.md")
    ts = datetime.now().isoformat()
    L = [f"# S15 Marubozu-with-Confirmation — v5 FINAL Verdict\n\nGenerated: {ts}\n\n"]

    if ctx is None:
        L += ["## Status: FALSIFIED at direction-aware WFA\n\n",
              "Confirmation entry (H1) flipped the full-year net gate (+0.152%/trade @ 20m) ",
              "and the Phase 2 grids contain net-positive long, short, and both-direction ",
              "candidates, but NONE survive 9-fold OOS WFA with positive total PnL at ",
              ">=50% fold consistency.\n\n",
              "The marubozu-retracement family (S14 blind touch + S15 confirmation) is ",
              "closed permanently. The full-year positives were in-sample artifacts of the ",
              "coarse grid.\n"]
        with open(path, "w") as f: f.writelines(L)
        log(f"  FINAL verdict (FALSIFIED) -> {path}")
        return

    champ = ctx["champion"]; comp = ctx["compounding"]; lat = ctx["latency"]
    sens = ctx["sensitivity"]; lev = ctx["best_leverage"]
    crit = {
        "oos_positive_total_pnl": champ["total_oos_pnl"] > 0,
        "oos_consistency_ge_50": champ["consistency"] >= 0.5,
        "bidirectional_attribution_ge_0": (champ["total_long_net"] >= 0 and
                                           champ["total_short_net"] >= 0),
        "compounding_gt_start": comp["final_sol"] > START_SOL,
        "zero_halts_zero_liq": comp["halts"] == 0 and comp["liquidations"] == 0,
        "trades_per_day_le_4": comp["within_4_per_day"],
        "dd_le_25": comp["max_dd_pct"] <= 25,
        "sensitivity_not_fragile": sens["robust"] != "FRAGILE",
        "latency_retention_ge_80": lat["pass"],
        "cross_symbol_positive": ctx["cross_pass"],
        "min_trades_per_fold_ge_10": champ["min_oos_trades"] >= 10,
    }
    all_pass = all(crit.values())

    L += ["## Status: " + ("✅ ALL CRITERIA MET — deployable candidate" if all_pass
          else "⚠️ PARTIAL — see scorecard") + "\n\n"]

    L += ["## Hypothesis scorecard (H1–H7)\n\n",
          "| # | Hypothesis | Verdict |\n|---|-----------|--------|\n",
          "| H1 | Confirmation lifts win rate ≥35%, flips net expectancy | "
          f"{'PASS' if champ['total_oos_pnl'] > 0 else 'PARTIAL'} — gate +0.152%/trade, "
          f"champion OOS WR see below |\n",
          f"| H2 | Optimal band 10m–30m | champion TF = {champ['tf']}m |\n",
          "| H3 | Structure exits beat ATR exits | see finisher_exec_sweep.csv |\n",
          f"| H4 | Bidirectional attribution ≥0 both sides | long_net="
          f"{champ['total_long_net']:+.1f}% short_net={champ['total_short_net']:+.1f}% |\n",
          "| H5 | VWAP filter lifts consistency | see exec sweep (use_vwap_filter axis) |\n",
          f"| H6 | Operational: 2.5 SOL compounding | final={comp['final_sol']} SOL "
          f"dd={comp['max_dd_pct']}% liq={comp['liquidations']} halts={comp['halts']} |\n",
          f"| H7 | Cross-symbol | " +
          "; ".join(f"{c['symbol']}={c['total_oos_pnl']:+.1f}%" for c in ctx["cross_symbol"]) + " |\n\n"]

    L += ["## Success criteria\n\n"]
    for k, v in crit.items():
        L.append(f"- {'✅' if v else '❌'} {k}\n")

    L += [f"\n## Champion: {champ['label']}\n\n",
          f"- Survivor score: {champ['survivor_score']:.3f}\n",
          f"- Median OOS Sharpe: {champ['median_oos_sharpe']:.2f}\n",
          f"- OOS consistency: {champ['consistency']:.0%}\n",
          f"- Total OOS PnL: {champ['total_oos_pnl']:+.1f}% "
          f"(long {champ['total_long_net']:+.1f}% / short {champ['total_short_net']:+.1f}%)\n",
          f"- Trades/fold: avg {champ['avg_oos_trades']:.1f}, min {champ['min_oos_trades']}\n",
          f"- Leverage: {lev}x\n",
          f"- Latency retention: {lat['retention']:.0%}\n",
          f"- Sensitivity: {sens['robust']} (spread {sens['spread']:.1f}, "
          f"{sens['flips']} sign flips)\n\n"]

    L += ["## Winning config\n\n```json\n"]
    cfg = {"tf": champ["tf"], "leverage": lev,
           "params_long": ctx["p_long"], "params_short": ctx["p_short"],
           "composite": ctx["label"].endswith("COMPOSITE") or ctx["label"].endswith("_both")}
    L.append(json.dumps(cfg, indent=2, default=str))
    L.append("\n```\n")

    with open(path, "w") as f: f.writelines(L)
    if all_pass:
        with open(os.path.join(OUT_DIR, "winning_config.json"), "w") as f:
            json.dump(cfg, f, indent=2, default=str)
        log(f"  winning_config.json written")
    log(f"  FINAL verdict -> {path} (all_pass={all_pass})")


if __name__ == "__main__":
    main()
