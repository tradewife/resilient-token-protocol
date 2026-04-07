"""
Backtest Round 3: Stateful multi-TF strategy with proper position management.

Key fixes from R2:
- Stateful: strategy tracks open positions, won't double-buy
- Asymmetric exits: sell to close when score deteriorates (not just on bearish reversal)
- ATR stop-loss and take-profit actually enforced in analyze()
- Max hold time forces exit after N hours
- Mean reversion targets: close longs when RSI reverts to mean (50), not only on bearish signals
"""
import asyncio
import os
import sys
import json
import numpy as np
import pandas as pd
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from backtesting.future_blind_simulator import (
    FutureBlindSimulator, TradingStrategy, TradeSignal, Trade
)

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")
SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT", "XRP/USDT"]


def load_multi_tf(symbol: str) -> Dict[str, pd.DataFrame]:
    """Load 1h, 4h, 1d data and align 4h/1d to 1h index"""
    safe = symbol.replace("/", "_")
    result = {}
    for tf in ["1h", "4h", "1d"]:
        path = os.path.join(DATA_DIR, f"{safe}_{tf}.parquet")
        if os.path.exists(path):
            df = pd.read_parquet(path)
            if tf != "1h":
                df = df.resample("1h").ffill()
            result[tf] = df
    return result


def timeframe_signal(close: pd.Series, lookback: int) -> Dict:
    """Compute trend signal for a single timeframe slice"""
    if len(close) < lookback:
        return {"trend": "neutral", "strength": 0.0, "rsi": 50}

    sma = close.rolling(lookback).mean()
    price = close.iloc[-1]
    sma_val = sma.iloc[-1]

    if price > sma_val:
        trend = "bullish"
        strength = min((price - sma_val) / sma_val * 100, 2.0)
    elif price < sma_val:
        trend = "bearish"
        strength = min((sma_val - price) / sma_val * 100, 2.0)
    else:
        trend = "neutral"
        strength = 0.0

    # RSI
    delta = close.diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss
    rsi = float((100 - (100 / (1 + rs))).iloc[-1]) if loss.iloc[-1] != 0 else 50

    # Momentum
    returns = close.pct_change()
    momentum = float(returns.rolling(lookback).mean().iloc[-1]) if len(returns) >= lookback else 0

    # Volatility (ATR proxy)
    vol = float(returns.rolling(lookback).std().iloc[-1]) if len(returns) >= lookback else 0

    return {
        "trend": trend,
        "strength": strength,
        "rsi": rsi,
        "momentum": momentum,
        "volatility": vol,
        "price": price,
        "sma": sma_val,
    }


