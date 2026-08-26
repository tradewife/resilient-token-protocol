"""
S17 — Trailing-stop arm variants: do delayed/armed trails beat the live 1.0xATR trail?

WHY: live-history forensics (2026-08-26 audit, 161 trades) showed the
trailing-stop family is the single biggest drag (95 exits, -26.4 net %pts):
60/87 trail exits had led >=1 ATR then gave it back, and 37 saw a >2%
favourable run within 24h AFTER exit. The naive fix ("hold to TP instead")
was counterfactually replayed bar-by-bar and found ~flat (-0.19 pts) — the
chop touches SL and TP alike, so patience alone does not pay. S17 tests the
one mechanism-based alternative: don't arm the trailing stop until the trade
has proven itself (lead >= N ATR), optionally ratcheting to breakeven first.

Model: identical to S16 real multi-TF (imports the S16 harness — real
independent 1h/4h/1d Binance feeds, score as compute_signal(), ATR =
std(returns,20)xprice, exit priority as check_exit()). Only the trailing
arm logic varies. All variants judged against the same gates, same folds,
same GMTrade measured fee model (0.022%/trip + long borrow 0.0036%/hr),
so the ranking is apples-to-apples on the model the trader actually runs.

Trail variant semantics (per side, all else as S16/check_exit):
  - live        : arm as soon as favourable (current production rule)
  - off         : trailing disabled entirely (TP/SL/hold/decay/flip rule the exit)
  - arm_N       : trailing only engages after lead >= N*ATR
  - be_N        : after lead >= N*ATR the stop ratchets to
                  max(entry, peak - trail*ATR) — breakeven first, then trail
  - be_N_wM     : like be_N but with trail width M*ATR once armed

Gates judged (research/promotion_criteria.py PromotionGate):
  median OOS Sharpe >= 1.5, folds >= 3, win rate >= 45%,
  profit factor >= 1.3, max drawdown <= 20%.
(Regime-robustness, paper-trading and consensus gates are out of scope for
this mission — statistical subset only.)

Usage: python research/missions/s17_trail_arm_variants.py
"""

import json
import os
import sys
from datetime import datetime, timezone

import numpy as np

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, ROOT)

import research.missions.s16_real_tf_revalidation as s16  # noqa: E402

OUT_DIR = os.path.join(ROOT, "data", "results", "s17_trail_variants")
LEVERAGE = s16.LEVERAGE
TRIP_FEE_PCT = s16.TRIP_FEE_PCT
LONG_BORROW_PCT_HR = s16.LONG_BORROW_PCT_HR
SHORT_BORROW_PCT_HR = s16.SHORT_BORROW_PCT_HR
WARMUP_HOURS = s16.WARMUP_HOURS
FOLD_DAYS = s16.FOLD_DAYS

BASELINE = dict(s16.BASELINE)


def log(msg=""):
    print(msg, flush=True)


