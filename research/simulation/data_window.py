"""
DataWindow — time-windowed market data container for future-blind simulations.

Extracted from agents/historical_data_collector.py. This is a simulation
primitive, not an autonomous agent.
"""

import pandas as pd
from datetime import timedelta
from dataclasses import dataclass


@dataclass
class DataWindow:
    """Represents a time window of market data that agents can see"""
    symbol: str
    exchange: str
    start_time: 'datetime'
    end_time: 'datetime'
    current_time: 'datetime'  # Simulation current time
    data: pd.DataFrame

    def get_visible_data(self) -> pd.DataFrame:
        """Returns only data up to current simulation time"""
        return self.data[self.data.index <= self.current_time]

    def advance_time(self, minutes: int = 1):
        """Advance simulation time by specified minutes"""
        self.current_time += timedelta(minutes=minutes)
        if self.current_time > self.end_time:
            self.current_time = self.end_time

    def has_more_data(self) -> bool:
        """Check if there's more future data available"""
        return self.current_time < self.end_time