class MultiTFStrategy(TradingStrategy):
    """Stateful multi-TF strategy: buys on confluence, sells on deterioration or stops"""

    def __init__(self, name, params=None):
        super().__init__(name, params or {})
        self.open_positions = {}  # symbol -> {entry_price, entry_time, score, atr}
        self.completed_round_trips = []  # filled during exits

    def reset(self):
        super().reset()
        self.open_positions = {}
        self.completed_round_trips = []

    def _compute_score(self, data: pd.DataFrame) -> Tuple[float, list, dict]:
        """Compute multi-TF confluence score. Returns (score, reasons, extras)."""
        close = data["close"]
        vol_series = data["volume"]

        tf_1h = timeframe_signal(close, 20)
        tf_4h = timeframe_signal(close, 80)
        tf_1d = timeframe_signal(close, 200)

        trends = [tf_1h["trend"], tf_4h["trend"], tf_1d["trend"]]
        bullish_count = sum(1 for t in trends if t == "bullish")
        bearish_count = sum(1 for t in trends if t == "bearish")

        rsi = tf_1h["rsi"]
        mr_signal = 0.0
        mr_reason = ""

        if rsi < 30:
            mr_signal = 0.3
            mr_reason = "rsi_oversold"
        elif rsi < 35 and tf_1d["trend"] == "bullish":
            mr_signal = 0.2
            mr_reason = "rsi_near_oversold_daily_bull"
        elif rsi > 70:
            mr_signal = -0.3
            mr_reason = "rsi_overbought"
        elif rsi > 65 and tf_1d["trend"] == "bearish":
            mr_signal = -0.2
            mr_reason = "rsi_near_overbought_daily_bear"

        sma20 = close.rolling(20).mean()
        std20 = close.rolling(20).std()
        bb_pos = (close.iloc[-1] - (sma20.iloc[-1] - 2 * std20.iloc[-1])) / (4 * std20.iloc[-1]) if std20.iloc[-1] > 0 else 0.5

        vol_ratio = vol_series.iloc[-1] / vol_series.rolling(20).mean().iloc[-1] if vol_series.rolling(20).mean().iloc[-1] > 0 else 1

        score = 0.0
        reasons = []
        min_alignment = self.params.get("min_alignment", 2)

        # 1. Multi-TF trend alignment (weight: 0.4)
        if bullish_count >= min_alignment:
            score += (bullish_count / 3.0) * 0.4
            reasons.append(f"tf_bull_{bullish_count}")
            if vol_ratio > 1.3:
                score += 0.1
                reasons.append("vol_confirm")
        elif bearish_count >= min_alignment:
            score -= (bearish_count / 3.0) * 0.4
            reasons.append(f"tf_bear_{bearish_count}")
            if vol_ratio > 1.3:
                score -= 0.1
                reasons.append("vol_confirm_bear")

        # 2. Mean reversion (weight: 0.3)
        if abs(mr_signal) > 0.1:
            score += mr_signal * 0.3
            reasons.append(mr_reason)

        # 3. Momentum (weight: 0.15)
        mom = tf_4h["momentum"]
        if mom > 0.003:
            score += 0.15
            reasons.append("mom_up")
        elif mom < -0.003:
            score -= 0.15
            reasons.append("mom_down")

        # 4. Bollinger Band (weight: 0.15)
        if bb_pos < 0.15:
            score += 0.15
            reasons.append("bb_lower")
        elif bb_pos > 0.85:
            score -= 0.15
            reasons.append("bb_upper")

        atr = tf_1h["volatility"] * close.iloc[-1] if tf_1h["volatility"] > 0 else close.iloc[-1] * 0.02

        extras = {
            "rsi": rsi,
            "atr": atr,
            "bb_pos": bb_pos,
            "vol_ratio": vol_ratio,
            "bullish_count": bullish_count,
            "bearish_count": bearish_count,
        }
        return score, reasons, extras

    async def analyze(self, data: pd.DataFrame, current_time: datetime) -> Optional[TradeSignal]:
        if len(data) < 100:
            return None

        symbol = self.params.get("symbol", "BTC/USDT")
        p = self.params
        price = data["close"].iloc[-1]

        score, reasons, extras = self._compute_score(data)
        rsi = extras["rsi"]
        atr = extras["atr"]

        # --- EXIT LOGIC (checked first if we have an open position) ---
        if symbol in self.open_positions:
            pos = self.open_positions[symbol]
            entry_price = pos["entry_price"]
            entry_time = pos["entry_time"]
            entry_score = pos["score"]
            hold_hrs = (current_time - entry_time).total_seconds() / 3600
            pnl_pct = (price - entry_price) / entry_price * 100

            # Trailing stop: ratchet up from peak, exit if price drops N ATR from peak
            trail_atr = p.get("trailing_stop_atr", 0)
            if trail_atr > 0 and atr > 0:
                peak = pos.get("peak_price", entry_price)
                if price > peak:
                    pos["peak_price"] = price
                    peak = price
                trail_trigger = trail_atr * atr / entry_price * 100
                pullback_pct = (peak - price) / entry_price * 100
                if pullback_pct >= trail_trigger and peak > entry_price:
                    trail_pnl = (peak - entry_price) / entry_price * 100 - trail_trigger
                    return self._sell_signal(current_time, symbol, price, "trailing_stop", score, reasons, extras, trail_pnl, hold_hrs)

            # Hard stop loss
            sl_atr = p.get("stop_loss_atr", 2.0)
            if atr > 0 and pnl_pct <= -(sl_atr * atr / entry_price * 100):
                return self._sell_signal(current_time, symbol, price, "stop_loss", score, reasons, extras, pnl_pct, hold_hrs)

            # Take profit
            tp_atr = p.get("take_profit_atr", 3.0)
            if atr > 0 and pnl_pct >= (tp_atr * atr / entry_price * 100):
                return self._sell_signal(current_time, symbol, price, "take_profit", score, reasons, extras, pnl_pct, hold_hrs)

            # Max hold time
            max_hold = p.get("max_hold_hours", 48)
            if hold_hrs >= max_hold:
                return self._sell_signal(current_time, symbol, price, "max_hold", score, reasons, extras, pnl_pct, hold_hrs)

            # Time decay: exit losing positions after decay period
            decay_hrs = p.get("time_decay_hours", 24)
            if pnl_pct < 0 and hold_hrs >= decay_hrs:
                return self._sell_signal(current_time, symbol, price, "time_decay", score, reasons, extras, pnl_pct, hold_hrs)

            # Asymmetric exit: close if score flips sign (we bought bullish, now bearish)
            # But NOT in the first N hours — let the trade breathe
            flip_delay = p.get("score_flip_delay_hrs", 0)
            if score < 0 and hold_hrs >= flip_delay:
                return self._sell_signal(current_time, symbol, price, "score_flip", score, reasons, extras, pnl_pct, hold_hrs)

            # Mean reversion target: if RSI was oversold at entry and now reverts to mean
            if rsi > 55 and pos.get("entry_rsi", 50) < 35:
                return self._sell_signal(current_time, symbol, price, "mr_target", score, reasons, extras, pnl_pct, hold_hrs)

            # Still in position, no exit signal
            return None

        # --- ENTRY LOGIC (only if no open position) ---
        threshold = p.get("signal_threshold", 0.35)
        if score > threshold:
            confidence = min(abs(score), 1.0)
            # Record the position for exit tracking
            self.open_positions[symbol] = {
                "entry_price": price,
                "entry_time": current_time,
                "score": score,
                "atr": atr,
                "entry_rsi": rsi,
                "peak_price": price,
            }
            return TradeSignal(
                timestamp=current_time,
                symbol=symbol,
                action="buy",
                confidence=confidence,
                size=1.0,
                strategy_name=self.name,
                metadata={
                    "score": round(score, 3),
                    "reasons": reasons,
                    "rsi": round(rsi, 1),
                    "atr": round(atr, 4),
                    "bb_pos": round(extras["bb_pos"], 2),
                    "vol_ratio": round(extras["vol_ratio"], 2),
                },
            )

        return None

    def _sell_signal(self, current_time, symbol, price, exit_reason, score, reasons, extras, pnl_pct, hold_hrs):
        """Generate a sell signal and clear the position"""
        if symbol in self.open_positions:
            del self.open_positions[symbol]
        # Record round trip for metrics
        self.completed_round_trips.append({
            "symbol": symbol,
            "pnl_pct": pnl_pct,
            "hold_hrs": hold_hrs,
            "exit_reason": exit_reason,
        })
        return TradeSignal(
            timestamp=current_time,
            symbol=symbol,
            action="sell",
            confidence=1.0,
            size=1.0,
            strategy_name=self.name,
            metadata={
                "exit_reason": exit_reason,
                "score": round(score, 3),
                "pnl_pct": round(pnl_pct, 2),
                "hold_hrs": round(hold_hrs, 1),
                "rsi": round(extras["rsi"], 1),
            },
        )