def simulate(f: dict, p: dict, start_i: int, end_i: int):
    """S16 exit priority with parameterised trailing-arm behaviour.

    Trail spec: dict(mode='live'|'off'|'arm'|'be', arm_atr=N, trail_atr=M, width_atr=W)
    """
    close = f["close"].values
    atr = f["atr"].values
    rsi = f["rsi"].values
    score = (f["score2"] if p["min_alignment"] == 2 else f["score3"]).values
    spec = p["trail_spec"]

    thr = p["signal_threshold"]
    trips = []
    pos = None

    for i in range(start_i, end_i):
        c = close[i]
        a = atr[i]
        sc = score[i]
        r = rsi[i]
        if np.isnan(a):
            continue

        if pos is not None:
            side = pos["side"]
            entry = pos["entry_price"]
            pnl_pct = (entry - c) / entry * 100 if side == -1 else (c - entry) / entry * 100
            hold = i - pos["entry_i"]
            peak = pos["peak"]
            exit_reason = None

            # 1. trailing stop — arm-logic variants
            if spec["mode"] != "off" and spec["trail_atr"] > 0 and a > 0 and entry > 0:
                lead_pct = ((peak - entry) if side == 1 else (entry - peak)) / entry * 100
                arm_thr = spec["arm_atr"] * a / entry * 100
                if spec["mode"] == "live":
                    armed = (peak > entry) if side == 1 else (peak < entry)
                else:  # 'arm' / 'be'
                    armed = lead_pct >= arm_thr
                if armed:
                    if spec["mode"] == "be":
                        # breakeven-first: stop never worse than entry once armed
                        if side == 1:
                            stop_px = max(entry, peak - spec["width_atr"] * a)
                            hit = c <= stop_px
                        else:
                            stop_px = min(entry, peak + spec["width_atr"] * a)
                            hit = c >= stop_px
                    else:  # 'live' / 'arm' — plain width trail from peak
                        trigger = spec["width_atr"] * a / entry * 100
                        drawdown = ((peak - c) / entry * 100) if side == 1 \
                            else ((c - peak) / entry * 100)
                        hit = drawdown >= trigger
                    if hit:
                        exit_reason = "trailing_stop"
            # 2. hard stop loss
            if exit_reason is None and a > 0:
                if pnl_pct <= -(p["sl_atr"] * a / entry * 100):
                    exit_reason = "stop_loss"
            # 3. take profit
            if exit_reason is None and a > 0:
                if pnl_pct >= (p["tp_atr"] * a / entry * 100):
                    exit_reason = "take_profit"
            # 4. max hold
            if exit_reason is None and hold >= p["max_hold_hours"]:
                exit_reason = "max_hold"
            # 5. time decay (losing)
            if exit_reason is None and pnl_pct < 0 and p["time_decay_hours"] > 0 \
                    and hold >= p["time_decay_hours"]:
                exit_reason = "time_decay"
            # 6. score flip with delay timer
            if exit_reason is None:
                flipped = sc > 0 if side == -1 else sc < 0
                if flipped:
                    if pos["flip_t"] is None:
                        pos["flip_t"] = i
                    if p["score_flip_delay_hrs"] <= 0 or (i - pos["flip_t"]) >= p["score_flip_delay_hrs"]:
                        exit_reason = "score_flip"
                else:
                    pos["flip_t"] = None
            # 7. MR target
            if exit_reason is None and r > 55 and pos["entry_rsi"] < 35:
                exit_reason = "mr_target"

            if exit_reason is not None:
                held_h = hold
                borrow = (LONG_BORROW_PCT_HR if side == 1 else SHORT_BORROW_PCT_HR) * held_h
                fee = TRIP_FEE_PCT + borrow
                net_pct = pnl_pct * LEVERAGE - fee * LEVERAGE
                trips.append(dict(pnl_pct=pnl_pct, net_pct=net_pct, hold_h=held_h,
                                  reason=exit_reason, side="long" if side == 1 else "short"))
                pos = None
            else:
                pos["peak"] = min(peak, c) if side == -1 else max(peak, c)
            continue

        if sc > thr:
            pos = dict(side=1, entry_price=c, entry_i=i, peak=c, entry_rsi=r, flip_t=None)
        elif sc < -thr:
            pos = dict(side=-1, entry_price=c, entry_i=i, peak=c, entry_rsi=r, flip_t=None)

    if pos is not None:
        c = close[end_i - 1]
        side = pos["side"]
        entry = pos["entry_price"]
        pnl_pct = (entry - c) / entry * 100 if side == -1 else (c - entry) / entry * 100
        held_h = (end_i - 1) - pos["entry_i"]
        borrow = (LONG_BORROW_PCT_HR if side == 1 else SHORT_BORROW_PCT_HR) * held_h
        fee = TRIP_FEE_PCT + borrow
        trips.append(dict(pnl_pct=pnl_pct, net_pct=(pnl_pct - fee) * LEVERAGE,
                          hold_h=held_h, reason="fold_end", side="long" if side == 1 else "short"))
    return trips


def gates_verdict(agg: dict) -> dict:
    g = dict(
        sharpe_ok=agg["median_sharpe"] >= 1.5,
        folds_ok=agg["folds"] >= 3,
        winrate_ok=agg["win_rate"] >= 0.45,
        pf_ok=agg["profit_factor"] >= 1.3,
        dd_ok=agg["max_dd"] <= 20.0,
    )
    g["pass"] = all(g.values())
    return g


def evaluate(f, p):
    n = len(f["close"])
    fl = s16.folds(n, WARMUP_HOURS, FOLD_DAYS * 24)
    per_fold = []
    all_trips = []
    for (a, b) in fl:
        trips = simulate(f, p, a, b)
        per_fold.append(s16.fold_metrics(trips))
        all_trips.extend(trips)

    sharpes = [m["sharpe"] for m in per_fold if m["n"] >= 5]
    nets = np.array([t["net_pct"] for t in all_trips])
    gross_w = nets[nets > 0].sum()
    gross_l = -nets[nets < 0].sum()
    reasons = {}
    for t in all_trips:
        reasons[t["reason"]] = reasons.get(t["reason"], 0) + 1

    agg = dict(
        folds=len(fl),
        median_sharpe=float(np.median(sharpes)) if sharpes else 0.0,
        total_net_pct=float(nets.sum()),
        consistency=float(np.mean([1 if m["pnl"] > 0 else 0 for m in per_fold])) if per_fold else 0.0,
        max_dd=float(max(m["dd"] for m in per_fold)) if per_fold else 0.0,
        trades=len(all_trips),
        win_rate=float((nets > 0).mean()) if len(nets) else 0.0,
        profit_factor=float(gross_w / gross_l) if gross_l > 0 else float("inf"),
        exits=reasons,
    )
    agg["gates"] = gates_verdict(agg)
    return agg


