"""
S13 — Trend-Following with ADX Filter.

Pure trend with ADX gate. Simpler than MultiTF but more focused.
Uses ADX to confirm a trend is present before entering.

Entry: close > SMA(20) AND ADX > threshold AND close > close[20] (momentum).
Exit:  Trailing stop, signal reversal, max hold.
"""
from typing import Dict, List, Optional

import numpy as np
import pandas as pd

from research.strategy_plugins.base import (
    StrategyPlugin, EntrySignal, ExitReason,
)


class ADXTrendPlugin(StrategyPlugin):
    """S13 Trend-Following with ADX Filter plugin."""

    @property
    def name(self) -> str:
        return "S13_ADX_Trend"

    @property
    def description(self) -> str:
        return "Pure trend-following with ADX confirmation gate"

    def compute_indicators(self, df: pd.DataFrame) -> pd.DataFrame:
        close = df["close"]
        high = df["high"]
        low = df["low"]

        # SMA
        sma_period = 20
        df["sma_fast"] = close.rolling(sma_period).mean()

        # ADX (Wilder's smoothing)
        period = 14
        up = high.diff()
        down = -low.diff()
        plus_dm = up.where((up > down) & (up > 0), 0.0)
        minus_dm = down.where((down > up) & (down > 0), 0.0)

        tr = pd.concat([
            high - low,
            (high - close.shift(1)).abs(),
            (low - close.shift(1)).abs(),
        ], axis=1).max(axis=1)

        alpha = 1.0 / period
        atr_adx = tr.ewm(alpha=alpha, min_periods=period).mean()
        plus_di = 100 * plus_dm.ewm(alpha=alpha, min_periods=period).mean() / atr_adx
        minus_di = 100 * minus_dm.ewm(alpha=alpha, min_periods=period).mean() / atr_adx
        dx = 100 * (plus_di - minus_di).abs() / (plus_di + minus_di)
        df["adx"] = dx.ewm(alpha=alpha, min_periods=period).mean()
        df["plus_di"] = plus_di
        df["minus_di"] = minus_di

        # Momentum (close > close[20])
        mom_period = 20
        df["momentum"] = close - close.shift(mom_period)

        # ATR (same formula as fast sim)
        if "atr" not in df.columns:
            df["atr"] = close.pct_change().rolling(20).std() * close

        return df

    def check_entry(self, df: pd.DataFrame, idx: int,
                    params: Dict) -> Optional[EntrySignal]:
        close = df["close"].values[idx]
        sma = df["sma_fast"].values[idx] if "sma_fast" in df.columns else np.nan
        adx = df["adx"].values[idx] if "adx" in df.columns else 0
        momentum = df["momentum"].values[idx] if "momentum" in df.columns else 0
        plus_di = df["plus_di"].values[idx] if "plus_di" in df.columns else 0
        minus_di = df["minus_di"].values[idx] if "minus_di" in df.columns else 0

        adx_thresh = params.get("adx_threshold", 25)
        require_momentum = params.get("require_momentum", True)
        require_di = params.get("require_di_cross", True)

        if np.isnan(sma) or np.isnan(adx):
            return None

        # Above SMA + ADX confirms trend + positive momentum + DI+ > DI-
        above_sma = close > sma
        trend_confirmed = adx > adx_thresh
        mom_ok = (not require_momentum) or (momentum > 0)
        di_ok = (not require_di) or (plus_di > minus_di)

        if above_sma and trend_confirmed and mom_ok and di_ok:
            atr = df["atr"].values[idx] if "atr" in df.columns else 0
            return EntrySignal(direction=1, price=close, bar_idx=idx,
                               atr=float(atr) if not np.isnan(atr) else 0)

        return None

    def check_exit(self, df: pd.DataFrame, idx: int,
                   position: Dict, params: Dict) -> Optional[ExitReason]:
        close = df["close"].values[idx]
        sma = df["sma_fast"].values[idx] if "sma_fast" in df.columns else np.nan

        # Signal reversal: price crosses below SMA
        if not np.isnan(sma) and close < sma:
            return ExitReason.SIGNAL_EXIT

        return None

    def param_grid(self) -> Dict[str, List]:
        return {
            "adx_threshold": [20, 25, 30, 35],
            "require_momentum": [True, False],
            "require_di_cross": [True, False],
            "stop_loss_atr": [1.5, 2.0, 2.5, 3.0],
            "take_profit_atr": [3.0, 4.0, 5.0, 6.0],
            "max_hold_hours": [48, 72, 96],
            "time_decay_hours": [24, 36, 48],
            "trailing_stop_atr": [0.0, 0.5, 1.0, 1.5],
            "leverage": [3.0, 5.0, 7.0, 9.0],
        }

    def default_params(self) -> Dict:
        return {
            "adx_threshold": 25,
            "require_momentum": True,
            "require_di_cross": True,
            "stop_loss_atr": 2.0,
            "take_profit_atr": 4.0,
            "max_hold_hours": 72,
            "time_decay_hours": 36,
            "trailing_stop_atr": 1.0,
            "leverage": 1.0,
        }