def compute_round_trip_metrics_list(trips: list) -> Dict:
    """Compute performance metrics from a list of round-trip dicts (post-processing)"""
    if not trips:
        return {"round_trips": 0, "win_rate": 0, "total_pnl_pct": 0, "avg_pnl_pct": 0,
                "sharpe": 0, "max_drawdown_pct": 0, "best_trade": 0, "worst_trade": 0,
                "avg_win": 0, "avg_loss": 0, "profit_factor": 0, "avg_hold_hrs": 0,
                "exit_reasons": {}}

    pnls = [r["pnl_pct"] for r in trips]
    wins = [p for p in pnls if p > 0]
    losses = [p for p in pnls if p <= 0]
    total_pnl = sum(pnls)

    cum = np.cumsum(pnls)
    running_max = np.maximum.accumulate(cum)
    drawdowns = cum - running_max
    max_dd = abs(min(drawdowns)) if len(drawdowns) > 0 else 0

    sharpe = np.mean(pnls) / np.std(pnls) * np.sqrt(24 * 365) if np.std(pnls) > 0 else 0

    avg_win = np.mean(wins) if wins else 0
    avg_loss = abs(np.mean(losses)) if losses else 0
    profit_factor = avg_win / avg_loss if avg_loss > 0 else float("inf")

    exit_reasons = {}
    for r in trips:
        reason = r.get("exit_reason", "signal")
        exit_reasons[reason] = exit_reasons.get(reason, 0) + 1

    return {
        "round_trips": len(trips),
        "win_rate": len(wins) / len(pnls),
        "total_pnl_pct": total_pnl,
        "avg_pnl_pct": np.mean(pnls),
        "avg_hold_hrs": np.mean([r["hold_hrs"] for r in trips]),
        "max_drawdown_pct": max_dd,
        "sharpe": sharpe,
        "best_trade": max(pnls),
        "worst_trade": min(pnls),
        "avg_win": avg_win,
        "avg_loss": avg_loss,
        "profit_factor": profit_factor,
        "exit_reasons": exit_reasons,
    }


