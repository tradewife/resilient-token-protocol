"""
S02 — Breakout-Band Expansion.

BB squeeze -> breakout. High priority for crypto (low-vol compression
before explosive directional moves).

Entry: BB Width < 10th percentile AND close breaks above upper band.
Exit:  TP at 3x ATR, SL at 1.5x ATR, max hold 48h, BB expansion complete.
"""
from typing import Dict, List, Optional

import numpy as np
import pandas as pd

from research.strategy_plugins.base import (
    StrategyPlugin, EntrySignal, ExitReason,
)


class BreakoutBandPlugin(StrategyPlugin):
    """S02 Breakout-Band Expansion plugin."""

    @property
    def name(self) -> str:
        return "S02_Breakout_Band"

    @property
    def description(self) -> str:
        return "BB squeeze -> breakout expansion"

    def compute_indicators(self, df: pd.DataFrame) -> pd.DataFrame:
        close = df["close"]
        bb_period = 20
        sma = close.rolling(bb_period).mean()
        std = close.rolling(bb_period).std()
        df["bb_upper"] = sma + 2.0 * std
        df["bb_lower"] = sma - 2.0 * std
        df["bb_middle"] = sma
        df["bb_width"] = (df["bb_upper"] - df["bb_lower"]) / sma

        # Width percentile over lookback
        lookback = 100
        df["bb_width_pctl"] = df["bb_width"].rolling(lookback).rank(pct=True) * 100

        # ATR (same formula as fast sim)
        if "atr" not in df.columns:
            df["atr"] = close.pct_change().rolling(20).std() * close

        return df

    def check_entry(self, df: pd.DataFrame, idx: int,
                    params: Dict) -> Optional[EntrySignal]:
        close = df["close"].values[idx]
        bb_upper = df["bb_upper"].values[idx] if "bb_upper" in df.columns else np.nan
        bb_width_pctl = df["bb_width_pctl"].values[idx] if "bb_width_pctl" in df.columns else 50

        squeeze_pctl = params.get("squeeze_percentile", 10)

        if np.isnan(bb_upper) or np.isnan(bb_width_pctl):
            return None

        # Squeeze + breakout above upper band
        if bb_width_pctl <= squeeze_pctl and close > bb_upper:
            atr = df["atr"].values[idx] if "atr" in df.columns else 0
            return EntrySignal(direction=1, price=close, bar_idx=idx,
                               atr=float(atr) if not np.isnan(atr) else 0)

        return None

    def check_exit(self, df: pd.DataFrame, idx: int,
                   position: Dict, params: Dict) -> Optional[ExitReason]:
        bb_width_pctl = df["bb_width_pctl"].values[idx] if "bb_width_pctl" in df.columns else 50
        expansion_pctl = params.get("expansion_exit_percentile", 80)

        # Exit when BB expansion is complete
        if not np.isnan(bb_width_pctl) and bb_width_pctl >= expansion_pctl:
            return ExitReason.SIGNAL_EXIT

        # Exit at BB middle band (mean reversion target)
        if position.get("direction", 1) == 1:
            bb_middle = df["bb_middle"].values[idx] if "bb_middle" in df.columns else np.nan
            close = df["close"].values[idx]
            if not np.isnan(bb_middle) and close <= bb_middle:
                return ExitReason.TARGET_REACHED

        return None

    def param_grid(self) -> Dict[str, List]:
        return {
            "squeeze_percentile": [5, 10, 15, 20],
            "expansion_exit_percentile": [70, 75, 80, 85],
            "stop_loss_atr": [1.0, 1.5, 2.0, 2.5],
            "take_profit_atr": [2.0, 3.0, 4.0, 5.0],
            "max_hold_hours": [24, 36, 48, 60],
            "time_decay_hours": [18, 24, 36],
            "trailing_stop_atr": [0.0, 0.3, 0.5],
            "leverage": [3.0, 5.0, 7.0, 9.0],
        }

    def default_params(self) -> Dict:
        return {
            "squeeze_percentile": 10,
            "expansion_exit_percentile": 80,
            "stop_loss_atr": 1.5,
            "take_profit_atr": 3.0,
            "max_hold_hours": 48,
            "time_decay_hours": 24,
            "trailing_stop_atr": 0.0,
            "leverage": 1.0,
        }
