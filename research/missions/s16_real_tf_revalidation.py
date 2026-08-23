"""
S16 — Real multi-TF re-validation of Survivor 2.69 (P3-4/P3-6).

WHY: every prior validation artifact (Calmar 44.89, OOS Sharpe 3.96, the
sensitivity CSV, night-shift candidates) computed "multi-TF" as lookback
20/80/200 on a SINGLE 1h close series ("fake" multi-TF — see
per_symbol_optimizer.py:37). The live trader (rtp/swarm) runs REAL
multi-TF: independent 1h/4h/1d Binance series, each with its own 20-period
SMA/trend, momentum off the true 4h series. The live model was therefore
NEVER validated; this script closes that gap and ranks candidate configs on
the model the trader actually runs.

Model parity with rtp/swarm/src/trader/strategy.rs + indicators.rs:
- tf trend: price vs SMA(20) on each of the 1h / 4h / 1d series (4h/1d
  bars forward-filled onto the hourly decision grid; a decision at hour H
  sees the 4h/1d closes finalised strictly before H — no look-ahead)
- score = 0.4*bull/3 (or -0.4*bear/3) gated on min_alignment, +0.1 vol
  confirm, MR 0.3*rsi term, 0.15 momentum (true 4h), 0.15 BB
- ATR = std(returns, 20) * price on the 1h series (NOT True Range)
- entry: score > thr long / score < -thr short; exit priority:
  trailing stop, hard SL, TP, max hold, time decay, score flip (with
  delay timer), MR target — same order as check_exit()
- exits evaluated on hourly closes, same as the hourly backtest reference

Fees: GMTrade measured basis (probe 2026-08-08): 0.022%/trip of notional
+ long borrow 0.0036%/hr of notional (shorts pay 0 — skip-smaller-side).
Usage: python research/missions/s16_real_tf_revalidation.py
"""

import json
import os
import sys
from datetime import datetime, timezone

import numpy as np
import pandas as pd

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DATA = os.path.join(ROOT, "data", "ohlcv")
OUT_DIR = os.path.join(ROOT, "data", "results", "s16_real_tf")
SYMBOL = "SOL/USDT"
LEVERAGE = 9.0
TRIP_FEE_PCT = 0.022          # % of notional per round trip (measured)
LONG_BORROW_PCT_HR = 0.0036   # % of notional per hour (measured usage 0.706)
SHORT_BORROW_PCT_HR = 0.0     # skip-smaller-side: shorts pay zero
WARMUP_HOURS = 300            # matches the live buffer size
FOLD_DAYS = 36                # anchored equal windows (v7e-corrected scheme)

BASELINE = dict(signal_threshold=0.30, min_alignment=2, tp_atr=6.0,
                sl_atr=2.5, max_hold_hours=96.0, trailing_stop_atr=1.0,
                time_decay_hours=48.0, score_flip_delay_hrs=2.0)


def log(msg=""):
    print(msg, flush=True)


