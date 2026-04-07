"""
Per-Symbol Parameter Optimization (v3 — fast, indicator-based).

Instead of running the full simulator for each param combo, we:
1. Compute all indicators once per symbol
2. Simulate trades directly on the indicator arrays
3. Test ~1000 param combos per symbol in seconds, not hours
"""
import os
import sys
import json
import numpy as np
import pandas as pd
from datetime import datetime
from typing import Dict, List, Tuple
from itertools import product

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")
SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT"]


def log(msg):
    print(msg, flush=True)


def compute_indicators(df: pd.DataFrame) -> pd.DataFrame:
    """Compute all indicators once — add columns to df."""
    close = df["close"]
    high = df["high"]
    low = df["low"]
    volume = df["volume"]

    # Multi-TF signals (simulated via lookback on 1h)
    for name, lookback in [("1h", 20), ("4h", 80), ("1d", 200)]:
        sma = close.rolling(lookback).mean()
        col = f"trend_{name}"
        df[col] = np.where(close > sma, 1, np.where(close < sma, -1, 0))

    # Alignment counts
    df["bullish_count"] = (
        (df["trend_1h"] == 1).astype(int) +
        (df["trend_4h"] == 1).astype(int) +
        (df["trend_1d"] == 1).astype(int)
    )
    df["bearish_count"] = (
        (df["trend_1h"] == -1).astype(int) +
        (df["trend_4h"] == -1).astype(int) +
        (df["trend_1d"] == -1).astype(int)
    )

    # RSI (14)
    delta = close.diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss
    df["rsi"] = 100 - (100 / (1 + rs))

    # Bollinger Bands
    sma20 = close.rolling(20).mean()
    std20 = close.rolling(20).std()
    df["bb_upper"] = sma20 + 2 * std20
    df["bb_lower"] = sma20 - 2 * std20
    df["bb_middle"] = sma20
    df["bb_pos"] = (close - df["bb_lower"]) / (4 * std20)

    # Momentum (80h = 4h TF)
    df["momentum_4h"] = close.pct_change().rolling(80).mean()

    # Volume ratio
    df["vol_ratio"] = volume / volume.rolling(20).mean()

    # ATR — MUST match FutureBlindSimulator's ATR proxy exactly:
    # Full sim uses: tf_1h["volatility"] * close = std(returns, 20h) * close
    # NOT True Range. Using the wrong ATR caused 1.6x wider stops in fast sim.
    df["atr"] = close.pct_change().rolling(20).std() * close

    # Volatility (% return std, 20h) — kept for backward compat
    df["volatility"] = close.pct_change().rolling(20).std()

    return df


def simulate_trades(df: pd.DataFrame, params: Dict) -> List[Dict]:
    """Simulate trades directly on indicator arrays — no simulator overhead."""
    threshold = params["signal_threshold"]
    min_alignment = params.get("min_alignment", 3)
    tp_atr_mult = params["take_profit_atr"]
    sl_atr_mult = params["stop_loss_atr"]
    max_hold = params["max_hold_hours"]
    decay_hours = params["time_decay_hours"]

    trips = []
    in_position = False
    entry_price = 0
    entry_idx = 0
    entry_score = 0
    entry_rsi = 50
    peak_price = 0
    trailing_atr = params.get("trailing_stop_atr", 0)

    close = df["close"].values
    atr = df["atr"].values
    rsi = df["rsi"].values
    bull_count = df["bullish_count"].values
    bear_count = df["bearish_count"].values
    mom = df["momentum_4h"].values
    bb_pos = df["bb_pos"].values
    vol_ratio = df["vol_ratio"].values
    # Daily trend — needed for MR conditions (must match full sim)
    daily_bull = df["trend_1d"].values if "trend_1d" in df.columns else np.zeros(len(df))
    daily_bear = -daily_bull  # trend_1d is 1, 0, or -1

    warmup = 250  # skip first 250 bars for indicator warmup

    for i in range(warmup, len(close)):
        price = close[i]
        a = atr[i] if not np.isnan(atr[i]) else 0
        r = rsi[i] if not np.isnan(rsi[i]) else 50

        if in_position:
            hold_hrs = i - entry_idx
            pnl_pct = (price - entry_price) / entry_price * 100

            # Update peak for trailing stop
            if price > peak_price:
                peak_price = price

            # Trailing stop: exit if price drops N ATR from peak
            if trailing_atr > 0 and a > 0 and peak_price > entry_price:
                trail_trigger = trailing_atr * a / entry_price * 100
                pullback_pct = (peak_price - price) / entry_price * 100
                if pullback_pct >= trail_trigger:
                    trail_pnl = (peak_price - entry_price) / entry_price * 100 - trail_trigger
                    trips.append({"pnl_pct": trail_pnl, "hold_hrs": hold_hrs, "exit": "trailing_stop"})
                    in_position = False
                    continue

            # Stop loss
            if a > 0 and pnl_pct <= -(sl_atr_mult * a / entry_price * 100):
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "stop_loss"})
                in_position = False
                continue

            # Take profit
            if a > 0 and pnl_pct >= (tp_atr_mult * a / entry_price * 100):
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "take_profit"})
                in_position = False
                continue

            # Max hold
            if hold_hrs >= max_hold:
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "max_hold"})
                in_position = False
                continue

            # Time decay
            if pnl_pct < 0 and hold_hrs >= decay_hours:
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "time_decay"})
                in_position = False
                continue

            # Compute score for exit check
            score = _compute_score(bull_count[i], bear_count[i], r, mom[i], bb_pos[i], vol_ratio[i],
                                   threshold, min_alignment,
                                   daily_bull=(daily_bull[i] == 1),
                                   daily_bear=(daily_bear[i] == 1))
            if score < 0:
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "score_flip"})
                in_position = False
                continue

            # MR target
            if r > 55 and entry_rsi < 35:
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs, "exit": "mr_target"})
                in_position = False
                continue
        else:
            # Entry
            score = _compute_score(bull_count[i], bear_count[i], r, mom[i], bb_pos[i], vol_ratio[i],
                                   threshold, min_alignment,
                                   daily_bull=(daily_bull[i] == 1),
                                   daily_bear=(daily_bear[i] == 1))
            if score > threshold:
                in_position = True
                entry_price = price
                entry_idx = i
                entry_score = score
                entry_rsi = r
                peak_price = price

    return trips


