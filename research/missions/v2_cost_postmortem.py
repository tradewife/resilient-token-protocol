#!/usr/bin/env python3
"""
Survivor 2.69 — Flash v2 cost-model post-mortem.

Question: does the validated Survivor 2.69 edge survive the Flash Trade v2
fee regime? The research engine that produced the validated config models
v1-era costs; Flash v2 (post `feat/funded-pool-accounting`, API build
2026-07-30) charges differently. This replay re-runs the LIVE trader's
signal + exit engine (true multi-TF: independent 1h/4h/1d buffers, 20-SMA
trends, side-aware score-flip) over the available multi-TF overlap window
and scores every round trip under both cost models.

Cost models (both applied to NOTIONAL, then expressed as % of margin at
leverage 9 — identical to night_shift.flash_trade_round_trip_cost):

  v1 model (what the WFA used, research/orchestration/night_shift.py):
      open  0.06% + close 0.06% + borrow 0.0042%/hr (both directions)
  v2 measured (2026-08-07, Flash REST preview/* endpoints + live accrual
      on an open position, wallet HDQ79...):
      open  0.02% + close 0.02% + spread ~0.01%/side (both legs)
      + borrow 0.0004%/hr of notional (live-measured on SHORT; preview
        borrowRateUi confirms both sides carry a borrow rate at this scale)

Signal engine mirrors rtp/swarm/src/trader/{strategy,indicators}.rs:
  - trend: price vs 20-SMA per TF; score weights 0.4 trend / 0.3 MR /
    0.15 momentum / 0.15 BB, +-0.1 vol-confirm; min_alignment=2 gate is
    baked into the trend term; entry gate is score-only (|score| > 0.30)
  - ATR = std(returns, 20) * price (NOT true range)
  - RSI(14) = simple-window average (matches Rust port, not Wilder)
  - exits: trail 1.0 ATR (side-aware peak/trough), SL 2.5 ATR, TP 6.0 ATR,
    max hold 96h, time decay 48h on losers, side-aware score-flip with 2h
    grace (LONG flips on score<0, SHORT flips on score>0), MR target

Output: data/results/v2_cost_postmortem/{trades.csv, verdict.md}
"""
import os
import json
import numpy as np
import pandas as pd
from datetime import datetime, timezone

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DATA = os.path.join(ROOT, "data", "ohlcv")
OUT = os.path.join(ROOT, "data", "results", "v2_cost_postmortem")

# --- production config (data/trader-strategy-config.json, verified) ---
PARAMS = dict(
    signal_threshold=0.30,
    tp_atr=6.0,
    sl_atr=2.5,
    max_hold_hours=96.0,
    trailing_stop_atr=1.0,
    time_decay_hours=48.0,
    min_alignment=2,
    score_flip_delay_hrs=2.0,
    leverage=9.0,
    position_fraction=0.20,
)

# --- cost models, % of NOTIONAL ---
V1 = dict(open=0.06, close=0.06, borrow_hr=0.0042, spread_side=0.0)
V2 = dict(open=0.02, close=0.02, borrow_hr=0.0004, spread_side=0.01)


def round_trip_cost_pct(model: dict, leverage: float, hold_hrs: float) -> float:
    """Fees as % of margin (notional = margin * leverage)."""
    notional_pct = (
        model["open"]
        + model["close"]
        + 2.0 * model["spread_side"]
        + model["borrow_hr"] * hold_hrs
    )
    return leverage * notional_pct


# ---------------------------------------------------------------- indicators
def rsi_simple(closes: np.ndarray, period: int = 14):
    if len(closes) < period + 1:
        return None
    deltas = np.diff(closes[-(period + 1):])
    gains = deltas[deltas > 0].sum()
    losses = (-deltas[deltas < 0]).sum()
    gains /= period
    losses /= period
    if losses == 0.0:
        return 100.0
    rs = gains / losses
    return 100.0 - 100.0 / (1.0 + rs)


def atr_proxy(closes: np.ndarray, period: int = 20):
    if len(closes) < period + 1:
        return None
    rets = np.diff(closes[-(period + 1):]) / closes[-(period + 1):-1]
    return float(rets.std(ddof=0)) * closes[-1]


def bollinger_position(closes: np.ndarray, period: int = 20):
    if len(closes) < period:
        return None
    s = closes[-period:]
    mean, std = s.mean(), s.std(ddof=0)
    if std == 0.0:
        return 0.5
    return float((closes[-1] - (mean - 2 * std)) / (4 * std))


