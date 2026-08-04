"""Strategy plugins — pluggable entry/exit logic for the night shift explorer."""
from research.strategy_plugins.base import StrategyPlugin, EntrySignal, ExitReason
from research.strategy_plugins.s02_breakout_band import BreakoutBandPlugin
from research.strategy_plugins.s04_rsi_exhaustion import RSIExhaustionPlugin
from research.strategy_plugins.s06_vol_squeeze import VolSqueezePlugin
from research.strategy_plugins.s13_adx_trend import ADXTrendPlugin
from research.strategy_plugins.s10_momentum_divergence import MomentumDivergencePlugin
from research.strategy_plugins.s14_marubozu_retracement import MarubozuRetracementPlugin

PLUGINS = {
    "S02": BreakoutBandPlugin,
    "S04": RSIExhaustionPlugin,
    "S06": VolSqueezePlugin,
    "S10": MomentumDivergencePlugin,
    "S13": ADXTrendPlugin,
    "S14": MarubozuRetracementPlugin,
}