def _compute_score(bull, bear, rsi, mom, bb_pos, vol_ratio, threshold, min_alignment,
                    daily_bull=False, daily_bear=False):
    """Replicate MultiTFStrategy._compute_score logic exactly.

    Args:
        daily_bull: True if daily (200h) trend is bullish. Used for MR conditions.
        daily_bear: True if daily trend is bearish. Used for MR conditions.
    """
    score = 0.0

    # Trend alignment (weight 0.4)
    if bull >= min_alignment:
        score += (bull / 3.0) * 0.4
        if vol_ratio > 1.3:
            score += 0.1
    elif bear >= min_alignment:
        score -= (bear / 3.0) * 0.4
        if vol_ratio > 1.3:
            score -= 0.1

    # Mean reversion (weight 0.3) — MUST match full sim exactly
    if np.isnan(rsi):
        rsi = 50
    if rsi < 30:
        score += 0.3
    elif rsi < 35 and daily_bull:
        score += 0.2
    elif rsi > 70:
        score -= 0.3
    elif rsi > 65 and daily_bear:
        score -= 0.2

    # Momentum (weight 0.15)
    if np.isnan(mom):
        mom = 0
    if mom > 0.003:
        score += 0.15
    elif mom < -0.003:
        score -= 0.15

    # BB (weight 0.15)
    if np.isnan(bb_pos):
        bb_pos = 0.5
    if bb_pos < 0.15:
        score += 0.15
    elif bb_pos > 0.85:
        score -= 0.15

    return score


def compute_metrics(trips: List[Dict], total_hours: float = 0) -> Dict:
    """Compute performance metrics from trips.

    Args:
        total_hours: Total time span of the backtest window in hours.
                     Used for correct Sharpe annualization. Falls back to
                     trade hold-time estimate if not provided.
    """
    if not trips:
        return {"round_trips": 0, "win_rate": 0, "total_pnl_pct": 0, "pf": 0,
                "sharpe": 0, "max_dd_pct": 0, "avg_pnl_pct": 0, "avg_hold_hrs": 0}

    pnls = [t["pnl_pct"] for t in trips]
    wins = [p for p in pnls if p > 0]
    losses = [p for p in pnls if p <= 0]

    cum = np.cumsum(pnls)
    running_max = np.maximum.accumulate(cum)
    drawdowns = cum - running_max
    max_dd = abs(min(drawdowns)) if len(drawdowns) > 0 else 0

    # Sharpe annualization: use actual trade frequency, not assume 1/hour.
    # Old formula assumed 1 trade/hour → 10x overstatement for ~15 trades/36 days.
    # Correct: sqrt(n_trades / total_hours * 8760)
    std_pnl = np.std(pnls)
    if std_pnl > 0:
        raw_sharpe = np.mean(pnls) / std_pnl
        if total_hours > 0:
            trades_per_year = (len(pnls) / total_hours) * 8760
        else:
            # Estimate from trade hold times (accounts for sequential trades)
            total_hold = sum(t["hold_hrs"] for t in trips)
            trades_per_year = (len(pnls) / max(total_hold, 1)) * 8760
        sharpe = raw_sharpe * np.sqrt(max(trades_per_year, 0.1))
    else:
        sharpe = 0.0
    avg_win = np.mean(wins) if wins else 0
    avg_loss = abs(np.mean(losses)) if losses else 0
    pf = avg_win / avg_loss if avg_loss > 0 else 999

    exit_reasons = {}
    for t in trips:
        e = t.get("exit", "signal")
        exit_reasons[e] = exit_reasons.get(e, 0) + 1

    return {
        "round_trips": len(trips),
        "win_rate": len(wins) / len(pnls),
        "total_pnl_pct": sum(pnls),
        "avg_pnl_pct": np.mean(pnls),
        "pf": pf,
        "sharpe": sharpe,
        "max_dd_pct": max_dd,
        "avg_hold_hrs": np.mean([t["hold_hrs"] for t in trips]),
        "best_trade": max(pnls),
        "worst_trade": min(pnls),
        "exit_reasons": exit_reasons,
    }


