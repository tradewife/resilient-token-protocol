"""
Base class for strategy plugins.

Each plugin provides entry/exit logic that the fast simulator can use
instead of the default MultiTF Survivor strategy. Plugins receive the
same indicator-augmented DataFrame and return standardized signals.
"""
from dataclasses import dataclass
from enum import Enum
from typing import Dict, List, Optional

import numpy as np
import pandas as pd


class ExitReason(Enum):
    TAKE_PROFIT = "take_profit"
    STOP_LOSS = "stop_loss"
    TRAILING_STOP = "trailing_stop"
    MAX_HOLD = "max_hold"
    TIME_DECAY = "time_decay"
    SIGNAL_EXIT = "signal_exit"
    TARGET_REACHED = "target_reached"


@dataclass
class EntrySignal:
    """Entry signal from a plugin."""
    direction: int          # 1 = long, -1 = short
    price: float
    bar_idx: int
    atr: float              # ATR at entry (for SL/TP calculation)


class StrategyPlugin:
    """Base class for strategy plugins.

    Subclasses must implement:
        compute_indicators(df) -> pd.DataFrame
        check_entry(df, idx, params) -> Optional[EntrySignal]
        check_exit(df, idx, position, params) -> Optional[ExitReason]

    And define:
        param_grid() -> Dict[str, List]
        default_params() -> Dict
        name() -> str
        description() -> str
    """

    def compute_indicators(self, df: pd.DataFrame) -> pd.DataFrame:
        """Add plugin-specific indicator columns to df."""
        return df

    def check_entry(self, df: pd.DataFrame, idx: int,
                    params: Dict) -> Optional[EntrySignal]:
        """Check for entry signal at bar idx. Returns None if no entry."""
        raise NotImplementedError

    def check_exit(self, df: pd.DataFrame, idx: int,
                   position: Dict, params: Dict) -> Optional[ExitReason]:
        """Check for exit at bar idx. Returns None if no exit."""
        raise NotImplementedError

    def param_grid(self) -> Dict[str, List]:
        """Return parameter grid for optimization."""
        raise NotImplementedError

    def default_params(self) -> Dict:
        """Return default parameter values."""
        raise NotImplementedError

    @property
    def name(self) -> str:
        raise NotImplementedError

    @property
    def description(self) -> str:
        raise NotImplementedError


def simulate_plugin_trades(
    df: pd.DataFrame,
    plugin: StrategyPlugin,
    params: Dict,
    leverage: float = 1.0,
) -> List[Dict]:
    """
    Simulate trades using a strategy plugin. Mirrors the fast simulator's
    interface: returns a list of trip dicts with pnl_pct, hold_hrs, exit.

    This is the generic simulator that any plugin can use.
    """
    close = df["close"].values
    warmup = 250

    in_position = False
    entry_price = 0.0
    entry_idx = 0
    direction = 1
    atr_at_entry = 0.0
    peak_price = 0.0

    # Default exit params
    sl_mult = params.get("stop_loss_atr", 2.0)
    tp_mult = params.get("take_profit_atr", 3.0)
    max_hold = params.get("max_hold_hours", 48)
    decay_hours = params.get("time_decay_hours", 24)
    trail_atr = params.get("trailing_stop_atr", 0.0)

    trips = []

    for i in range(warmup, len(close)):
        price = close[i]

        if in_position:
            hold_hrs = i - entry_idx
            pnl_raw = (price - entry_price) / entry_price * 100 * direction
            pnl_pct = pnl_raw * leverage

            if price > peak_price if direction == 1 else price < peak_price:
                peak_price = price

            # Plugin-specific exit
            exit_reason = plugin.check_exit(df, i, {
                "entry_price": entry_price,
                "entry_idx": entry_idx,
                "direction": direction,
                "peak_price": peak_price,
                "atr_at_entry": atr_at_entry,
            }, params)

            if exit_reason is not None:
                trips.append({
                    "pnl_pct": pnl_pct,
                    "hold_hrs": hold_hrs,
                    "exit": exit_reason.value if isinstance(exit_reason, ExitReason) else exit_reason,
                })
                in_position = False
                continue

            # Generic ATR stop loss
            atr = df["atr"].values[i] if "atr" in df.columns else 0
            if atr > 0:
                sl_pct = sl_mult * atr / entry_price * 100 * leverage
                if pnl_pct <= -sl_pct:
                    trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs,
                                  "exit": "stop_loss"})
                    in_position = False
                    continue

            # Generic ATR take profit
            if atr > 0:
                tp_pct = tp_mult * atr / entry_price * 100 * leverage
                if pnl_pct >= tp_pct:
                    trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs,
                                  "exit": "take_profit"})
                    in_position = False
                    continue

            # Trailing stop
            if trail_atr > 0 and atr > 0:
                trail_dist = trail_atr * atr / entry_price * 100 * leverage
                if direction == 1:
                    trail_price = peak_price - trail_atr * atr / leverage
                    if price <= trail_price:
                        trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs,
                                      "exit": "trailing_stop"})
                        in_position = False
                        continue
                else:
                    trail_price = peak_price + trail_atr * atr / leverage
                    if price >= trail_price:
                        trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs,
                                      "exit": "trailing_stop"})
                        in_position = False
                        continue

            # Max hold
            if hold_hrs >= max_hold:
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs,
                              "exit": "max_hold"})
                in_position = False
                continue

            # Time decay (exit if losing after decay threshold)
            if pnl_pct < 0 and hold_hrs >= decay_hours:
                trips.append({"pnl_pct": pnl_pct, "hold_hrs": hold_hrs,
                              "exit": "time_decay"})
                in_position = False
                continue

            # Liquidation check
            if pnl_pct <= -100:
                trips.append({"pnl_pct": -100, "hold_hrs": hold_hrs,
                              "exit": "liquidation", "liquidated": True})
                in_position = False
                continue

        else:
            # Check for entry
            signal = plugin.check_entry(df, i, params)
            if signal is not None:
                in_position = True
                entry_price = signal.price
                entry_idx = signal.bar_idx
                direction = signal.direction
                atr_at_entry = signal.atr
                peak_price = entry_price

    return trips