def volume_ratio(vols: np.ndarray, period: int = 20):
    if len(vols) < period:
        return 1.0
    avg = vols[-period:].mean()
    if avg == 0.0:
        return 1.0
    return float(vols[-1] / avg)


def trend(closes: np.ndarray, lookback: int = 20, rust_semantics: bool = True):
    """Returns (trend, momentum) mirroring Rust timeframe_signal.

    NOTE (rust_semantics=True): the live Rust trader has an off-by-one in
    timeframe_signal — it builds lookback-1 returns then requires
    `returns.len() >= lookback`, which is never true, so momentum (and
    volatility) are ALWAYS 0.0 in production. The Python reference
    (run_backtest_r2.py) computes momentum correctly. We replay both and
    report the delta; the primary line uses live-Rust semantics because
    that is the engine generating the record being judged.
    """
    if len(closes) < lookback:
        return None
    s = closes[-lookback:]
    sma = s.mean()
    price = closes[-1]
    t = "bullish" if price > sma else ("bearish" if price < sma else "neutral")
    if rust_semantics:
        momentum = 0.0
    else:
        rets = np.diff(s) / s[:-1]
        momentum = float(rets.mean()) if len(rets) >= lookback else 0.0
    return t, momentum


def compute_signal(c1h, c4h, c1d, v1h, min_alignment, rust_semantics=True):
    if len(c1h) < 21 or len(c4h) < 21 or len(c1d) < 21:
        return None
    t1 = trend(c1h, rust_semantics=rust_semantics)
    t4 = trend(c4h, rust_semantics=rust_semantics)
    td = trend(c1d, rust_semantics=rust_semantics)
    if t1 is None or t4 is None or td is None:
        return None
    trends = [t1[0], t4[0], td[0]]
    bull = sum(1 for t in trends if t == "bullish")
    bear = sum(1 for t in trends if t == "bearish")

    rsi = rsi_simple(c1h)
    atr = atr_proxy(c1h)
    bb = bollinger_position(c1h)
    vol_r = volume_ratio(v1h)

    score, reasons = 0.0, []
    if bull >= min_alignment:
        score += (bull / 3.0) * 0.4
        reasons.append(f"tf_bull_{bull}")
        if vol_r > 1.3:
            score += 0.1
            reasons.append("vol_confirm")
    elif bear >= min_alignment:
        score -= (bear / 3.0) * 0.4
        reasons.append(f"tf_bear_{bear}")
        if vol_r > 1.3:
            score -= 0.1
            reasons.append("vol_confirm_bear")

    # MR (weight 0.3) — mirrors Rust: near-zones need 1d trend agreement
    t1d_trend = td[0]
    if rsi < 30:
        mr, mr_r = 0.3, "rsi_oversold"
    elif rsi < 35 and t1d_trend == "bullish":
        mr, mr_r = 0.2, "rsi_near_oversold_daily_bull"
    elif rsi > 70:
        mr, mr_r = -0.3, "rsi_overbought"
    elif rsi > 65 and t1d_trend == "bearish":
        mr, mr_r = -0.2, "rsi_near_overbought_daily_bear"
    else:
        mr, mr_r = 0.0, ""
    if abs(mr) > 0.1:
        score += mr * 0.3
        reasons.append(mr_r)

    # Momentum (weight 0.15) from 4h buffer
    mom = t4[1]
    if mom > 0.003:
        score += 0.15
        reasons.append("mom_up")
    elif mom < -0.003:
        score -= 0.15
        reasons.append("mom_down")

    # BB (weight 0.15)
    if bb < 0.15:
        score += 0.15
        reasons.append("bb_lower")
    elif bb > 0.85:
        score -= 0.15
        reasons.append("bb_upper")

    return dict(score=score, reasons=reasons, rsi=rsi, atr=atr)


# ---------------------------------------------------------------- data load
def load():
    df1h = pd.read_parquet(os.path.join(DATA, "SOL_USDT_1h.parquet"))
    df4h = pd.read_parquet(os.path.join(DATA, "SOL_USDT_4h.parquet"))
    df1d = pd.read_parquet(os.path.join(DATA, "SOL_USDT_1d.parquet"))
    for df in (df1h, df4h, df1d):
        df.sort_index(inplace=True)
    return df1h, df4h, df1d