async def backtest_symbol(symbol: str, params: Dict = None) -> Dict:
    """Run backtest for one symbol"""
    data = load_multi_tf(symbol)
    if "1h" not in data or len(data["1h"]) < 200:
        return {"symbol": symbol, "error": "insufficient data"}

    df = data["1h"].copy()

    strategy = MultiTFStrategy(
        f"mtf_r2_{symbol}",
        {**{"symbol": symbol}, **(params or {})},
    )

    sim = FutureBlindSimulator(initial_capital=10000)
    sim.add_strategy(strategy)

    from agents.historical_data_collector import DataWindow
    window = DataWindow(
        symbol=symbol,
        exchange="binance",
        start_time=df.index[0].to_pydatetime(),
        end_time=df.index[-1].to_pydatetime(),
        current_time=df.index[0].to_pydatetime(),
        data=df,
    )

    result = await sim.run_simulation(window, time_step_minutes=60)

    # Use the strategy's own round-trip tracking (with exit reasons)
    round_trips = strategy.completed_round_trips
    metrics = compute_round_trip_metrics_list(round_trips)

    return {
        "symbol": symbol,
        "candles": len(df),
        "start": str(df.index[0].date()),
        "end": str(df.index[-1].date()),
        "final_balance": round(result.final_balance, 2),
        **metrics,
    }


