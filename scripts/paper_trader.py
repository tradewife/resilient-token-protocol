"""
Live Paper Trading Engine.

Connects to Binance via ccxt, runs the MultiTFStrategy on real-time 1h candles,
tracks hypothetical positions and P&L without executing real trades.

Features:
- Streams real-time OHLCV from Binance
- Uses the production wide_tp config from knowledge base
- ADX regime filter (once validated)
- Tracks all signals, positions, and round trips
- Logs to JSON for later analysis
- Prints live dashboard every hour
- Drops XRP (confirmed net-negative in WFA)
"""
import asyncio
import os
import sys
import json
import time
import signal
import numpy as np
import pandas as pd
from datetime import datetime, timedelta, timezone
from typing import Dict, List, Optional
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

import ccxt.async_support as ccxt

# Production config — validated on 365d data + 47-fold WFA + ADX regime filter
PRODUCTION_PARAMS = {
    "signal_threshold": 0.40,
    "min_alignment": 3,
    "take_profit_atr": 6.0,
    "stop_loss_atr": 2.5,
    "max_hold_hours": 96,
    "time_decay_hours": 48,
    "score_flip_delay_hrs": 2,
    "trailing_stop_atr": 1.0,
}

# Per-symbol overrides — night shift validated configs
# Updated: 2026-04-06 from night shift run (9-fold WFA, full pipeline)
# SOL: OOS Sharpe +18.68, 100% consistency, 0 overfitting, 28 trades/fold
# BTC/ETH/BNB: production baseline underperforming — kept as learning probes
#   to identify what the optimizer can't improve (helps build self-correction)
SYMBOL_PARAMS = {
    "BTC/USDT": PRODUCTION_PARAMS,  # TODO: night shift can't find edge — needs regime-aware config
    "ETH/USDT": PRODUCTION_PARAMS,  # TODO: marginal survivor 3.04, overfitting flagged
    "SOL/USDT": {
        "signal_threshold": 0.35,
        "min_alignment": 3,
        "take_profit_atr": 4.0,
        "stop_loss_atr": 1.25,
        "max_hold_hours": 36,
        "time_decay_hours": 41,
        "score_flip_delay_hrs": 1,
        "trailing_stop_atr": 0.7036,
        "label": "night_shift_2026-04-05",
    },
    "BNB/USDT": PRODUCTION_PARAMS,  # TODO: highly correlated to BTC (0.90), same problem
}

# Drop XRP — confirmed net-negative across all WFA folds (-10.4% total)
TRADE_SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT"]

# ADX regime filter — validated: reduces std dev 29%, improves total PnL +31.7%
ADX_THRESHOLD = 25.0

LOG_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "paper_trading")
STATE_FILE = os.path.join(LOG_DIR, "state.json")


def compute_adx(high, low, close, period=14):
    """Compute ADX using Wilder's smoothing."""
    up = high.diff()
    down = -low.diff()
    plus_dm = up.where((up > down) & (up > 0), 0.0)
    minus_dm = down.where((down > up) & (down > 0), 0.0)
    tr = pd.concat([high - low, (high - close.shift(1)).abs(),
                     (low - close.shift(1)).abs()], axis=1).max(axis=1)
    alpha = 1.0 / period
    atr = tr.ewm(alpha=alpha, min_periods=period).mean()
    plus_di = 100 * plus_dm.ewm(alpha=alpha, min_periods=period).mean() / atr
    minus_di = 100 * minus_dm.ewm(alpha=alpha, min_periods=period).mean() / atr
    dx = 100 * (plus_di - minus_di).abs() / (plus_di + minus_di)
    return dx.ewm(alpha=alpha, min_periods=period).mean()