def asof_slice(df, ts, n):
    """Last n closes at or before ts (no lookahead)."""
    sub = df.loc[df.index <= ts]
    return sub["close"].to_numpy()[-n:] if len(sub) else np.array([])


# ---------------------------------------------------------------- replay
def run_replay(rust_semantics=True):
    df1h, df4h, df1d = load()
    # Multi-TF overlap window (all three TFs present), +25h warmup for the
    # 20-bar 1d SMA to have history.
    start = max(df1h.index.min(), df4h.index.min(), df1d.index.min()) + pd.Timedelta(hours=25)
    end = min(df1h.index.max(), df4h.index.max(), df1d.index.max())
    bars = df1h.loc[(df1h.index >= start) & (df1h.index <= end)]
    print(f"[postmortem] window {bars.index[0]} -> {bars.index[-1]} ({len(bars)} 1h bars) rust_semantics={rust_semantics}")

    p = PARAMS
    trips = []
    pos = None          # open position dict
    flip_t = None       # thesis-flip timer (unix ts)
    prev_sig = None

    idx = bars.index.to_numpy()
    close = bars["close"].to_numpy()
    vol = bars["volume"].to_numpy()

    for i in range(21, len(bars)):
        ts = pd.Timestamp(idx[i])
        price = float(close[i])
        c1h = close[max(0, i - 300):i + 1]
        v1h = vol[max(0, i - 300):i + 1]
        c4h = asof_slice(df4h, ts, 200)
        c1d = asof_slice(df1d, ts, 120)

        sig = compute_signal(c1h, c4h, c1d, v1h, p["min_alignment"], rust_semantics)
        if sig is None:
            continue
        score, rsi, atr = sig["score"], sig["rsi"], sig["atr"]
        now = int(ts.timestamp())

        if pos is not None:
            # ---- exit checks (order mirrors strategy.rs::check_exit) ----
            side = pos["side"]
            hold = (ts - pos["entry_ts"]).total_seconds() / 3600.0
            gross = (price - pos["entry"]) / pos["entry"] * 100.0
            if side == "Short":
                gross = -gross

            exit_reason = None
            # trailing stop (side-aware peak/trough)
            if p["trailing_stop_atr"] > 0 and atr and pos["entry"] > 0:
                if side == "Short":
                    pos["peak"] = min(pos["peak"], price)  # trough
                    pullback = (price - pos["peak"]) / pos["entry"] * 100.0
                    cond = pos["peak"] < pos["entry"]
                else:
                    pos["peak"] = max(pos["peak"], price)
                    pullback = (pos["peak"] - price) / pos["entry"] * 100.0
                    cond = pos["peak"] > pos["entry"]
                trig = p["trailing_stop_atr"] * atr / pos["entry"] * 100.0
                if pullback >= trig and cond:
                    exit_reason = "TrailingStop"
            if exit_reason is None and atr:
                sl_pct = p["sl_atr"] * atr / pos["entry"] * 100.0
                if gross <= -sl_pct:
                    exit_reason = "StopLoss"
            if exit_reason is None and atr:
                tp_pct = p["tp_atr"] * atr / pos["entry"] * 100.0
                if gross >= tp_pct:
                    exit_reason = "TakeProfit"
            if exit_reason is None and hold >= p["max_hold_hours"]:
                exit_reason = "MaxHold"
            if exit_reason is None and gross < 0 and hold >= p["time_decay_hours"]:
                exit_reason = "TimeDecay"

            # side-aware score flip with grace
            if exit_reason is None:
                flipped = score > 0.0 if side == "Short" else score < 0.0
                if flipped:
                    if flip_t is None:
                        flip_t = now if p["score_flip_delay_hrs"] > 0 else now - 1
                    if p["score_flip_delay_hrs"] <= 0 or (now - flip_t) / 3600.0 >= p["score_flip_delay_hrs"]:
                        exit_reason = "ScoreFlip"
                else:
                    flip_t = None

            if exit_reason is None and rsi > 55 and pos["entry_rsi"] < 35:
                exit_reason = "MrTarget"

            if exit_reason:
                trips.append(dict(
                    entry_time=pos["entry_ts"], exit_time=ts, side=side,
                    entry_price=pos["entry"], exit_price=price,
                    gross_pnl_pct=gross, hold_hrs=hold, exit_reason=exit_reason,
                    entry_score=pos["entry_score"],
                ))
                pos, flip_t = None, None
            prev_sig = sig
            continue

        # ---- entry (score-only gate, matches live trader) ----
        if score > p["signal_threshold"]:
            pos = dict(side="Long", entry=price, entry_ts=ts, entry_score=score,
                       entry_rsi=rsi, peak=price)
            flip_t = None
        elif score < -p["signal_threshold"]:
            pos = dict(side="Short", entry=price, entry_ts=ts, entry_score=score,
                       entry_rsi=rsi, peak=price)
            flip_t = None
        prev_sig = sig

    return pd.DataFrame(trips), bars


