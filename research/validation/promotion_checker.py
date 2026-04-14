"""
Check whether a validated strategy meets the promotion gates
defined in promotion_criteria.py.

Called from validate_night_shift.py after the existing
STRONG/MODERATE/MARGINAL/FAILED verdict is computed.
"""
from research.promotion_criteria import PromotionGate


_PROMO = PromotionGate()


def _gate_result(passed: bool, value, threshold) -> dict:
    return {"pass": passed, "value": value, "threshold": threshold}


def _pending_gate(name: str, note: str) -> dict:
    return {"pass": True, "value": None, "threshold": None, "status": "PENDING", "note": note}


def check_promotion_eligibility(validation_result: dict) -> dict:
    """
    Evaluate a validation_result dict (output of validate_night_shift.py)
    against every PromotionGate threshold.

    Returns a structured verdict dict:
        {
            "strategy_id": str,
            "symbol": str,
            "eligible": bool,
            "gates": {gate_name: {"pass": bool, "value": ..., "threshold": ...}},
            "blocking_gates": [str],
            "recommendation": "PROMOTE" | "CONDITIONAL" | "REJECT",
        }
    """
    symbol = validation_result.get("symbol", "unknown")
    label = validation_result.get("label", "unknown")
    strategy_id = f"{symbol}_{label}"

    median_sharpe = validation_result.get("median_sharpe", 0.0)
    consistency = validation_result.get("consistency", 0.0)
    avg_win_rate = validation_result.get("avg_win_rate", 0.0)
    avg_pf = validation_result.get("avg_pf", 0.0)
    avg_max_dd = validation_result.get("avg_max_dd", 0.0)
    total_trades = validation_result.get("total_trades", 0)
    folds = validation_result.get("folds", [])

    # Count profitable OOS folds
    profitable_folds = sum(1 for f in folds if f.get("total_pnl_pct", 0) > 0)

    # Overfitting ratio: approximate IS vs OOS degradation.
    # If IS sharpe is unavailable, we approximate from the fold-level sharpe ceiling.
    # We use the max fold sharpe as a proxy for IS ceiling.
    if folds:
        max_fold_sharpe = max(abs(f.get("sharpe", 0)) for f in folds)
        is_proxy = max(max_fold_sharpe, abs(median_sharpe))
        overfitting_ratio = 1.0 - (abs(median_sharpe) / is_proxy) if is_proxy > 0 else 1.0
    else:
        overfitting_ratio = 1.0

    gates = {}

    # --- Statistical validity gates ---
    gates["oos_sharpe"] = _gate_result(
        median_sharpe >= _PROMO.MIN_OOS_SHARPE,
        round(median_sharpe, 2),
        _PROMO.MIN_OOS_SHARPE,
    )
    gates["oos_folds"] = _gate_result(
        profitable_folds >= _PROMO.MIN_OOS_FOLDS,
        profitable_folds,
        _PROMO.MIN_OOS_FOLDS,
    )
    gates["overfitting_ratio"] = _gate_result(
        overfitting_ratio <= _PROMO.MAX_OVERFITTING_RATIO,
        round(overfitting_ratio, 3),
        _PROMO.MAX_OVERFITTING_RATIO,
    )
    gates["win_rate"] = _gate_result(
        avg_win_rate >= _PROMO.MIN_WIN_RATE,
        round(avg_win_rate, 3),
        _PROMO.MIN_WIN_RATE,
    )
    gates["profit_factor"] = _gate_result(
        avg_pf >= _PROMO.MIN_PROFIT_FACTOR,
        round(avg_pf, 2),
        _PROMO.MIN_PROFIT_FACTOR,
    )
    gates["max_drawdown"] = _gate_result(
        avg_max_dd <= _PROMO.MAX_DRAWDOWN_PCT,
        round(avg_max_dd, 2),
        _PROMO.MAX_DRAWDOWN_PCT,
    )

    # --- Stubs for gates requiring live data ---
    gates["regime_robustness"] = _pending_gate(
        "regime_robustness",
        "Requires regime classification per fold (trending/ranging/high-vol)",
    )
    gates["paper_trading"] = _pending_gate(
        "paper_trading",
        f"Requires >= {_PROMO.MIN_PAPER_HOURS}h of live paper trading confirmation",
    )
    gates["portfolio_fit"] = _pending_gate(
        "portfolio_fit",
        f"Requires rolling correlation < {_PROMO.MAX_PORTFOLIO_CORRELATION} with existing live strategies",
    )
    gates["swarm_consensus"] = _pending_gate(
        "swarm_consensus",
        f"Requires >= {_PROMO.MIN_APPROVALS} of 3 validator agents to vote APPROVE",
    )

    blocking = [name for name, g in gates.items() if not g["pass"]]
    has_pending = any(g.get("status") == "PENDING" for g in gates.values())

    if not blocking and not has_pending:
        recommendation = "PROMOTE"
    elif not blocking and has_pending:
        recommendation = "CONDITIONAL"
    else:
        recommendation = "REJECT"

    eligible = len(blocking) == 0

    return {
        "strategy_id": strategy_id,
        "symbol": symbol,
        "eligible": eligible,
        "gates": gates,
        "blocking_gates": blocking,
        "recommendation": recommendation,
    }


def print_promotion_summary(verdict: dict) -> None:
    """Print a human-readable PROMOTION ELIGIBILITY block."""
    rec = verdict["recommendation"]
    sym = verdict["symbol"]
    sid = verdict["strategy_id"]
    eligible = verdict["eligible"]
    blocking = verdict["blocking_gates"]

    status_icon = "+" if eligible else "-"
    print(f"\n  PROMOTION ELIGIBILITY [{status_icon}] {sid}")
    print(f"    Recommendation: {rec}")
    print(f"    Eligible: {eligible}")

    for gate_name, gate in verdict["gates"].items():
        if gate.get("status") == "PENDING":
            mark = "?"
        elif gate["pass"]:
            mark = "+"
        else:
            mark = "FAIL"
        val_str = f"{gate['value']}" if gate['value'] is not None else "n/a"
        thr_str = f"{gate['threshold']}" if gate['threshold'] is not None else "n/a"
        print(f"      [{mark:>4s}] {gate_name:22s}  value={val_str:>8s}  threshold={thr_str:>8s}")

    if blocking:
        print(f"    Blocking gates: {', '.join(blocking)}")
    print()
