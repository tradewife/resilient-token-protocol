"""
DecayMonitor — tracks live strategy performance against RetirementGate thresholds.

Records completed trades in a rolling window sized by DecayRisk, checks hard stops
and soft decay signals, and returns the current StrategyStatus.
"""
from datetime import datetime, timedelta, timezone
from typing import Optional

from research.promotion_criteria import (
    DecayRisk,
    RetirementGate,
    StrategyStatus,
)


class DecayMonitor:
    def __init__(
        self,
        strategy_id: str,
        symbol: str,
        promotion_sharpe: float,
        decay_risk: DecayRisk,
    ):
        self.strategy_id = strategy_id
        self.symbol = symbol
        self.promotion_sharpe = promotion_sharpe
        self.decay_risk = decay_risk
        self._gate = RetirementGate()
        self._window_days = decay_risk.window_days

        # Trade log: list of (timestamp, pnl_pct)
        self._trades: list[tuple[datetime, float]] = []
        # Soft strike state
        self._strikes: int = 0
        self._strike_signals: list[str] = []
        # Hard stop state
        self._suspended: bool = False
        self._suspension_reason: Optional[str] = None

    # ---- trade recording ----

    def record_trade(self, pnl_pct: float, timestamp: datetime) -> None:
        """Append a completed trade and prune the rolling window."""
        self._trades.append((timestamp, pnl_pct))
        self._prune_window(timestamp)

    def _prune_window(self, now: datetime) -> None:
        cutoff = now - timedelta(days=self._window_days)
        self._trades = [(ts, pnl) for ts, pnl in self._trades if ts >= cutoff]

    # ---- hard stops ----

    def check_hard_stops(self) -> dict:
        """
        Returns {"triggered": bool, "reason": str | None}
        Checks HARD_DRAWDOWN_24H_PCT, HARD_CONSECUTIVE_LOSSES,
        HARD_ROLLING_SHARPE_MIN.
        """
        now = self._trades[-1][0] if self._trades else datetime.now(timezone.utc)
        cutoff_24h = now - timedelta(hours=24)
        recent = [(ts, pnl) for ts, pnl in self._trades if ts >= cutoff_24h]

        # 1. 24h drawdown
        if recent:
            pnls_24h = [pnl for _, pnl in recent]
            cum = 0.0
            peak = 0.0
            max_dd = 0.0
            for pnl in pnls_24h:
                cum += pnl
                peak = max(peak, cum)
                max_dd = max(max_dd, peak - cum)
            if max_dd >= self._gate.HARD_DRAWDOWN_24H_PCT:
                reason = (
                    f"24h drawdown {max_dd:.2f}% >= "
                    f"{self._gate.HARD_DRAWDOWN_24H_PCT}%"
                )
                self._suspended = True
                self._suspension_reason = reason
                return {"triggered": True, "reason": reason}

        # 2. Consecutive losses
        if len(self._trades) >= self._gate.HARD_CONSECUTIVE_LOSSES:
            tail = [pnl for _, pnl in self._trades[-self._gate.HARD_CONSECUTIVE_LOSSES:]]
            if all(pnl <= 0 for pnl in tail):
                reason = (
                    f"{self._gate.HARD_CONSECUTIVE_LOSSES} consecutive losses"
                )
                self._suspended = True
                self._suspension_reason = reason
                return {"triggered": True, "reason": reason}

        # 3. Rolling Sharpe
        if len(self._trades) >= 5:
            pnls = [pnl for _, pnl in self._trades]
            import numpy as np

            mean_pnl = np.mean(pnls)
            std_pnl = np.std(pnls)
            if std_pnl > 0:
                rolling_sharpe = mean_pnl / std_pnl
                if rolling_sharpe < self._gate.HARD_ROLLING_SHARPE_MIN:
                    reason = (
                        f"Rolling Sharpe {rolling_sharpe:.2f} < "
                        f"{self._gate.HARD_ROLLING_SHARPE_MIN}"
                    )
                    self._suspended = True
                    self._suspension_reason = reason
                    return {"triggered": True, "reason": reason}

        return {"triggered": False, "reason": None}

    # ---- soft decay ----

    def check_soft_decay(
        self,
        current_regime: str,
        strategy_regime_fit: str,
        current_funding_rate: Optional[float] = None,
        portfolio_correlation: Optional[float] = None,
    ) -> dict:
        """
        Returns {"strikes": int, "signals": [str], "retire": bool}
        Increments strike counter for each soft signal triggered.
        """
        new_signals: list[str] = []

        # Need enough trades for meaningful statistics
        if len(self._trades) >= 5:
            import numpy as np

            pnls = [pnl for _, pnl in self._trades]
            mean_pnl = np.mean(pnls)
            std_pnl = np.std(pnls)

            # 1. Sharpe ratio of promo
            if std_pnl > 0:
                rolling_sharpe = mean_pnl / std_pnl
                threshold_sharpe = (
                    self.promotion_sharpe * self._gate.SOFT_SHARPE_RATIO_OF_PROMO
                )
                if rolling_sharpe < threshold_sharpe:
                    new_signals.append(
                        f"Sharpe {rolling_sharpe:.2f} < "
                        f"{threshold_sharpe:.2f} (50% of promo)"
                    )

        # 2. Win rate over last 50 trades
        recent_trades = self._trades[-50:]
        if len(recent_trades) >= 10:
            wins = sum(1 for _, pnl in recent_trades if pnl > 0)
            wr = wins / len(recent_trades)
            if wr < self._gate.SOFT_MIN_WIN_RATE:
                new_signals.append(
                    f"Win rate {wr:.2%} < {self._gate.SOFT_MIN_WIN_RATE:.2%}"
                )

        # 3. Regime mismatch (checked externally, passed as params)
        # The caller tracks how many consecutive days of mismatch have elapsed.
        # We just check if the regime is currently mismatched and let the caller
        # accumulate the day count.
        if current_regime != strategy_regime_fit and strategy_regime_fit != "both":
            new_signals.append(
                f"Regime mismatch: active={current_regime}, fit={strategy_regime_fit}"
            )

        # 4. Funding floor (for carry strategies)
        if current_funding_rate is not None:
            if abs(current_funding_rate) < self._gate.SOFT_FUNDING_FLOOR_PCT:
                new_signals.append(
                    f"Funding rate {current_funding_rate:.4f}% < "
                    f"{self._gate.SOFT_FUNDING_FLOOR_PCT}%"
                )

        # 5. Correlation creep
        if portfolio_correlation is not None:
            if portfolio_correlation > self._gate.SOFT_CORRELATION_CREEP:
                new_signals.append(
                    f"Correlation {portfolio_correlation:.2f} > "
                    f"{self._gate.SOFT_CORRELATION_CREEP}"
                )

        self._strikes += len(new_signals)
        self._strike_signals.extend(new_signals)

        retire = self._strikes >= self._gate.SOFT_STRIKE_THRESHOLD

        return {
            "strikes": self._strikes,
            "signals": new_signals,
            "retire": retire,
        }

    # ---- status ----

    def get_status(self) -> StrategyStatus:
        """Returns current StrategyStatus based on hard stop and strike state."""
        if self._suspended:
            return StrategyStatus.SUSPENDED
        if self._strikes >= self._gate.SOFT_STRIKE_THRESHOLD:
            return StrategyStatus.RETIRED
        return StrategyStatus.LIVE

    # ---- serialisation ----

    def to_dict(self) -> dict:
        """Serialise full monitor state for on-chain audit log."""
        return {
            "strategy_id": self.strategy_id,
            "symbol": self.symbol,
            "promotion_sharpe": self.promotion_sharpe,
            "decay_risk": self.decay_risk.value,
            "window_days": self._window_days,
            "total_trades": len(self._trades),
            "strikes": self._strikes,
            "strike_signals": self._strike_signals,
            "suspended": self._suspended,
            "suspension_reason": self._suspension_reason,
            "status": self.get_status().value,
        }