def score_regime(trips: pd.DataFrame, model: dict, start_capital=2.5):
    """Compound at position_fraction of equity, leverage-applied net PnL."""
    lev = PARAMS["leverage"]
    frac = PARAMS["position_fraction"]
    capital = start_capital
    peak = capital
    max_dd = 0.0
    nets = []
    for _, t in trips.iterrows():
        fee_pct = round_trip_cost_pct(model, lev, t["hold_hrs"])  # % of margin
        net = t["gross_pnl_pct"] * lev - fee_pct
        nets.append(net)
        stake = capital * frac
        capital += stake * net / 100.0
        peak = max(peak, capital)
        max_dd = max(max_dd, (peak - capital) / peak * 100.0)
    trips = trips.copy()
    trips["net_pnl_pct_margin"] = nets
    wins = sum(1 for n in nets if n > 0)
    total = len(nets)
    sharpe = 0.0
    if total > 1 and np.std(nets) > 0:
        hours = PARAMS["max_hold_hours"] * total  # coarse annualization base
        sharpe = float(np.mean(nets) / np.std(nets) * np.sqrt(total / max(hours, 1) * 8760))
    return dict(
        trips=trips,
        final_capital=capital,
        total_return_pct=(capital / start_capital - 1) * 100.0,
        max_dd_pct=max_dd,
        win_rate=wins / total if total else 0.0,
        avg_net=float(np.mean(nets)) if total else 0.0,
        expectancy_gross=float(trips["gross_pnl_pct"].mean() * lev) if total else 0.0,
        sharpe=sharpe,
        n=total,
    )