def fetch_fresh(suffix_tf: str, interval: str, days: int = 380):
    """Fetch fresh Binance klines into OUT_DIR (never touches data/ohlcv).

    Returns a DataFrame indexed at bar-OPEN UTC-naive (same convention as
    the existing data/ohlcv parquets). Drops the in-progress candle.
    Falls back to the stored parquet on any fetch failure.
    """
    out_path = os.path.join(OUT_DIR, f"SOL_USDT_{suffix_tf}.parquet")
    try:
        import urllib.request

        rows = []
        end_ms = int(datetime.now(timezone.utc).timestamp() * 1000)
        since_ms = end_ms - days * 86400_000
        while True:
            url = (f"https://api.binance.com/api/v3/klines?symbol=SOLUSDT"
                   f"&interval={interval}&startTime={since_ms}&limit=1000")
            req = urllib.request.Request(url, headers={"User-Agent": "rtp-s16-audit"})
            batch = json.load(urllib.request.urlopen(req, timeout=30))
            if not batch:
                break
            rows.extend(batch)
            if len(batch) < 1000:
                break
            since_ms = batch[-1][0] + 1
        if not rows:
            raise RuntimeError("empty kline response")
        df = pd.DataFrame(rows, columns=["ts", "open", "high", "low", "close", "volume"] + ["_"] * (len(rows[0]) - 6))
        df = df[["ts", "open", "high", "low", "close", "volume"]].astype(
            {"open": float, "high": float, "low": float, "close": float, "volume": float})
        df["timestamp"] = pd.to_datetime(df["ts"], unit="ms", utc=True).dt.tz_localize(None)
        df = df.drop_duplicates(subset="timestamp").set_index("timestamp").sort_index()
        df = df.drop(columns=["ts"])
        # drop the in-progress candle (its open time == current period start)
        period_h = {"1h": 1, "4h": 4, "1d": 24}[interval]
        cur_start = (int(datetime.now(timezone.utc).timestamp()) // (period_h * 3600)) * period_h * 3600
        if len(df) and df.index[-1] == pd.Timestamp.utcfromtimestamp(cur_start).tz_localize(None):
            df = df.iloc[:-1]
        df.to_parquet(out_path)
        log(f"  fetched {interval}: {len(df)} bars {df.index[0]} -> {df.index[-1]}")
        return df
    except Exception as e:  # noqa: BLE001 — network fallback path
        log(f"  fetch {interval} failed ({e}) — using stored parquet")
        return pd.read_parquet(os.path.join(DATA, f"SOL_USDT_{suffix_tf}.parquet"))


def timeframe_arrays(close: pd.Series, lookback: int = 20):
    """trend (+1 bullish / -1 bearish / 0 neutral), momentum, rsi, vol."""
    sma = close.rolling(lookback).mean()
    trend = np.where(close > sma, 1, np.where(close < sma, -1, 0))
    returns = close.pct_change()
    momentum = returns.rolling(lookback).mean()
    delta = close.diff()
    gain = delta.where(delta > 0, 0.0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0.0)).rolling(14).mean()
    rs = gain / loss.replace(0, np.nan)
    rsi = 100 - 100 / (1 + rs)
    rsi = rsi.fillna(50.0)
    vol = returns.rolling(lookback).std()
    return trend, momentum, rsi, vol


def hourly_slow_tf(slow_close: pd.Series, hourly_index: pd.DatetimeIndex) -> pd.Series:
    """Map slow-TF closes onto hourly decision timestamps with NO look-ahead.

    A slow bar covering [T, T+period) finalises at its close time; a
    decision at hour H must see the last bar that FINALISED before H.
    slow_close is indexed at bar-OPEN (Binance convention), so shift the
    index to bar-CLOSE (open + period), then ffill onto H and require the
    bar closed <= H: reindex ffill from the close-stamped series does
    exactly that.
    """
    period = slow_close.index.to_series().diff().median()
    close_stamped = slow_close.copy()
    close_stamped.index = close_stamped.index + period
    return close_stamped.reindex(hourly_index.union(close_stamped.index)).ffill().reindex(hourly_index)


def build_features():
    # Stored parquets lag (1h to 2026-08-05, 4h/1d to 2026-04-08); fetch
    # fresh so validation covers the window the live trader actually traded.
    # fetch_fresh falls back to the stored file on any failure.
    df1 = fetch_fresh("1h", "1h", days=380)
    df4 = fetch_fresh("4h", "4h", days=380)
    dfd = fetch_fresh("1d", "1d", days=400)
    idx = df1.index

    close = df1["close"]
    volume = df1["volume"]

    t1, m1, rsi1, vol1 = timeframe_arrays(close, 20)
    c4 = hourly_slow_tf(df4["close"], idx)
    cd = hourly_slow_tf(dfd["close"], idx)
    t4, m4, _, _ = timeframe_arrays(c4, 20)
    td, _, _, _ = timeframe_arrays(cd, 20)

    bull = (t1 == 1).astype(int) + (t4 == 1).astype(int) + (td == 1).astype(int)
    bear = (t1 == -1).astype(int) + (t4 == -1).astype(int) + (td == -1).astype(int)

    vol_ratio = volume / volume.rolling(20).mean().replace(0, np.nan)
    vol_ratio = vol_ratio.fillna(1.0)

    sma20 = close.rolling(20).mean()
    std20 = close.rolling(20).std()
    bb = (close - (sma20 - 2 * std20)) / (4 * std20)
    bb = bb.fillna(0.5)

    returns = close.pct_change()
    atr = returns.rolling(20).std() * close  # validated formula, NOT TR

    bull = pd.Series(bull, index=idx)
    bear = pd.Series(bear, index=idx)
    mom = pd.Series(m4, index=idx)
    rsi = pd.Series(rsi1, index=idx)

    # score exactly as compute_signal() does
    for ma in (2, 3):
        s = pd.Series(0.0, index=idx)
        bl = bull >= ma
        be = bear >= ma
        s = s.where(~bl, s + (bull / 3.0) * 0.4)
        s = s.where(~(bl & (vol_ratio > 1.3)), s + 0.1)
        s = s.where(~be, s - (bear / 3.0) * 0.4)
        s = s.where(~(be & (vol_ratio > 1.3)), s - 0.1)
        mr = pd.Series(0.0, index=idx)
        daily_bull = td == 1
        daily_bear = td == -1
        mr = mr.where(~(rsi < 30), 0.3)
        mr = mr.where(~((rsi >= 30) & (rsi < 35) & daily_bull), 0.2)
        mr = mr.where(~(rsi > 70), -0.3)
        mr = mr.where(~((rsi >= 65) & (rsi <= 70) & daily_bear), -0.2)
        s = s + mr * 0.3
        s = s + np.where(mom > 0.003, 0.15, np.where(mom < -0.003, -0.15, 0.0))
        s = s + np.where(bb < 0.15, 0.15, np.where(bb > 0.85, -0.15, 0.0))
        if ma == 2:
            score2 = s
        else:
            score3 = s

    return dict(close=close, atr=atr, rsi=rsi, score2=score2, score3=score3,
                bull=bull, bear=bear)