def build_grid():
    """Variants x both thresholds (production runs 0.24; validated file 0.30)."""

    def spec(mode, arm=0.0, trail=1.0, width=1.0):
        return dict(mode=mode, arm_atr=arm, trail_atr=trail, width_atr=width)

    variants = [
        ("live_trail1.0", spec("live", width=1.0)),
        ("trail_off", spec("off")),
        ("arm_1.0", spec("arm", arm=1.0, width=1.0)),
        ("arm_1.5", spec("arm", arm=1.5, width=1.0)),
        ("arm_2.0", spec("arm", arm=2.0, width=1.0)),
        ("be_1.0", spec("be", arm=1.0, width=1.0)),
        ("be_1.5", spec("be", arm=1.5, width=1.0)),
        ("be_2.0", spec("be", arm=2.0, width=1.0)),
        ("be_1.5_w2.0", spec("be", arm=1.5, width=2.0)),
        ("be_2.0_w2.0", spec("be", arm=2.0, width=2.0)),
        ("be_2.0_w1.5", spec("be", arm=2.0, width=1.5)),
    ]
    grid = []
    for thr_label, thr in (("thr0.24", 0.24), ("thr0.30", 0.30)):
        for label, sp in variants:
            p = dict(BASELINE, signal_threshold=thr, trail_spec=sp)
            grid.append((f"{thr_label}/{label}", p))
    return grid


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    # fetch fresh real-TF data into THIS mission's dir (S16 fetcher respects OUT_DIR)
    s16.OUT_DIR = OUT_DIR
    log("S17 trail-arm variants — building real multi-TF features via S16 harness...")
    f = s16.build_features()
    n = len(f["close"])
    log(f"data: {f['close'].index[0]} -> {f['close'].index[-1]} ({n} hourly bars)\n")

    grid = build_grid()
    results = []
    log(f"{'config':28s} {'medShrp':>7s} {'net%':>8s} {'cons':>5s} {'win%':>5s} {'PF':>6s} {'maxDD':>6s} {'tr':>5s}  gates  exits")
    log("-" * 120)
    for label, p in grid:
        r = evaluate(f, p)
        r["label"], r["params"] = label, {k: v for k, v in p.items() if k != "trail_spec"}
        r["trail_spec"] = p["trail_spec"]
        results.append(r)
        g = r["gates"]
        mark = "PASS" if g["pass"] else "fail"
        ex = ", ".join(f"{k}:{v}" for k, v in sorted(r["exits"].items(), key=lambda kv: -kv[1])[:4])
        log(f"{label:28s} {r['median_sharpe']:7.2f} {r['total_net_pct']:8.1f} {r['consistency']*100:4.0f}% "
            f"{r['win_rate']*100:4.0f}% {r['profit_factor']:6.2f} {r['max_dd']:6.1f} {r['trades']:5d}  {mark:4s}  {ex}")

    out = dict(run_at=datetime.now(timezone.utc).isoformat(),
               mission="S17 trailing-stop arm variants",
               model="S16 real multi-TF (imports s16 harness)",
               leverage=LEVERAGE,
               fee_model="GMTrade measured 0.022%/trip + long borrow 0.0036%/hr",
               data_window=f"{f['close'].index[0]} -> {f['close'].index[-1]}",
               fold_scheme=f"anchored equal {FOLD_DAYS}d windows after {WARMUP_HOURS}h warmup",
               gates="median Sharpe>=1.5, folds>=3, WR>=45%, PF>=1.3, DD<=20%",
               results=results)
    path = os.path.join(OUT_DIR, "variants.json")
    with open(path, "w") as fh:
        json.dump(out, fh, indent=2)
    log(f"\nsaved -> {path}")

    # verdict summary
    best = max(results, key=lambda r: r["median_sharpe"])
    log(f"\nVERDICT: best by median Sharpe = {best['label']} "
        f"(sharpe {best['median_sharpe']:.2f}, net {best['total_net_pct']:+.1f}%, "
        f"gates {'PASS' if best['gates']['pass'] else 'FAIL'})")
    passing = [r["label"] for r in results if r["gates"]["pass"]]
    log(f"configs clearing all statistical gates: {passing if passing else 'NONE'}")


if __name__ == "__main__":
    sys.exit(main())
