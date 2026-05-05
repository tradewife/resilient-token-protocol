"""
S04 — Mean-Reversion RSI Exhaustion.

Deep RSI + BB extreme + trend filter. Catches exhaustion moves
in ranging markets where leverage-induced liquidation cascades
create temporary dislocations.

Entry: RSI < 25 AND close < BB lower AND daily trend is bullish.
Exit:  RSI crosses back through 45, or BB middle band reached, SL at 2x ATR.
"""
from typing import Dict, List, Optional

import numpy as np
import pandas as pd

from research.strategy_plugins.base import (
    StrategyPlugin, EntrySignal, ExitReason,
)


class RSIExhaustionPlugin(StrategyPlugin):
    """S04 Mean-Reversion RSI Exhaustion plugin."""

    @property
    def name(self) -> str:
        return "S04_RSI_Exhaustion"

    @property
    def description(self) -> str:
        return "RSI exhaustion + BB extreme + trend filter mean reversion"

    def compute_indicators(self, df: pd.DataFrame) -> pd.DataFrame:
        close = df["close"]

        # RSI
        delta = close.diff()
        gain = delta.where(delta > 0, 0).rolling(14).mean()
        loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
        rs = gain / loss
        df["rsi"] = 100 - (100 / (1 + rs))

        # Bollinger Bands
        sma = close.rolling(20).mean()
        std = close.rolling(20).std()
        df["bb_upper"] = sma + 2.0 * std
        df["bb_lower"] = sma - 2.0 * std
        df["bb_middle"] = sma

        # Trend filter: SMA 200
        df["trend_sma"] = close.rolling(200).mean()
        df["daily_trend"] = np.where(
            close > df["trend_sma"] * 0.97, "bullish",
            np.where(close < df["trend_sma"] * 1.03, "bearish", "neutral")
        )

        # ATR
        if "atr" not in df.columns:
            df["atr"] = close.pct_change().rolling(20).std() * close

        return df

    def check_entry(self, df: pd.DataFrame, idx: int,
                    params: Dict) -> Optional[EntrySignal]:
        close = df["close"].values[idx]
        rsi = df["rsi"].values[idx] if "rsi" in df.columns else 50
        bb_lower = df["bb_lower"].values[idx] if "bb_lower" in df.columns else np.nan
        trend = df["daily_trend"].values[idx] if "daily_trend" in df.columns else "neutral"

        rsi_thresh = params.get("rsi_oversold", 25)
        require_trend = params.get("require_trend_filter", True)

        if np.isnan(rsi) or np.isnan(bb_lower):
            return None

        # RSI exhaustion + BB extreme + trend filter
        rsi_extreme = rsi < rsi_thresh
        bb_extreme = close <= bb_lower
        trend_ok = (not require_trend) or (trend == "bullish")

        if rsi_extreme and bb_extreme and trend_ok:
            atr = df["atr"].values[idx] if "atr" in df.columns else 0
            return EntrySignal(direction=1, price=close, bar_idx=idx,
                               atr=float(atr) if not np.isnan(atr) else 0)

        return None

    def check_exit(self, df: pd.DataFrame, idx: int,
                   position: Dict, params: Dict) -> Optional[ExitReason]:
        rsi = df["rsi"].values[idx] if "rsi" in df.columns else 50
        close = df["close"].values[idx]
        bb_middle = df["bb_middle"].values[idx] if "bb_middle" in df.columns else np.nan
        rsi_exit = params.get("rsi_exit_threshold", 45)

        # RSI recovery exit
        if not np.isnan(rsi) and rsi >= rsi_exit:
            return ExitReason.SIGNAL_EXIT

        # BB middle band target
        if not np.isnan(bb_middle) and close >= bb_middle:
            return ExitReason.TARGET_REACHED

        return None

    def param_grid(self) -> Dict[str, List]:
        return {
            "rsi_oversold": [20, 25, 28, 30],
            "rsi_exit_threshold": [40, 45, 50, 55],
            "require_trend_filter": [True, False],
            "stop_loss_atr": [1.5, 2.0, 2.5, 3.0],
            "take_profit_atr": [2.0, 3.0, 4.0],
            "max_hold_hours": [24, 36, 48],
            "time_decay_hours": [18, 24, 36],
            "trailing_stop_atr": [0.0, 0.2, 0.4],
            "leverage": [3.0, 5.0, 7.0, 9.0],
        }

    def default_params(self) -> Dict:
        return {
            "rsi_oversold": 25,
            "rsi_exit_threshold": 45,
            "require_trend_filter": True,
            "stop_loss_atr": 2.0,
            "take_profit_atr": 3.0,
            "max_hold_hours": 36,
            "time_decay_hours": 24,
            "trailing_stop_atr": 0.0,
            "leverage": 1.0,
        }