def simulate(f: dict, p: dict, start_i: int, end_i: int):
    """Sequential hour-bar simulation mirroring check_exit() priorities."""
    close = f["close"].values
    atr = f["atr"].values
    rsi = f["rsi"].values
    score = (f["score2"] if p["min_alignment"] == 2 else f["score3"]).values

    thr = p["signal_threshold"]
    trips = []
    pos = None  # dict with side/entry fields

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

            # 1. trailing stop (peak/trough must be favourable first)
            if p["trailing_stop_atr"] > 0 and a > 0 and entry > 0:
                trigger = p["trailing_stop_atr"] * a / entry * 100
                if side == 1:
                    if peak > entry and (peak - c) / entry * 100 >= trigger:
                        exit_reason = "trailing_stop"
                else:
                    if peak < entry and (c - peak) / entry * 100 >= trigger:
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
                fee = TRIP_FEE_PCT + borrow  # % of notional
                net_pct = pnl_pct * LEVERAGE - fee * LEVERAGE
                trips.append(dict(pnl_pct=pnl_pct, net_pct=net_pct, hold_h=held_h,
                                  reason=exit_reason, side="long" if side == 1 else "short"))
                pos = None
            else:
                pos["peak"] = min(peak, c) if side == -1 else max(peak, c)
            continue

        # flat → entry check
        if sc > thr:
            pos = dict(side=1, entry_price=c, entry_i=i, peak=c, entry_rsi=r, flip_t=None)
        elif sc < -thr:
            pos = dict(side=-1, entry_price=c, entry_i=i, peak=c, entry_rsi=r, flip_t=None)

    # open position at fold end: mark-to-market close
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


def folds(n: int, warmup: int, fold_bars: int):
    """Anchored equal test windows after warmup (v7e scheme)."""
    out = []
    start = warmup
    while start + fold_bars <= n:
        out.append((start, start + fold_bars))
        start += fold_bars
    return out


def fold_metrics(trips):
    if not trips:
        return dict(n=0, sharpe=0.0, pnl=0.0, wr=0.0, dd=0.0)
    nets = np.array([t["net_pct"] for t in trips])
    cum = np.cumsum(nets)
    dd = float((np.maximum.accumulate(cum) - cum).max()) if len(cum) else 0.0
    sharpe = float(nets.mean() / nets.std()) if nets.std() > 0 else 0.0
    return dict(n=len(trips), sharpe=sharpe, pnl=float(nets.sum()),
                wr=float((nets > 0).mean()), dd=dd)


