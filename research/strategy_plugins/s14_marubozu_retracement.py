"""
S14 — Marubozu Retracement Entry.

Identifies bullish (uptrend) or bearish (downtrend) candles with little or
no wick — a "marubozu" pattern — which signal strong directional conviction.
After the trigger candle closes, the strategy waits for price to retrace back
into the candle body, then enters in the original trend direction.

Logic:
  Uptrend:  EMA_fast > EMA_slow
            Trigger = bullish candle where lower_wick / body <= wick_tol
                      AND body >= body_atr_mult * ATR
            Entry   = price retraces to (high - retracement_pct * body)
            SL      = below trigger candle low
            TP      = ATR-based (stop_loss_atr / take_profit_atr)

  Downtrend: EMA_fast < EMA_slow (inverted logic)

Optimisation parameters are chosen for walk-forward grid search via the
RTP research engine on SOL/USDT 1-minute OHLCV data.
"""
from typing import Dict, List, Optional

import numpy as np
import pandas as pd

from research.strategy_plugins.base import (
    EntrySignal,
    ExitReason,
    StrategyPlugin,
)


class MarubozuRetracementPlugin(StrategyPlugin):
    """S14 Marubozu Retracement Entry plugin."""

    @property
    def name(self) -> str:
        return "S14_Marubozu_Retracement"

    @property
    def description(self) -> str:
        return (
            "Enter on retracement into a no-wick trend-continuation candle. "
            "Detects bullish/bearish marubozu in trend context, waits for "
            "pullback into body, enters with ATR-based SL/TP."
        )

    def compute_indicators(self, df: pd.DataFrame) -> pd.DataFrame:
        close = df["close"]
        high = df["high"]
        low = df["low"]
        open_ = df["open"]

        for period in [9, 20, 50]:
            col = f"ema_{period}"
            if col not in df.columns:
                df[col] = close.ewm(span=period, adjust=False).mean()

        if "atr" not in df.columns:
            tr = pd.concat([
                high - low,
                (high - close.shift(1)).abs(),
                (low - close.shift(1)).abs(),
            ], axis=1).max(axis=1)
            df["atr"] = tr.ewm(span=14, adjust=False).mean()

        body = (close - open_).abs()
        full_range = high - low
        bull_lower_wick = open_ - low
        df["maru_bull_lower_wick_frac"] = np.where(body > 0, bull_lower_wick / body, np.nan)
        df["maru_bull_upper_wick_frac"] = np.where(body > 0, (high - close) / body, np.nan)
        bear_upper_wick = high - open_
        df["maru_bear_upper_wick_frac"] = np.where(body > 0, bear_upper_wick / body, np.nan)
        df["maru_bear_lower_wick_frac"] = np.where(body > 0, (close - low) / body, np.nan)
        df["maru_body"] = body
        df["maru_full_range"] = full_range

        if "volume" in df.columns:
            df["vol_ma20"] = df["volume"].rolling(20).mean()

        return df

    def _is_uptrend(self, df: pd.DataFrame, idx: int, params: Dict) -> bool:
        fast = df[f"ema_{params.get('trend_fast_period', 9)}"].values[idx]
        slow = df[f"ema_{params.get('trend_slow_period', 20)}"].values[idx]
        if np.isnan(fast) or np.isnan(slow):
            return False
        return float(fast) > float(slow)

    def _is_downtrend(self, df: pd.DataFrame, idx: int, params: Dict) -> bool:
        fast = df[f"ema_{params.get('trend_fast_period', 9)}"].values[idx]
        slow = df[f"ema_{params.get('trend_slow_period', 20)}"].values[idx]
        if np.isnan(fast) or np.isnan(slow):
            return False
        return float(fast) < float(slow)

    def _is_bullish_marubozu(self, df: pd.DataFrame, idx: int, params: Dict) -> bool:
        close = df["close"].values[idx]
        open_ = df["open"].values[idx]
        if close <= open_:
            return False
        body = df["maru_body"].values[idx]
        atr = df["atr"].values[idx]
        body_mult = params.get("body_atr_multiplier", 1.5)
        if np.isnan(atr) or body < body_mult * atr:
            return False
        wick_tol = params.get("wick_tolerance_pct", 0.10)
        lower = df["maru_bull_lower_wick_frac"].values[idx]
        upper = df["maru_bull_upper_wick_frac"].values[idx]
        if np.isnan(lower) or np.isnan(upper):
            return False
        if "vol_ma20" in df.columns:
            vol = df["volume"].values[idx] if "volume" in df.columns else np.nan
            vol_ma = df["vol_ma20"].values[idx]
            vol_mult = params.get("volume_multiplier", 0.0)
            if vol_mult > 0 and not np.isnan(vol) and not np.isnan(vol_ma):
                if vol < vol_mult * vol_ma:
                    return False
        return float(lower) <= wick_tol and float(upper) <= wick_tol

    def _is_bearish_marubozu(self, df: pd.DataFrame, idx: int, params: Dict) -> bool:
        close = df["close"].values[idx]
        open_ = df["open"].values[idx]
        if open_ <= close:
            return False
        body = df["maru_body"].values[idx]
        atr = df["atr"].values[idx]
        body_mult = params.get("body_atr_multiplier", 1.5)
        if np.isnan(atr) or body < body_mult * atr:
            return False
        wick_tol = params.get("wick_tolerance_pct", 0.10)
        upper = df["maru_bear_upper_wick_frac"].values[idx]
        lower = df["maru_bear_lower_wick_frac"].values[idx]
        if np.isnan(upper) or np.isnan(lower):
            return False
        if "vol_ma20" in df.columns:
            vol = df["volume"].values[idx] if "volume" in df.columns else np.nan
            vol_ma = df["vol_ma20"].values[idx]
            vol_mult = params.get("volume_multiplier", 0.0)
            if vol_mult > 0 and not np.isnan(vol) and not np.isnan(vol_ma):
                if vol < vol_mult * vol_ma:
                    return False
        return float(upper) <= wick_tol and float(lower) <= wick_tol

    def __init__(self):
        self._pending: Optional[Dict] = None

    def check_entry(self, df: pd.DataFrame, idx: int, params: Dict) -> Optional[EntrySignal]:
        direction_filter = params.get("direction_filter", "both")
        retracement_pct = params.get("retracement_pct", 0.50)
        expiry_bars = int(params.get("expiry_bars", 10))

        if self._pending is not None:
            p = self._pending
            age = idx - p["trigger_bar"]
            if age > expiry_bars:
                self._pending = None
            else:
                current_close = float(df["close"].values[idx])
                atr = float(df["atr"].values[idx]) if not np.isnan(df["atr"].values[idx]) else p["atr"]
                if p["direction"] == 1:
                    if current_close <= p["entry_zone"] and current_close > p["sl_price"]:
                        self._pending = None
                        return EntrySignal(direction=1, price=current_close, bar_idx=idx, atr=atr)
                else:
                    if current_close >= p["entry_zone"] and current_close < p["sl_price"]:
                        self._pending = None
                        return EntrySignal(direction=-1, price=current_close, bar_idx=idx, atr=atr)
                return None

        atr_val = df["atr"].values[idx]
        if np.isnan(atr_val):
            return None

        if direction_filter in ("both", "long") and self._is_uptrend(df, idx, params):
            if self._is_bullish_marubozu(df, idx, params):
                candle_high = float(df["high"].values[idx])
                candle_low = float(df["low"].values[idx])
                candle_open = float(df["open"].values[idx])
                body = candle_high - candle_open
                entry_zone = candle_high - retracement_pct * body
                self._pending = {"direction": 1, "trigger_bar": idx, "entry_zone": entry_zone, "sl_price": candle_low, "atr": float(atr_val)}
                return None

        if direction_filter in ("both", "short") and self._is_downtrend(df, idx, params):
            if self._is_bearish_marubozu(df, idx, params):
                candle_high = float(df["high"].values[idx])
                candle_low = float(df["low"].values[idx])
                candle_open = float(df["open"].values[idx])
                body = candle_open - candle_low
                entry_zone = candle_low + retracement_pct * body
                self._pending = {"direction": -1, "trigger_bar": idx, "entry_zone": entry_zone, "sl_price": candle_high, "atr": float(atr_val)}
                return None

        return None

    def check_exit(self, df: pd.DataFrame, idx: int, position: Dict, params: Dict) -> Optional[ExitReason]:
        direction = position.get("direction", 1)
        if direction == 1 and self._is_downtrend(df, idx, params):
            return ExitReason.SIGNAL_EXIT
        if direction == -1 and self._is_uptrend(df, idx, params):
            return ExitReason.SIGNAL_EXIT
        return None

    def param_grid(self) -> Dict[str, List]:
        return {
            "wick_tolerance_pct":  [0.05, 0.10, 0.15, 0.20],
            "body_atr_multiplier": [1.0, 1.5, 2.0, 2.5],
            "retracement_pct":     [0.25, 0.38, 0.50, 0.62, 0.75],
            "trend_fast_period":   [9, 20],
            "trend_slow_period":   [20, 50],
            "expiry_bars":         [5, 10, 15, 20],
            "volume_multiplier":   [0.0, 1.0, 1.5],
            "direction_filter":    ["both", "long", "short"],
            "stop_loss_atr":       [1.0, 1.5, 2.0, 2.5],
            "take_profit_atr":     [2.0, 3.0, 4.0, 5.0],
            "max_hold_hours":      [1, 2, 4, 8],
            "time_decay_hours":    [1, 2, 4],
            "trailing_stop_atr":   [0.0, 0.5, 1.0],
            "leverage":            [1.0, 3.0, 5.0],
        }

    def default_params(self) -> Dict:
        return {
            "wick_tolerance_pct":  0.10,
            "body_atr_multiplier": 1.5,
            "retracement_pct":     0.50,
            "trend_fast_period":   9,
            "trend_slow_period":   20,
            "expiry_bars":         10,
            "volume_multiplier":   0.0,
            "direction_filter":    "both",
            "stop_loss_atr":       1.5,
            "take_profit_atr":     3.0,
            "max_hold_hours":      4,
            "time_decay_hours":    2,
            "trailing_stop_atr":   0.0,
            "leverage":            1.0,
        }