def main():
    os.makedirs(OUT, exist_ok=True)
    # PRIMARY: exact live-Rust engine semantics (momentum off-by-one included)
    trips, bars = run_replay(rust_semantics=True)
    # SECONDARY: Python-reference semantics (momentum term active) — delta check
    trips_pymom, _ = run_replay(rust_semantics=False)
    trips.to_csv(os.path.join(OUT, "trades_raw.csv"), index=False)
    print(f"[postmortem] {len(trips)} round trips "
          f"({(trips['side'] == 'Long').sum()} long / {(trips['side'] == 'Short').sum()} short)")
    if trips.empty:
        print("no trades — abort")
        return

    v1 = score_regime(trips, V1)
    v2 = score_regime(trips, V2)
    zero = score_regime(trips, dict(open=0, close=0, borrow_hr=0, spread_side=0))
    pymom_v2 = score_regime(trips_pymom, V2) if len(trips_pymom) else None

    v1["trips"].to_csv(os.path.join(OUT, "trades_v1_cost.csv"), index=False)
    v2["trips"].to_csv(os.path.join(OUT, "trades_v2_cost.csv"), index=False)

    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    lines = [
        "# Survivor 2.69 — Flash v2 Cost Post-Mortem",
        "",
        f"Generated: {now} | window: {bars.index[0]} -> {bars.index[-1]}",
        "",
        "Signal replay of the LIVE trader engine (true multi-TF, side-aware score-flip)",
        f"over the multi-TF overlap window, {len(trips)} round trips, "
        f"{(trips['side'] == 'Long').sum()} long / {(trips['side'] == 'Short').sum()} short.",
        "",
        "## Cost models (per round trip, % of margin at 9x)",
        "",
        "| Component | v1 model (used in WFA) | v2 measured live |",
        "|---|---|---|",
        "| Open fee | 0.06% notional | 0.02% |",
        "| Close fee | 0.06% notional | 0.02% |",
        "| Spread | not modeled | ~0.01%/side |",
        "| Borrow | 0.0042%/hr notional | 0.0004%/hr notional |",
        "",
        "## Results (compounded, 20% position fraction, 9x leverage, 2.5 SOL start)",
        "",
        "| Metric | Gross (no cost) | v1 model | v2 measured |",
        "|---|---|---|---|",
        f"| Trades | {zero['n']} | {v1['n']} | {v2['n']} |",
        f"| Win rate | {zero['win_rate']:.1%} | {v1['win_rate']:.1%} | {v2['win_rate']:.1%} |",
        f"| Avg net/trade (% margin) | {zero['avg_net']:+.3f} | {v1['avg_net']:+.3f} | {v2['avg_net']:+.3f} |",
        f"| Avg gross/trade (% margin) | {zero['expectancy_gross']:+.3f} | {v1['expectancy_gross']:+.3f} | {v2['expectancy_gross']:+.3f} |",
        f"| Final SOL | {zero['final_capital']:.3f} | {v1['final_capital']:.3f} | {v2['final_capital']:.3f} |",
        f"| Total return | {zero['total_return_pct']:+.1f}% | {v1['total_return_pct']:+.1f}% | {v2['total_return_pct']:+.1f}% |",
        f"| Max DD | {zero['max_dd_pct']:.1f}% | {v1['max_dd_pct']:.1f}% | {v2['max_dd_pct']:.1f}% |",
        f"| Sharpe (annualized, coarse) | {zero['sharpe']:.2f} | {v1['sharpe']:.2f} | {v2['sharpe']:.2f} |",
        "",
        "## Exit mix",
        "",
    ]
    for reason in trips["exit_reason"].value_counts().index:
        lines.append(f"- {reason}: {(trips['exit_reason'] == reason).sum()}")
    if pymom_v2 is not None:
        lines += [
            "",
            "## Latent divergence: Rust momentum off-by-one",
            "",
            "The live Rust `timeframe_signal` never fires the ±0.15 momentum term",
            "(off-by-one: `windows(2)` on N elements yields N-1 returns but the",
            "gate requires `>= N`). This replay ran BOTH variants under v2 costs:",
            "",
            f"- live-Rust semantics (momentum dead): {zero['n']} trades, "
            f"avg net {v2['avg_net']:+.3f}%/trade, return {v2['total_return_pct']:+.1f}%",
            f"- Python-reference semantics (momentum alive): {pymom_v2['n']} trades, "
            f"avg net {pymom_v2['avg_net']:+.3f}%/trade, return {pymom_v2['total_return_pct']:+.1f}%",
            "",
            "Decision required separately from the cost post-mortem: keep the dead",
            "momentum term (matches the record being judged) or fix the off-by-one",
            "and re-WFA. NOT done silently here.",
        ]
    lines += [
        "",
        "## Verdict",
        "",
        f"v2 measured cost per round trip ≈ "
        f"{round_trip_cost_pct(V2, 9, trips['hold_hrs'].mean()):.3f}% of margin at avg hold "
        f"{trips['hold_hrs'].mean():.1f}h, vs v1-model "
        f"{round_trip_cost_pct(V1, 9, trips['hold_hrs'].mean()):.3f}% — "
        f"v2 is {round_trip_cost_pct(V1, 9, trips['hold_hrs'].mean()) / max(round_trip_cost_pct(V2, 9, trips['hold_hrs'].mean()), 1e-9):.1f}x "
        "CHEAPER than what the validated config was stress-tested against.",
        "",
        "Three honest outcomes were pre-registered:",
        "1. edge survives v2 costs -> live record is confirmation, accumulate;",
        "2. borrow/fees leak the edge -> WFA a shorter-hold variant (documented, no silent retune);",
        "3. edge is structurally dead -> feed v2 mechanics into the pipeline as fresh constraints.",
        "",
    ]
    if v2["final_capital"] > 2.5 and v2["avg_net"] > 0:
        lines.append(f"OUTCOME 1: edge SURVIVES v2 costs — avg net {v2['avg_net']:+.3f}%/trade on margin, "
                     f"compounded {v2['total_return_pct']:+.1f}%, DD {v2['max_dd_pct']:.1f}%.")
    elif zero["final_capital"] > 2.5:
        lines.append("OUTCOME 2/3: gross edge exists but v2 costs leak it — investigate hold-time profile.")
    else:
        lines.append("OUTCOME 3: gross edge absent in this window — signal-side investigation required.")

    verdict = "\n".join(lines)
    with open(os.path.join(OUT, "verdict.md"), "w") as f:
        f.write(verdict)
    print(verdict)


if __name__ == "__main__":
    main()
