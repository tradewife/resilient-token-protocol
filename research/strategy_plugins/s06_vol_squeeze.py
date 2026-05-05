"""
S06 — Volatility Breakout (Squeeze).

ATR compression -> expansion. Different from BB squeeze (uses raw ATR
percentile, not band width). Catches the start of explosive moves
after low-volatility compression.

Entry: ATR/close < 20th percentile -> breakout above highest_high(20) + 0.5*ATR.
Exit:  TP at 4x ATR, SL at 1.5x ATR, trail after 2x ATR profit.
"""
from typing import Dict, List, Optional

import numpy as np
import pandas as pd

from research.strategy_plugins.base import (
    StrategyPlugin, EntrySignal, ExitReason,
)


class VolSqueezePlugin(StrategyPlugin):
    """S06 Volatility Breakout Squeeze plugin."""

    @property
    def name(self) -> str:
        return "S06_Vol_Squeeze"

    @property
    def description(self) -> str:
        return "ATR compression -> volatility breakout expansion"

    def compute_indicators(self, df: pd.DataFrame) -> pd.DataFrame:
        close = df["close"]
        high = df["high"]
        low = df["low"]

        # ATR (same formula as fast sim)
        if "atr" not in df.columns:
            df["atr"] = close.pct_change().rolling(20).std() * close

        # ATR as percentage of price
        df["atr_pct"] = df["atr"] / close

        # ATR percentile
        lookback = 100
        df["atr_pctl"] = df["atr_pct"].rolling(lookback).rank(pct=True) * 100

        # Breakout levels: highest high / lowest low over window
        window = 20
        df["highest_high"] = high.rolling(window).max()
        df["lowest_low"] = low.rolling(window).min()

        # Compression flag
        squeeze_pctl = 20  # default, overridden by params
        df["in_squeeze"] = df["atr_pctl"] <= squeeze_pctl

        return df

    def check_entry(self, df: pd.DataFrame, idx: int,
                    params: Dict) -> Optional[EntrySignal]:
        close = df["close"].values[idx]
        atr_pctl = df["atr_pctl"].values[idx] if "atr_pctl" in df.columns else 50
        highest_high = df["highest_high"].values[idx] if "highest_high" in df.columns else np.nan
        atr = df["atr"].values[idx] if "atr" in df.columns else 0

        squeeze_pctl = params.get("squeeze_atr_percentile", 20)
        breakout_buffer = params.get("breakout_buffer_atr", 0.5)

        if np.isnan(atr_pctl) or np.isnan(highest_high) or np.isnan(atr):
            return None

        # Check if we're in a squeeze (low vol regime)
        if atr_pctl > squeeze_pctl:
            return None

        # Breakout above highest high + buffer
        upper_level = highest_high + breakout_buffer * atr
        if close > upper_level:
            return EntrySignal(direction=1, price=close, bar_idx=idx,
                               atr=float(atr))

        return None

    def check_exit(self, df: pd.DataFrame, idx: int,
                   position: Dict, params: Dict) -> Optional[ExitReason]:
        # No plugin-specific exits — rely on generic SL/TP/trail/max_hold
        return None

    def param_grid(self) -> Dict[str, List]:
        return {
            "squeeze_atr_percentile": [10, 15, 20, 25],
            "breakout_buffer_atr": [0.3, 0.5, 0.8, 1.0],
            "stop_loss_atr": [1.0, 1.5, 2.0, 2.5],
            "take_profit_atr": [3.0, 4.0, 5.0, 6.0],
            "max_hold_hours": [36, 48, 60, 72],
            "time_decay_hours": [24, 36, 48],
            "trailing_stop_atr": [0.0, 0.5, 1.0, 1.5],
            "leverage": [3.0, 5.0, 7.0, 9.0],
        }

    def default_params(self) -> Dict:
        return {
            "squeeze_atr_percentile": 20,
            "breakout_buffer_atr": 0.5,
            "stop_loss_atr": 1.5,
            "take_profit_atr": 4.0,
            "max_hold_hours": 60,
            "time_decay_hours": 36,
            "trailing_stop_atr": 1.0,
            "leverage": 1.0,
        }
