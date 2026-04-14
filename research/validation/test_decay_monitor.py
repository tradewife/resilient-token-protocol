"""
Tests for DecayMonitor — verify hard stop suspension and soft decay retirement.
"""
from datetime import datetime, timedelta, timezone

from research.promotion_criteria import DecayRisk, StrategyStatus
from research.validation.decay_monitor import DecayMonitor


def _make_monitor(decay_risk: DecayRisk = DecayRisk.MEDIUM) -> DecayMonitor:
    return DecayMonitor(
        strategy_id="TEST_S01",
        symbol="SOL/USDT",
        promotion_sharpe=3.0,
        decay_risk=decay_risk,
    )


def _ts(hours_ago: float) -> datetime:
    return datetime.now(timezone.utc) - timedelta(hours=hours_ago)


# ---------- Hard stops ----------


def test_no_hard_stop_with_profitable_trades():
    mon = _make_monitor()
    for i in range(10):
        mon.record_trade(1.0, _ts(24 - i * 2))
    result = mon.check_hard_stops()
    assert result["triggered"] is False
    assert mon.get_status() == StrategyStatus.LIVE


def test_hard_stop_consecutive_losses():
    mon = _make_monitor()
    # Use small losses to avoid the 24h drawdown gate (10%)
    for i in range(5):
        mon.record_trade(-0.5, _ts(24 - i * 2))
    result = mon.check_hard_stops()
    assert result["triggered"] is True
    assert "consecutive losses" in result["reason"]
    assert mon.get_status() == StrategyStatus.SUSPENDED


def test_hard_stop_24h_drawdown():
    mon = _make_monitor()
    # Big winning trade then a series of losses causing >10% drawdown
    mon.record_trade(5.0, _ts(48))
    mon.record_trade(-4.0, _ts(20))
    mon.record_trade(-4.0, _ts(16))
    mon.record_trade(-4.0, _ts(12))
    mon.record_trade(-4.0, _ts(8))
    result = mon.check_hard_stops()
    assert result["triggered"] is True
    assert "24h drawdown" in result["reason"]
    assert mon.get_status() == StrategyStatus.SUSPENDED


# ---------- Soft decay ----------


def test_no_soft_decay_with_good_performance():
    mon = _make_monitor()
    for i in range(20):
        mon.record_trade(1.5, _ts(48 - i * 2))
    result = mon.check_soft_decay(
        current_regime="trending",
        strategy_regime_fit="trending",
    )
    assert result["retire"] is False
    assert result["strikes"] == 0
    assert mon.get_status() == StrategyStatus.LIVE


def test_soft_decay_retirement_after_three_strikes():
    mon = _make_monitor()
    # Poor trades to tank rolling Sharpe and win rate
    for i in range(30):
        pnl = -0.5 if i % 2 == 0 else 0.1
        mon.record_trade(pnl, _ts(72 - i * 2))

    # Strike 1 + 2: low Sharpe + low win rate
    result1 = mon.check_soft_decay(
        current_regime="ranging",
        strategy_regime_fit="trending",
    )
    assert result1["retire"] is False
    assert result1["strikes"] >= 2

    # Strike 3: correlation creep
    result2 = mon.check_soft_decay(
        current_regime="trending",
        strategy_regime_fit="trending",
        portfolio_correlation=0.7,
    )
    assert result2["retire"] is True
    assert mon.get_status() == StrategyStatus.RETIRED


def test_to_dict_serialisation():
    mon = _make_monitor(DecayRisk.LOW)
    for i in range(5):
        mon.record_trade(1.0, _ts(24 - i * 2))
    d = mon.to_dict()
    assert d["strategy_id"] == "TEST_S01"
    assert d["decay_risk"] == "low"
    assert d["window_days"] == 45
    assert d["total_trades"] == 5
    assert d["status"] == "live"


def test_window_pruning():
    mon = _make_monitor(DecayRisk.HIGH)  # 14-day window
    base = datetime(2026, 4, 1, tzinfo=timezone.utc)
    # Old trades outside window
    for i in range(5):
        mon.record_trade(1.0, base + timedelta(days=i))
    # Recent trades inside window
    for i in range(5):
        mon.record_trade(2.0, base + timedelta(days=15 + i))
    assert len(mon._trades) == 5
    assert all(pnl == 2.0 for _, pnl in mon._trades)