async def run_sweep():
    """Parameter sweep for round 2"""
    param_sets = [
        # R6: Validate best R5 configs on 365 days of data
        # r4_winner was best: 3/5 profitable on 90d. Does it hold?
        {"signal_threshold": 0.40, "min_alignment": 3, "take_profit_atr": 4.0, "stop_loss_atr": 2.5, "max_hold_hours": 72, "time_decay_hours": 48, "label": "r4_winner_365d"},
        # tight_sl: best portfolio without XRP on 90d
        {"signal_threshold": 0.40, "min_alignment": 3, "take_profit_atr": 4.0, "stop_loss_atr": 1.5, "max_hold_hours": 72, "time_decay_hours": 48, "label": "tight_sl_365d"},
        # wide_tp: BNB's best config
        {"signal_threshold": 0.40, "min_alignment": 3, "take_profit_atr": 6.0, "stop_loss_atr": 2.5, "max_hold_hours": 96, "time_decay_hours": 48, "label": "wide_tp_365d"},
        # R7: Trailing stop experiments — WFA-validated trailing_stop_atr=1.0
        {"signal_threshold": 0.40, "min_alignment": 3, "take_profit_atr": 4.0, "stop_loss_atr": 2.5, "max_hold_hours": 72, "time_decay_hours": 48, "trailing_stop_atr": 1.0, "label": "r4_winner_trail1_365d"},
        {"signal_threshold": 0.40, "min_alignment": 3, "take_profit_atr": 4.0, "stop_loss_atr": 1.5, "max_hold_hours": 72, "time_decay_hours": 48, "trailing_stop_atr": 1.0, "label": "tight_sl_trail1_365d"},
    ]

    all_results = {}

    print(f"\n{'='*90}")
    print(f"BACKTEST ROUND 7 — Trailing stop experiments on 365 days of data")
    print(f"5 symbols × 365 days real Binance data (1h/4h/1d)")
    print(f"{'='*90}\n")

    for ps in param_sets:
        label = ps.pop("label")
        params = {k: v for k, v in ps.items()}

        print(f"\n── {label.upper()} ──")
        print(f"  threshold={params['signal_threshold']} alignment={params['min_alignment']} "
              f"TP={params['take_profit_atr']}xATR SL={params['stop_loss_atr']}xATR "
              f"max_hold={params['max_hold_hours']}h decay={params['time_decay_hours']}h\n")

        print(f"  {'Symbol':12s} {'Rounds':>7s} {'Win%':>6s} {'AvgPnL':>8s} {'Total':>8s} {'MaxDD':>7s} {'Sharpe':>7s} {'PF':>5s} {'AvgHrs':>6s} {'Best':>7s} {'Worst':>7s}")
        print(f"  {'─'*12} {'─'*7} {'─'*6} {'─'*8} {'─'*8} {'─'*7} {'─'*7} {'─'*5} {'─'*6} {'─'*7} {'─'*7}")

        results = []
        for symbol in SYMBOLS:
            r = await backtest_symbol(symbol, params)
            if "error" not in r:
                results.append(r)
                rt = r.get("round_trips", 0)
                wr = r.get("win_rate", 0) * 100
                avg_pnl = r.get("avg_pnl_pct", 0)
                total = r.get("total_pnl_pct", 0)
                max_dd = r.get("max_drawdown_pct", 0)
                sharpe = r.get("sharpe", 0)
                pf = r.get("profit_factor", 0)
                avg_h = r.get("avg_hold_hrs", 0)
                best = r.get("best_trade", 0)
                worst = r.get("worst_trade", 0)
                pf_str = f"{pf:.1f}" if pf != float("inf") else "INF"
                print(f"  {r['symbol']:12s} {rt:7d} {wr:5.1f}% {avg_pnl:7.3f}% {total:7.3f}% {max_dd:6.2f}% {sharpe:7.2f} {pf_str:>5s} {avg_h:5.1f}h {best:6.2f}% {worst:6.2f}%")

        if results:
            avg_wr = np.mean([r.get("win_rate", 0) for r in results])
            avg_pnl = np.mean([r.get("avg_pnl_pct", 0) for r in results])
            total_pnl = sum(r.get("total_pnl_pct", 0) for r in results)
            avg_sharpe = np.mean([r.get("sharpe", 0) for r in results])
            avg_dd = np.mean([r.get("max_drawdown_pct", 0) for r in results])
            avg_pf = np.mean([r.get("profit_factor", 0) for r in results if r.get("profit_factor", 0) != float("inf")])
            avg_hold = np.mean([r.get("avg_hold_hrs", 0) for r in results])

            # Aggregate exit reasons
            all_exits = {}
            for r in results:
                for reason, count in r.get("exit_reasons", {}).items():
                    all_exits[reason] = all_exits.get(reason, 0) + count

            print(f"  {'─'*12} {'─'*7} {'─'*6} {'─'*8} {'─'*8} {'─'*7} {'─'*7} {'─'*5} {'─'*6} {'─'*7} {'─'*7}")
            print(f"  {'PORTFOLIO':12s} {sum(r.get('round_trips', 0) for r in results):7d} {avg_wr*100:5.1f}% {avg_pnl:7.3f}% {total_pnl:7.3f}% {avg_dd:6.2f}% {avg_sharpe:7.2f} {avg_pf:5.1f} {avg_hold:5.1f}h")
            print(f"  Exit reasons: {all_exits}")

        all_results[label] = results

    # Save
    out = os.path.join(DATA_DIR, "..", "backtest_r2_results.json")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w") as f:
        json.dump({"run_at": datetime.now().isoformat(), "strategies": all_results}, f, indent=2, default=str)
    print(f"\nResults saved to {out}")


if __name__ == "__main__":
    asyncio.run(run_sweep())