class PaperTrader:
    """Live paper trading engine using the MultiTFStrategy logic."""

    def __init__(self):
        self.exchange = None
        self.ohlcv_cache = {}  # symbol -> DataFrame of recent candles
        self.positions = {}    # symbol -> {entry_price, entry_time, entry_score, atr, entry_rsi}
        self.round_trips = []  # completed trades
        self.signals = []      # all signals (including filtered)
        self.equity_curve = [] # (timestamp, total_value)
        self.start_time = None
        self.initial_capital = 10000.0
        self.balance = self.initial_capital
        self.fee_rate = 0.001  # 0.1% Binance maker fee
        self.slippage_bps = 10
        self.running = True
        self.last_dashboard = datetime.now(timezone.utc)

    async def initialize(self):
        """Set up exchange connection and load historical data."""
        os.makedirs(LOG_DIR, exist_ok=True)

        self.exchange = ccxt.binance({"enableRateLimit": True})
        print("Connected to Binance")

        # Load 300h of historical data for indicator warmup
        print("Loading historical OHLCV for warmup...")
        for symbol in TRADE_SYMBOLS:
            since = int((datetime.now(timezone.utc) - timedelta(hours=300)).timestamp() * 1000)
            all_candles = []
            while len(all_candles) < 350:
                batch = await self.exchange.fetch_ohlcv(symbol, "1h", since=since, limit=500)
                if not batch:
                    break
                all_candles.extend(batch)
                since = batch[-1][0] + 1
                await asyncio.sleep(self.exchange.rateLimit / 1000)

            df = pd.DataFrame(all_candles, columns=["timestamp", "open", "high", "low", "close", "volume"])
            df["timestamp"] = pd.to_datetime(df["timestamp"], unit="ms", utc=True)
            df = df.drop_duplicates(subset="timestamp").set_index("timestamp").sort_index()
            self.ohlcv_cache[symbol] = df
            print(f"  {symbol}: {len(df)} candles loaded")

        # Load any existing state (for restarts)
        self._load_state()
        self.start_time = self.start_time or datetime.now(timezone.utc)
        print(f"\nPaper trader initialized. Balance: ${self.balance:,.2f}")
        print(f"Open positions: {len(self.positions)}")
        print(f"Completed trades: {len(self.round_trips)}")

    def _load_state(self):
        """Load previous state from disk for restart resilience."""
        if os.path.exists(STATE_FILE):
            try:
                with open(STATE_FILE) as f:
                    state = json.load(f)
                self.balance = state.get("balance", self.initial_capital)
                self.round_trips = state.get("round_trips", [])
                self.signals = state.get("signals", [])
                # Reconstruct positions (we lose some detail on restart)
                for sym, pos in state.get("positions", {}).items():
                    pos["entry_time"] = datetime.fromisoformat(pos["entry_time"])
                    self.positions[sym] = pos
                if state.get("start_time"):
                    self.start_time = datetime.fromisoformat(state["start_time"])
                print(f"Loaded state: {len(self.round_trips)} trades, {len(self.positions)} open")
            except Exception as e:
                print(f"State load failed: {e}, starting fresh")

    def _save_state(self):
        """Persist state to disk."""
        state = {
            "balance": self.balance,
            "start_time": self.start_time.isoformat() if self.start_time else None,
            "positions": {
                sym: {**pos, "entry_time": pos["entry_time"].isoformat()}
                for sym, pos in self.positions.items()
            },
            "round_trips": self.round_trips[-500:],  # keep last 500
            "signals": self.signals[-200:],
            "updated_at": datetime.now(timezone.utc).isoformat(),
        }
        with open(STATE_FILE, "w") as f:
            json.dump(state, f, indent=2, default=str)

    def _compute_score(self, df: pd.DataFrame, symbol: str) -> tuple:
        """Compute multi-TF confluence score (same logic as backtest strategy)."""
        from scripts.run_backtest_r2 import timeframe_signal

        close = df["close"]
        if len(close) < 200:
            return 0, [], {}

        vol_series = df["volume"]
        tf_1h = timeframe_signal(close, 20)
        tf_4h = timeframe_signal(close, 80)
        tf_1d = timeframe_signal(close, 200)

        trends = [tf_1h["trend"], tf_4h["trend"], tf_1d["trend"]]
        bullish = sum(1 for t in trends if t == "bullish")
        bearish = sum(1 for t in trends if t == "bearish")
        rsi = tf_1h["rsi"]

        mr_signal, mr_reason = 0.0, ""
        if rsi < 30:
            mr_signal, mr_reason = 0.3, "rsi_oversold"
        elif rsi < 35 and tf_1d["trend"] == "bullish":
            mr_signal, mr_reason = 0.2, "rsi_near_oversold"
        elif rsi > 70:
            mr_signal, mr_reason = -0.3, "rsi_overbought"

        sma20 = close.rolling(20).mean()
        std20 = close.rolling(20).std()
        bb_pos = ((close.iloc[-1] - (sma20.iloc[-1] - 2*std20.iloc[-1])) /
                  (4*std20.iloc[-1])) if std20.iloc[-1] > 0 else 0.5

        vol_ma = vol_series.rolling(20).mean().iloc[-1]
        vol_ratio = vol_series.iloc[-1] / vol_ma if vol_ma > 0 else 1

        score, reasons = 0.0, []
        if bullish >= 3:
            score += (bullish / 3.0) * 0.4
            reasons.append(f"tf_bull_{bullish}")
            if vol_ratio > 1.3:
                score += 0.1
                reasons.append("vol_confirm")
        elif bearish >= 3:
            score -= (bearish / 3.0) * 0.4
            reasons.append(f"tf_bear_{bearish}")

        if abs(mr_signal) > 0.1:
            score += mr_signal * 0.3
            reasons.append(mr_reason)

        mom = tf_4h["momentum"]
        if mom > 0.003:
            score += 0.15; reasons.append("mom_up")
        elif mom < -0.003:
            score -= 0.15; reasons.append("mom_down")

        if bb_pos < 0.15:
            score += 0.15; reasons.append("bb_lower")
        elif bb_pos > 0.85:
            score -= 0.15; reasons.append("bb_upper")

        atr = tf_1h["volatility"] * close.iloc[-1] if tf_1h["volatility"] > 0 else close.iloc[-1] * 0.02

        extras = {"rsi": rsi, "atr": atr, "bb_pos": bb_pos, "vol_ratio": vol_ratio,
                  "bullish_count": bullish, "bearish_count": bearish}
        return score, reasons, extras

    def _check_exits(self, symbol: str, price: float, score: float,
                      rsi: float, atr: float, now: datetime) -> Optional[dict]:
        """Check if open position should be exited. Returns round_trip dict or None."""
        if symbol not in self.positions:
            return None

        pos = self.positions[symbol]
        hold_hrs = (now - pos["entry_time"]).total_seconds() / 3600
        pnl_pct = (price - pos["entry_price"]) / pos["entry_price"] * 100

        exit_reason = None

        # Trailing stop: ratchet from peak, exit if price drops N ATR from peak
        trail_atr = SYMBOL_PARAMS[symbol].get("trailing_stop_atr", 0)
        if trail_atr > 0 and atr > 0:
            peak = pos.get("peak_price", pos["entry_price"])
            if price > peak:
                pos["peak_price"] = price
                peak = price
            trail_trigger = trail_atr * atr / pos["entry_price"] * 100
            pullback_pct = (peak - price) / pos["entry_price"] * 100
            if pullback_pct >= trail_trigger and peak > pos["entry_price"]:
                trail_pnl = (peak - pos["entry_price"]) / pos["entry_price"] * 100 - trail_trigger
                return {
                    "symbol": symbol, "pnl_pct": trail_pnl, "hold_hrs": hold_hrs,
                    "exit_reason": "trailing_stop", "entry_price": pos["entry_price"],
                    "exit_price": price, "entry_time": pos["entry_time"].isoformat(),
                    "exit_time": now.isoformat(), "score": round(score, 3),
                }

        # Stop loss
        if atr > 0 and pnl_pct <= -(SYMBOL_PARAMS[symbol]["stop_loss_atr"] * atr / pos["entry_price"] * 100):
            exit_reason = "stop_loss"
        # Take profit
        elif atr > 0 and pnl_pct >= (SYMBOL_PARAMS[symbol]["take_profit_atr"] * atr / pos["entry_price"] * 100):
            exit_reason = "take_profit"
        # Max hold
        elif hold_hrs >= SYMBOL_PARAMS[symbol]["max_hold_hours"]:
            exit_reason = "max_hold"
        # Time decay
        elif pnl_pct < 0 and hold_hrs >= SYMBOL_PARAMS[symbol]["time_decay_hours"]:
            exit_reason = "time_decay"
        # Score flip (with delay — let the trade breathe)
        elif score < 0 and hold_hrs >= SYMBOL_PARAMS[symbol].get("score_flip_delay_hrs", 0):
            exit_reason = "score_flip"
        # MR target
        elif rsi > 55 and pos.get("entry_rsi", 50) < 35:
            exit_reason = "mr_target"

        if exit_reason:
            return {
                "symbol": symbol, "pnl_pct": pnl_pct, "hold_hrs": hold_hrs,
                "exit_reason": exit_reason, "entry_price": pos["entry_price"],
                "exit_price": price, "entry_time": pos["entry_time"].isoformat(),
                "exit_time": now.isoformat(), "score": round(score, 3),
            }
        return None

    async def process_candle(self, symbol: str, candle: list):
        """Process a new candle for a symbol."""
        ts, o, h, l, c, v = candle
        now = pd.Timestamp(ts, unit="ms", tz="UTC")

        # Update OHLCV cache
        df = self.ohlcv_cache[symbol]
        new_row = pd.DataFrame({"open": [o], "high": [h], "low": [l],
                                 "close": [c], "volume": [v]}, index=[now])
        df = pd.concat([df, new_row])
        df = df[~df.index.duplicated(keep="last")]
        self.ohlcv_cache[symbol] = df

        if len(df) < 250:
            return

        score, reasons, extras = self._compute_score(df, symbol)
        price = c
        rsi = extras.get("rsi", 50)
        atr = extras.get("atr", 0)

        # ADX regime filter (only for new entries, not exits)
        adx_series = compute_adx(df["high"], df["low"], df["close"])
        current_adx = adx_series.iloc[-1] if len(adx_series) > 0 and pd.notna(adx_series.iloc[-1]) else 0

        # --- Check exits first ---
        exit = self._check_exits(symbol, price, score, rsi, atr, now)
        if exit:
            # Simulate sell
            sell_price = price * (1 - self.slippage_bps / 10000)
            fee = sell_price * 0.02 * self.fee_rate  # assuming 0.02 BTC per trade
            self.balance += sell_price * 0.02 - fee  # approximate
            del self.positions[symbol]
            self.round_trips.append(exit)
            self.signals.append({
                "time": now.isoformat(), "symbol": symbol, "action": "SELL",
                "price": sell_price, "reason": exit["exit_reason"],
                "pnl_pct": round(exit["pnl_pct"], 2), "score": round(score, 3),
                "adx": round(current_adx, 1),
            })
            self._print_signal("SELL", symbol, sell_price, exit["exit_reason"], exit["pnl_pct"])
            self._save_state()
            return

        # --- Check entries ---
        if symbol not in self.positions:
            if score > SYMBOL_PARAMS[symbol]["signal_threshold"]:
                if current_adx < ADX_THRESHOLD:
                    self.signals.append({
                        "time": now.isoformat(), "symbol": symbol, "action": "FILTERED",
                        "price": price, "score": round(score, 3), "adx": round(current_adx, 1),
                        "reason": f"adx_{current_adx:.0f}_below_{ADX_THRESHOLD}",
                    })
                    return

                # Simulate buy
                buy_price = price * (1 + self.slippage_bps / 10000)
                fee = buy_price * 0.02 * self.fee_rate
                self.balance -= buy_price * 0.02 + fee

                self.positions[symbol] = {
                    "entry_price": buy_price,
                    "entry_time": now,
                    "entry_score": score,
                    "atr": atr,
                    "entry_rsi": rsi,
                    "peak_price": buy_price,
                }
                self.signals.append({
                    "time": now.isoformat(), "symbol": symbol, "action": "BUY",
                    "price": buy_price, "score": round(score, 3), "adx": round(current_adx, 1),
                    "reasons": reasons,
                })
                self._print_signal("BUY", symbol, buy_price, f"score={score:.2f}", None)
                self._save_state()

    def _print_signal(self, action, symbol, price, reason, pnl):
        now = datetime.now(timezone.utc).strftime("%H:%M:%S")
        pnl_str = f" PnL: {pnl:+.2f}%" if pnl is not None else ""
        print(f"  [{now}] {action:4s} {symbol:12s} @ ${price:,.2f}  {reason}{pnl_str}")

    def _print_dashboard(self):
        """Print current status."""
        now = datetime.now(timezone.utc)
        elapsed = (now - self.start_time).total_seconds() / 3600 if self.start_time else 0

        # Unrealized PnL
        unrealized = 0
        for sym, pos in self.positions.items():
            df = self.ohlcv_cache.get(sym)
            if df is not None and len(df) > 0:
                current_price = df["close"].iloc[-1]
                pnl = (current_price - pos["entry_price"]) / pos["entry_price"] * 100
                unrealized += pnl

        # Stats from completed trades
        if self.round_trips:
            pnls = [t["pnl_pct"] for t in self.round_trips]
            wins = [p for p in pnls if p > 0]
            total_realized = sum(pnls)
            wr = len(wins) / len(pnls) * 100
        else:
            total_realized, wr = 0, 0

        # ADX for each symbol
        adx_strs = []
        for sym in TRADE_SYMBOLS:
            df = self.ohlcv_cache.get(sym)
            if df is not None and len(df) > 20:
                adx = compute_adx(df["high"], df["low"], df["close"])
                if len(adx) > 0 and pd.notna(adx.iloc[-1]):
                    val = adx.iloc[-1]
                    regime = "TREND" if val > ADX_THRESHOLD else "RANGE"
                    adx_strs.append(f"{sym[:3]}:{val:.0f}({regime})")

        pos_str = ", ".join(f"{sym[:3]}:{(df['close'].iloc[-1] - pos['entry_price'])/pos['entry_price']*100:+.1f}%"
                          for sym, pos in self.positions.items()
                          for df in [self.ohlcv_cache.get(sym)]
                          if df is not None and len(df) > 0)

        print(f"\n  ┌──────────── PAPER TRADER DASHBOARD ────────────┐")
        print(f"  │ Uptime: {elapsed:.0f}h  │  Trades: {len(self.round_trips)}  │  WR: {wr:.0f}%  │  Realized: {total_realized:+.1f}%  │")
        print(f"  │ Open: {len(self.positions)}  │  {pos_str or 'No positions'}")
        print(f"  │ ADX:  {'  '.join(adx_strs)}")
        print(f"  └─────────────────────────────────────────────────┘\n")

    async def run(self):
        """Main loop — poll for new candles every minute."""
        await self.initialize()
        print(f"\n{'='*60}")
        print(f"PAPER TRADER STARTED")
        print(f"Symbols: {', '.join(TRADE_SYMBOLS)}")
        print(f"ADX filter: > {ADX_THRESHOLD} (trending only)")
        custom = {s: p.get("label", "production") for s, p in SYMBOL_PARAMS.items() if p != PRODUCTION_PARAMS}
        print(f"Custom configs: {custom or 'none'}")
        for sym, p in SYMBOL_PARAMS.items():
            label = p.get("label", "production")
            tp = p.get("take_profit_atr", "?")
            sl = p.get("stop_loss_atr", "?")
            trail = p.get("trailing_stop_atr", "?")
            thresh = p.get("signal_threshold", "?")
            print(f"  {sym:12s} [{label:25s}] thresh={thresh} TP={tp} SL={sl} trail={trail}")
        print(f"{'='*60}\n")

        last_candles = {sym: None for sym in TRADE_SYMBOLS}

        while self.running:
            try:
                for symbol in TRADE_SYMBOLS:
                    candles = await self.exchange.fetch_ohlcv(symbol, "1h", limit=2)
                    if candles and len(candles) >= 1:
                        latest = candles[-1]
                        latest_ts = latest[0]

                        if last_candles[symbol] != latest_ts:
                            last_candles[symbol] = latest_ts
                            await self.process_candle(symbol, latest)

                # Dashboard every 30 minutes
                now = datetime.now(timezone.utc)
                if (now - self.last_dashboard).total_seconds() > 1800:
                    self._print_dashboard()
                    self.last_dashboard = now

                await asyncio.sleep(60)  # poll every minute

            except KeyboardInterrupt:
                print("\nShutting down...")
                break
            except ccxt.NetworkError as e:
                print(f"Network error: {e} — retrying in 60s")
                await asyncio.sleep(60)
            except ccxt.RateLimitExceeded as e:
                print(f"Rate limited: {e} — backing off 120s")
                await asyncio.sleep(120)
            except ccxt.ExchangeError as e:
                print(f"Exchange error: {e} — retrying in 60s")
                await asyncio.sleep(60)
            except Exception as e:
                print(f"Error: {e}")
                await asyncio.sleep(30)

        # Final state save and summary
        self._save_state()
        self._print_dashboard()
        print(f"\nSession complete. {len(self.round_trips)} round trips completed.")
        await self.exchange.close()


async def main():
    trader = PaperTrader()

    # Graceful shutdown
    def shutdown(sig, frame):
        print(f"\nSignal {sig} received, shutting down...")
        trader.running = False

    signal.signal(signal.SIGINT, shutdown)
    signal.signal(signal.SIGTERM, shutdown)

    await trader.run()


if __name__ == "__main__":
    asyncio.run(main())
