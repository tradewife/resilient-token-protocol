"""
S10 — Momentum Divergence.

Price makes new high but momentum doesn't -> mean-reversion signal.
Works in both regimes — catches exhaustion in trends.

Entry: Price > highest in N bars BUT momentum < highest momentum in N bars
       (bearish divergence, fade the move via short).
       For longs: price < lowest BUT momentum > lowest momentum (bullish divergence).

Exit:  RSI reversion, max hold, ATR-based stops.
"""
from typing import Dict, List, Optional

import numpy as np
import pandas as pd

from research.strategy_plugins.base import (
    StrategyPlugin, EntrySignal, ExitReason,
)


class MomentumDivergencePlugin(StrategyPlugin):
    """S10 Momentum Divergence plugin."""

    @property
    def name(self) -> str:
        return "S10_Momentum_Divergence"

    @property
    def description(self) -> str:
        return "Price/momentum divergence mean-reversion signal"

    def compute_indicators(self, df: pd.DataFrame) -> pd.DataFrame:
        close = df["close"]

        # Momentum (ROC)
        mom_period = 20
        df["roc"] = close.pct_change(mom_period) * 100

        # RSI
        delta = close.diff()
        gain = delta.where(delta > 0, 0).rolling(14).mean()
        loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
        rs = gain / loss
        df["rsi"] = 100 - (100 / (1 + rs))

        # Rolling highest close / lowest close
        lookback = 40
        df["highest_close"] = close.rolling(lookback).max()
        df["lowest_close"] = close.rolling(lookback).min()

        # Rolling highest/lowest momentum
        df["highest_roc"] = df["roc"].rolling(lookback).max()
        df["lowest_roc"] = df["roc"].rolling(lookback).min()

        # ATR
        if "atr" not in df.columns:
            df["atr"] = close.pct_change().rolling(20).std() * close

        return df

    def check_entry(self, df: pd.DataFrame, idx: int,
                    params: Dict) -> Optional[EntrySignal]:
        close = df["close"].values[idx]
        roc = df["roc"].values[idx] if "roc" in df.columns else 0
        highest_close = df["highest_close"].values[idx] if "highest_close" in df.columns else np.nan
        lowest_close = df["lowest_close"].values[idx] if "lowest_close" in df.columns else np.nan
        highest_roc = df["highest_roc"].values[idx] if "highest_roc" in df.columns else np.nan
        lowest_roc = df["lowest_roc"].values[idx] if "lowest_roc" in df.columns else np.nan
        rsi = df["rsi"].values[idx] if "rsi" in df.columns else 50

        roc_thresh = params.get("divergence_roc_threshold", 0.5)
        rsi_overbought = params.get("rsi_overbought", 70)
        rsi_oversold = params.get("rsi_oversold", 30)

        if np.isnan(highest_close) or np.isnan(highest_roc):
            return None

        # Bearish divergence: price at/near high but momentum not confirming
        price_at_high = close >= highest_close * (1 - 0.005)
        mom_not_confirming = roc < highest_roc - roc_thresh

        if price_at_high and mom_not_confirming and rsi > rsi_overbought:
            atr = df["atr"].values[idx] if "atr" in df.columns else 0
            return EntrySignal(direction=-1, price=close, bar_idx=idx,
                               atr=float(atr) if not np.isnan(atr) else 0)

        # Bullish divergence: price at/near low but momentum not confirming
        price_at_low = close <= lowest_close * (1 + 0.005)
        mom_bullish = roc > lowest_roc + roc_thresh

        if price_at_low and mom_bullish and rsi < rsi_oversold:
            atr = df["atr"].values[idx] if "atr" in df.columns else 0
            return EntrySignal(direction=1, price=close, bar_idx=idx,
                               atr=float(atr) if not np.isnan(atr) else 0)

        return None

    def check_exit(self, df: pd.DataFrame, idx: int,
                   position: Dict, params: Dict) -> Optional[ExitReason]:
        rsi = df["rsi"].values[idx] if "rsi" in df.columns else 50
        direction = position.get("direction", 1)

        # RSI reversion exit
        if direction == -1 and rsi < 50:
            return ExitReason.SIGNAL_EXIT
        if direction == 1 and rsi > 50:
            return ExitReason.SIGNAL_EXIT

        return None

    def param_grid(self) -> Dict[str, List]:
        return {
            "divergence_roc_threshold": [0.3, 0.5, 0.8, 1.0],
            "rsi_overbought": [65, 70, 75, 80],
            "rsi_oversold": [20, 25, 30, 35],
            "stop_loss_atr": [1.5, 2.0, 2.5, 3.0],
            "take_profit_atr": [2.0, 3.0, 4.0],
            "max_hold_hours": [24, 36, 48, 60],
            "time_decay_hours": [18, 24, 36],
            "trailing_stop_atr": [0.0, 0.3, 0.5],
            "leverage": [3.0, 5.0, 7.0, 9.0],
        }

    def default_params(self) -> Dict:
        return {
            "divergence_roc_threshold": 0.5,
            "rsi_overbought": 70,
            "rsi_oversold": 30,
            "stop_loss_atr": 2.0,
            "take_profit_atr": 3.0,
            "max_hold_hours": 48,
            "time_decay_hours": 24,
            "trailing_stop_atr": 0.0,
            "leverage": 1.0,
        }
