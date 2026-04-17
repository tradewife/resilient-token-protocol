"""
Single source-of-truth for strategy promotion and retirement thresholds.

Importable by both the research pipeline and the execution layer.
All numeric thresholds live here — no magic numbers in consumers.
"""
from dataclasses import dataclass
from enum import Enum


class StrategyStatus(Enum):
    RESEARCH = "research"
    PAPER_TRADING = "paper_trading"
    LIVE = "live"
    SUSPENDED = "suspended"
    RETIRED = "retired"


class DecayRisk(Enum):
    LOW = "low"          # tighter rolling window: 45 days
    MEDIUM = "medium"    # standard: 30 days
    HIGH = "high"        # aggressive: 14 days

    @property
    def window_days(self) -> int:
        return {DecayRisk.LOW: 45, DecayRisk.MEDIUM: 30, DecayRisk.HIGH: 14}[self]


@dataclass(frozen=True)
class PromotionGate:
    """Statistical thresholds a strategy must clear before going live."""

    # Statistical validity
    MIN_OOS_SHARPE: float = 1.5
    MIN_OOS_FOLDS: int = 3
    MAX_OVERFITTING_RATIO: float = 0.4
    MIN_WIN_RATE: float = 0.45
    MIN_PROFIT_FACTOR: float = 1.3
    MAX_DRAWDOWN_PCT: float = 20.0

    # Regime robustness
    MIN_PROFITABLE_REGIMES: int = 2

    # Paper trading confirmation
    MIN_PAPER_HOURS: int = 72
    MAX_SLIPPAGE_MULTIPLIER: float = 1.5

    # Portfolio fit
    MAX_PORTFOLIO_CORRELATION: float = 0.4

    # Swarm consensus
    MIN_APPROVALS: int = 2


@dataclass(frozen=True)
class RetirementGate:
    """Thresholds that trigger strategy suspension or retirement."""

    # Hard stops (immediate suspension)
    HARD_DRAWDOWN_24H_PCT: float = 10.0
    HARD_CONSECUTIVE_LOSSES: int = 5
    HARD_ROLLING_SHARPE_MIN: float = 0.5

    # Soft decay signals (3 strikes = retire)
    SOFT_SHARPE_RATIO_OF_PROMO: float = 0.5
    SOFT_MIN_WIN_RATE: float = 0.38
    SOFT_REGIME_MISMATCH_DAYS: int = 5
    SOFT_FUNDING_FLOOR_PCT: float = 0.01
    SOFT_CORRELATION_CREEP: float = 0.6
    SOFT_STRIKE_THRESHOLD: int = 3