def optimize_symbol(df: pd.DataFrame, symbol: str) -> Dict:
    """Fast grid search on precomputed indicators."""
    grid = {
        "signal_threshold": [0.30, 0.35, 0.40, 0.45, 0.50],
        "take_profit_atr": [3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        "stop_loss_atr": [1.5, 2.0, 2.5, 3.0, 3.5],
        "max_hold_hours": [48, 72, 96, 120],
        "time_decay_hours": [24, 48],
    }

    combos = list(product(
        grid["signal_threshold"],
        grid["take_profit_atr"],
        grid["stop_loss_atr"],
        grid["max_hold_hours"],
        grid["time_decay_hours"],
    ))

    log(f"  Testing {len(combos)} combos for {symbol}...")

    results = []
    for i, (thresh, tp, sl, hold, decay) in enumerate(combos):
        params = {
            "signal_threshold": thresh,
            "min_alignment": 3,
            "take_profit_atr": tp,
            "stop_loss_atr": sl,
            "max_hold_hours": hold,
            "time_decay_hours": decay,
        }

        trips = simulate_trades(df, params)
        m = compute_metrics(trips)
        m["params"] = params
        results.append(m)

        if (i + 1) % 200 == 0:
            log(f"    [{symbol}] {i+1}/{len(combos)} done...")

    # Sort by total PnL
    results.sort(key=lambda x: x["total_pnl_pct"], reverse=True)

    # Filter: PF > 1.5 and 15+ trades
    valid = [r for r in results if r["pf"] > 1.5 and r["round_trips"] >= 15]

    log(f"    [{symbol}] All {len(combos)} combos done. {len(valid)} passed PF>1.5 filter.")

    return {
        "symbol": symbol,
        "total_combos": len(combos),
        "valid_combos": len(valid),
        "best": valid[0] if valid else (results[0] if results else None),
        "top5_valid": valid[:5],
        "top5_raw": results[:5],
    }


def main():
    log(f"\n{'='*80}")
    log(f"PER-SYMBOL OPTIMIZATION (v3 — fast indicator-based)")
    log(f"Symbols: {', '.join(SYMBOLS)}")
    log(f"{'='*80}")

    global_config = {
        "signal_threshold": 0.40,
        "min_alignment": 3,
        "take_profit_atr": 6.0,
        "stop_loss_atr": 2.5,
        "max_hold_hours": 96,
        "time_decay_hours": 48,
    }

    all_results = {}
    all_dfs = {}

    # Precompute indicators
    for symbol in SYMBOLS:
        safe = symbol.replace("/", "_")
        path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
        df = pd.read_parquet(path)
        log(f"\n  Loading {symbol}: {len(df)} candles, computing indicators...")
        df = compute_indicators(df)
        all_dfs[symbol] = df

    # Run optimization
    for symbol in SYMBOLS:
        log(f"\n{'─'*80}")
        log(f"── {symbol} ──")
        r = optimize_symbol(all_dfs[symbol], symbol)
        all_results[symbol] = r

        best = r.get("best")
        if best:
            p = best["params"]
            pf_s = f"{best['pf']:.1f}" if best['pf'] < 999 else "INF"
            log(f"\n  BEST (PF>1.5 filter):")
            log(f"    thresh={p['signal_threshold']} TP={p['take_profit_atr']}x SL={p['stop_loss_atr']}x hold={p['max_hold_hours']}h decay={p['time_decay_hours']}h")
            log(f"    PnL={best['total_pnl_pct']:.2f}% WR={best['win_rate']*100:.1f}% PF={pf_s} "
                f"Sharpe={best['sharpe']:.2f} DD={best['max_dd_pct']:.2f}% Trades={best['round_trips']}")
            log(f"    Exits: {best.get('exit_reasons', {})}")

        # Global baseline
        log(f"\n  Global config baseline:")
        trips = simulate_trades(all_dfs[symbol], global_config)
        gm = compute_metrics(trips)
        pf_s = f"{gm['pf']:.1f}" if gm['pf'] < 999 else "INF"
        log(f"    PnL={gm['total_pnl_pct']:.2f}% WR={gm['win_rate']*100:.1f}% PF={pf_s} "
            f"Sharpe={gm['sharpe']:.2f} DD={gm['max_dd_pct']:.2f}% Trades={gm['round_trips']}")
        all_results[symbol]["global_baseline"] = gm

    # Summary table
    log(f"\n\n{'='*80}")
    log(f"SUMMARY — Per-Symbol Best vs Global Config")
    log(f"{'='*80}")
    log(f"  {'Symbol':12s} {'Config':12s} {'PnL':>8s} {'WR':>6s} {'PF':>5s} {'Sharpe':>7s} {'DD':>7s} {'Trades':>7s}")
    log(f"  {'─'*12} {'─'*12} {'─'*8} {'─'*6} {'─'*5} {'─'*7} {'─'*7} {'─'*7}")

    recommendations = {}
    for symbol in SYMBOLS:
        r = all_results[symbol]
        best = r.get("best")
        baseline = r.get("global_baseline")

        if baseline:
            pf_s = f"{baseline['pf']:.1f}" if baseline['pf'] < 999 else "INF"
            log(f"  {symbol:12s} {'GLOBAL':12s} {baseline['total_pnl_pct']:+7.2f}% {baseline['win_rate']*100:5.1f}% {pf_s:>5s} {baseline['sharpe']:7.2f} {baseline['max_dd_pct']:6.2f}% {baseline['round_trips']:7d}")

        if best:
            p = best["params"]
            pf_s = f"{best['pf']:.1f}" if best['pf'] < 999 else "INF"
            tag = "OPTIMIZED" if best["total_pnl_pct"] > baseline["total_pnl_pct"] else "WORSE"
            log(f"  {symbol:12s} {tag:12s} {best['total_pnl_pct']:+7.2f}% {best['win_rate']*100:5.1f}% {pf_s:>5s} {best['sharpe']:7.2f} {best['max_dd_pct']:6.2f}% {best['round_trips']:7d}")

            if best["total_pnl_pct"] > baseline["total_pnl_pct"] and best["pf"] > baseline["pf"]:
                recommendations[symbol] = {"action": "use_optimized", "params": p, "pnl": best["total_pnl_pct"], "pf": best["pf"]}
            else:
                recommendations[symbol] = {"action": "keep_global", "reason": "optimized not better",
                                           "opt_pnl": best["total_pnl_pct"], "global_pnl": baseline["total_pnl_pct"]}

    log(f"\n  RECOMMENDATIONS:")
    for symbol, rec in recommendations.items():
        if rec["action"] == "use_optimized":
            p = rec["params"]
            log(f"    {symbol}: SWITCH to optimized → TP={p['take_profit_atr']}x SL={p['stop_loss_atr']}x hold={p['max_hold_hours']}h thresh={p['signal_threshold']}")
        else:
            log(f"    {symbol}: KEEP global (optimized PnL={rec.get('opt_pnl',0):.1f}% vs global={rec.get('global_pnl',0):.1f}%)")

    # Save
    out_path = os.path.join(DATA_DIR, "..", "per_symbol_optimization.json")
    with open(out_path, "w") as f:
        json.dump({
            "run_at": datetime.now().isoformat(),
            "results": {sym: {
                "best_params": r.get("best", {}).get("params"),
                "best_pnl": r.get("best", {}).get("total_pnl_pct"),
                "best_pf": r.get("best", {}).get("pf"),
                "best_sharpe": r.get("best", {}).get("sharpe"),
                "global_pnl": r.get("global_baseline", {}).get("total_pnl_pct"),
                "global_pf": r.get("global_baseline", {}).get("pf"),
            } for sym, r in all_results.items()},
            "recommendations": recommendations,
        }, f, indent=2, default=str)

    log(f"\n  Saved to {out_path}")


if __name__ == "__main__":
    main()