def evaluate(f, p):
    n = len(f["close"])
    fl = folds(n, WARMUP_HOURS, FOLD_DAYS * 24)
    per_fold = []
    reasons = {}
    for (a, b) in fl:
        trips = simulate(f, p, a, b)
        per_fold.append(fold_metrics(trips))
        for t in trips:
            reasons[t["reason"]] = reasons.get(t["reason"], 0) + 1
    sharpes = [m["sharpe"] for m in per_fold if m["n"] >= 5]
    pnls = [m["pnl"] for m in per_fold]
    return dict(
        folds=len(fl),
        median_sharpe=float(np.median(sharpes)) if sharpes else 0.0,
        total_net_pct=float(sum(pnls)),
        consistency=float(np.mean([1 if m["pnl"] > 0 else 0 for m in per_fold])) if per_fold else 0.0,
        max_dd=float(max(m["dd"] for m in per_fold)) if per_fold else 0.0,
        trades=sum(m["n"] for m in per_fold),
        avg_trades_per_fold=float(np.mean([m["n"] for m in per_fold])) if per_fold else 0.0,
        exits=reasons,
    )


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    log("S16 real multi-TF re-validation — building features (this mirrors the live trader)...")
    f = build_features()
    n = len(f["close"])
    log(f"data: {f['close'].index[0]} -> {f['close'].index[-1]} ({n} hourly bars)")

    grid = []
    # 1. baseline as deployed
    grid.append(("baseline_live", dict(BASELINE)))
    # 2. live-with-override threshold
    grid.append(("live_override_0.24", dict(BASELINE, signal_threshold=0.24)))
    # 3. night-shift candidate (align 3)
    grid.append(("night_top_align3", dict(BASELINE, min_alignment=3)))
    # 4. trailing-stop sweep (P3-6) at align 2
    for tr in (0.4, 0.5, 0.6, 0.75, 1.5):
        grid.append((f"trail_{tr}", dict(BASELINE, trailing_stop_atr=tr)))
    # 5. threshold sweep at align 2
    for th in (0.26, 0.28, 0.35):
        grid.append((f"thr_{th}", dict(BASELINE, signal_threshold=th)))
    # 6. tp/sl focused variants
    grid.append(("tp_4.5", dict(BASELINE, tp_atr=4.5)))
    grid.append(("tp_7.5", dict(BASELINE, tp_atr=7.5)))
    grid.append(("sl_2.0", dict(BASELINE, sl_atr=2.0)))
    grid.append(("hold_72_decay_36", dict(BASELINE, max_hold_hours=72.0, time_decay_hours=36.0)))
    grid.append(("flip_1h", dict(BASELINE, score_flip_delay_hrs=1.0)))
    grid.append(("flip_4h", dict(BASELINE, score_flip_delay_hrs=4.0)))
    # combined candidates (trail + thr)
    grid.append(("thr0.24_trail0.5", dict(BASELINE, signal_threshold=0.24, trailing_stop_atr=0.5)))
    grid.append(("thr0.24_trail0.75", dict(BASELINE, signal_threshold=0.24, trailing_stop_atr=0.75)))
    grid.append(("thr0.30_trail0.5", dict(BASELINE, trailing_stop_atr=0.5)))
    grid.append(("align3_thr0.30_trail0.5", dict(BASELINE, min_alignment=3, trailing_stop_atr=0.5)))

    results = []
    log(f"\n{'label':28s} {'medSharpe':>9s} {'netPnL%':>8s} {'cons':>5s} {'trades':>6s} {'maxDD':>6s}  exits")
    log("-" * 110)
    for label, p in grid:
        r = evaluate(f, p)
        r["label"], r["params"] = label, p
        results.append(r)
        ex = ", ".join(f"{k}:{v}" for k, v in sorted(r["exits"].items(), key=lambda kv: -kv[1])[:4])
        log(f"{label:28s} {r['median_sharpe']:9.2f} {r['total_net_pct']:8.1f} {r['consistency']*100:4.0f}% "
            f"{r['trades']:6d} {r['max_dd']:6.1f}  {ex}")

    results.sort(key=lambda r: r["median_sharpe"], reverse=True)
    out = dict(run_at=datetime.now(timezone.utc).isoformat(),
               symbol=SYMBOL, leverage=LEVERAGE, fee_model="GMTrade measured 0.022%/trip + long borrow 0.0036%/hr",
               data_window=f"{f['close'].index[0]} -> {f['close'].index[-1]}",
               fold_scheme=f"anchored equal {FOLD_DAYS}d windows after {WARMUP_HOURS}h warmup",
               results=results)
    path = os.path.join(OUT_DIR, "revalidation.json")
    with open(path, "w") as fh:
        json.dump(out, fh, indent=2)
    log(f"\nTOP 5 by median OOS Sharpe:")
    for r in results[:5]:
        log(f"  {r['label']:28s} sharpe={r['median_sharpe']:.2f} net={r['total_net_pct']:+.1f}% "
            f"cons={r['consistency']*100:.0f}% dd={r['max_dd']:.1f}% params={json.dumps(r['params'])}")
    log(f"\nsaved -> {path}")


if __name__ == "__main__":
    sys.exit(main())
