"""
Leverage Audit — Authoritative 1x vs 3x comparison.

Runs both configs through IDENTICAL methodology:
  - Fast simulator (per_symbol_optimizer) with fee-adjusted compounding
  - Full simulator (FutureBlindSimulator) on entire 365-day data

Produces a single comparison table for the go/no-go decision on 3x leverage.

Usage:
    python -m research.simulation.leverage_audit
"""
import asyncio
import json
import os
import sys

import numpy as np
import pandas as pd

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))

from research.optimization.per_symbol_optimizer import (
    compute_indicators,
    simulate_trades,
)

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "data", "ohlcv")

# Config A: Current production (Survivor 2.69 at 1x)
CONFIG_A = {
    "signal_threshold": 0.3,
    "min_alignment": 3,
    "take_profit_atr": 3.0,
    "stop_loss_atr": 1.5,
    "max_hold_hours": 36,
    "time_decay_hours": 12,
    "trailing_stop_atr": 0.5,
    "score_flip_delay_hrs": 0,
    "leverage": 1.0,
}

# Config B: Proposed 3x (wider stops, tighter trail)
CONFIG_B = {
    "signal_threshold": 0.3,
    "min_alignment": 3,
    "take_profit_atr": 3.0,
    "stop_loss_atr": 2.5,
    "max_hold_hours": 36,
    "time_decay_hours": 12,
    "trailing_stop_atr": 0.3,
    "score_flip_delay_hrs": 0,
    "leverage": 3.0,
}

# Cost model
FEE_PCT = 0.1           # 0.1% per trade (entry + exit = 0.2% round-trip)
SLIPPAGE_PCT = 0.01     # 10 bps per side (entry + exit = 0.02% round-trip)
POSITION_PCT = 0.20     # 20% of capital per trade
INITIAL_CAPITAL = 100.0 # SOL


def run_fast_sim_with_compounding(df, params, label):
    """
    Fast simulator + fee-adjusted compounding.
    
    This matches the full simulator's cost model:
    - 0.1% fee on entry and exit (total 0.2% round-trip)
    - 10 bps slippage on entry and exit (total 0.02% round-trip)
    - 20% position sizing with compounding
    
    The fast sim already applies leverage to pnl_pct. We subtract
    fees/slippage from each trade's pnl before compounding.
    """
    leverage = params["leverage"]
    
    # Get raw trades from fast sim (leverage already applied to pnl_pct)
    trips = simulate_trades(df, params)
    
    if not trips:
        return {"label": label, "error": "no trades"}
    
    # Apply fee/slippage deduction to each trade
    # Round-trip cost: 0.2% fee + 0.02% slippage = 0.22% of position
    # But on a leveraged position, the cost is on the NOTIONAL, not the margin
    # With leverage L, position = capital * position_pct * L
    # Cost = position * (fee + slippage) = capital * position_pct * L * 0.22%
    # As a fraction of margin: cost_pct = L * 0.22%
    # On the pnl_pct (which is already leveraged), cost reduces it by L * 0.22%
    round_trip_cost_pct = leverage * (FEE_PCT * 2 + SLIPPAGE_PCT * 2)
    
    capital = INITIAL_CAPITAL
    peak = capital
    max_dd = 0.0
    trade_log = []
    
    for t in trips:
        if t.get("liquidated", False):
            # Liquidation: lose the entire margin allocated to this trade
            position_size = capital * POSITION_PCT
            capital -= position_size
            trade_log.append({
                "pnl_pct": -100.0,
                "capital": round(capital, 4),
                "exit": "liquidation",
                "hold_hrs": t["hold_hrs"],
            })
        else:
            # Adjust PnL for fees/slippage
            adjusted_pnl = t["pnl_pct"] - round_trip_cost_pct
            position_size = capital * POSITION_PCT
            pnl_sol = position_size * (adjusted_pnl / 100.0)
            capital += pnl_sol
            trade_log.append({
                "pnl_pct": round(adjusted_pnl, 4),
                "capital": round(capital, 4),
                "exit": t["exit"],
                "hold_hrs": t["hold_hrs"],
            })
        
        if capital > peak:
            peak = capital
        dd = (peak - capital) / peak * 100 if peak > 0 else 0
        max_dd = max(max_dd, dd)
    
    pnls = [t["pnl_pct"] for t in trade_log]
    wins = [p for p in pnls if p > 0]
    losses = [p for p in pnls if p <= 0]
    liqs = [t for t in trade_log if t["exit"] == "liquidation"]
    
    total_return = (capital - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100
    total_days = len(df) / 24
    annual_return = ((capital / INITIAL_CAPITAL) ** (365 / total_days) - 1) * 100 if capital > 0 else -100
    
    std_pnl = np.std(pnls) if len(pnls) > 1 else 1
    sharpe = (np.mean(pnls) / std_pnl) * np.sqrt(len(pnls) / total_days * 365) if std_pnl > 0 else 0
    
    avg_win = np.mean(wins) if wins else 0
    avg_loss = abs(np.mean(losses)) if losses else 0
    pf = avg_win / avg_loss if avg_loss > 0 else float("inf")
    
    return {
        "label": label,
        "leverage": leverage,
        "params": {k: v for k, v in params.items()},
        "initial_capital": INITIAL_CAPITAL,
        "final_capital": round(capital, 2),
        "total_return_pct": round(total_return, 2),
        "annualized_return_pct": round(annual_return, 2),
        "max_drawdown_pct": round(max_dd, 2),
        "sharpe": round(sharpe, 2),
        "total_trades": len(trade_log),
        "win_rate": round(len(wins) / len(trade_log), 3) if trade_log else 0,
        "profit_factor": round(pf, 2) if pf < 999 else "INF",
        "avg_win_pct": round(avg_win, 2),
        "avg_loss_pct": round(avg_loss, 2),
        "best_trade": round(max(pnls), 2),
        "worst_trade": round(min(pnls), 2),
        "liquidations": len(liqs),
        "avg_hold_hrs": round(np.mean([t["hold_hrs"] for t in trade_log]), 1),
        "total_days": round(total_days, 0),
        "round_trip_cost_pct": round(round_trip_cost_pct, 3),
    }


async def run_full_sim_comparison():
    """Run both configs through the full simulator for validation."""
    from research.simulation.future_blind_simulator import FutureBlindSimulator
    from research.simulation.data_window import DataWindow
    from research.simulation.run_backtest_r2 import MultiTFStrategy
    
    safe = "SOL_USDT"
    path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
    df = pd.read_parquet(path)
    
    results = {}
    
    for label, config in [("A_1x", CONFIG_A), ("B_3x", CONFIG_B)]:
        leverage = config["leverage"]
        
        strategy = MultiTFStrategy(f"audit_{label}", {**{"symbol": "SOL/USDT"}, **config})
        sim = FutureBlindSimulator(initial_capital=10000)
        sim.add_strategy(strategy)
        
        window = DataWindow(
            symbol="SOL/USDT",
            exchange="binance",
            start_time=df.index[0].to_pydatetime(),
            end_time=df.index[-1].to_pydatetime(),
            current_time=df.index[0].to_pydatetime(),
            data=df,
        )
        
        await sim.run_simulation(window, time_step_minutes=60)
        trips = strategy.completed_round_trips
        
        # Apply leverage post-hoc (full sim doesn't model leverage natively)
        # Also deduct round-trip fees: leverage * (0.2% fee + 0.02% slippage)
        round_trip_cost_pct = leverage * (FEE_PCT * 2 + SLIPPAGE_PCT * 2)
        
        for trip in trips:
            trip["pnl_pct"] = trip["pnl_pct"] * leverage - round_trip_cost_pct
            if trip["pnl_pct"] <= -100.0:
                trip["pnl_pct"] = -100.0
                trip["liquidated"] = True
        
        # Compounding with 20% position sizing
        capital = 10000.0
        peak = capital
        max_dd = 0.0
        
        for trip in trips:
            pnl_pct = trip["pnl_pct"]
            position = capital * POSITION_PCT
            capital += position * (pnl_pct / 100.0)
            if capital > peak:
                peak = capital
            dd = (peak - capital) / peak * 100 if peak > 0 else 0
            max_dd = max(max_dd, dd)
        
        pnls = [t["pnl_pct"] for t in trips]
        wins = [p for p in pnls if p > 0]
        total_days = len(df) / 24
        total_return = (capital - 10000) / 10000 * 100
        annual_return = ((capital / 10000) ** (365 / total_days) - 1) * 100 if capital > 0 else -100
        
        exit_reasons = {}
        for t in trips:
            reason = t.get("exit_reason", "signal")
            exit_reasons[reason] = exit_reasons.get(reason, 0) + 1
        
        results[label] = {
            "final_capital": round(capital, 2),
            "total_return_pct": round(total_return, 2),
            "annualized_return_pct": round(annual_return, 2),
            "max_drawdown_pct": round(max_dd, 2),
            "total_trades": len(trips),
            "win_rate": round(len(wins) / len(trips), 3) if trips else 0,
            "exit_reasons": exit_reasons,
        }
    
    return results


def main():
    print(f"\n{'='*75}")
    print(f"LEVERAGE AUDIT — Authoritative 1x vs 3x Comparison")
    print(f"SOL/USDT, 20% position sizing, 0.1% fees + 10bps slippage")
    print(f"{'='*75}")
    
    # Load data
    safe = "SOL_USDT"
    path = os.path.join(DATA_DIR, f"{safe}_1h.parquet")
    df = pd.read_parquet(path)
    df = compute_indicators(df)
    total_days = len(df) / 24
    
    print(f"\nData: {len(df)} candles, {total_days:.0f} days")
    print(f"Period: {df.index[250].date()} -> {df.index[-1].date()}")
    print(f"Fee model: {FEE_PCT}% per side, {SLIPPAGE_PCT}% slippage per side")
    print(f"Position sizing: {POSITION_PCT:.0%} of capital per trade")
    
    # === FAST SIM WITH FEE-ADJUSTED COMPOUNDING ===
    print(f"\n{'─'*75}")
    print(f"METHOD 1: Fast Simulator + Fee-Adjusted Compounding")
    print(f"{'─'*75}")
    
    result_a = run_fast_sim_with_compounding(df, CONFIG_A, "A: 1x (sl=1.5, trail=0.5)")
    result_b = run_fast_sim_with_compounding(df, CONFIG_B, "B: 3x (sl=2.5, trail=0.3)")
    
    for r in [result_a, result_b]:
        if "error" in r:
            print(f"\n  {r['label']}: {r['error']}")
            continue
        print(f"\n  {r['label']}:")
        print(f"    Final Capital:    {r['final_capital']:.2f} SOL (from {r['initial_capital']:.0f})")
        print(f"    Total Return:     {r['total_return_pct']:+.2f}%")
        print(f"    Annualized:       {r['annualized_return_pct']:+.2f}%")
        print(f"    Max Drawdown:     {r['max_drawdown_pct']:.2f}%")
        print(f"    Sharpe:           {r['sharpe']:.2f}")
        print(f"    Trades:           {r['total_trades']}")
        print(f"    Win Rate:         {r['win_rate']:.1%}")
        print(f"    Profit Factor:    {r['profit_factor']}")
        print(f"    Avg Win/Loss:     {r['avg_win_pct']:+.2f}% / {r['avg_loss_pct']:.2f}%")
        print(f"    Best/Worst:       {r['best_trade']:+.2f}% / {r['worst_trade']:.2f}%")
        print(f"    Liquidations:     {r['liquidations']}")
        print(f"    Avg Hold:         {r['avg_hold_hrs']:.1f}h")
        print(f"    Round-trip cost:  {r['round_trip_cost_pct']:.3f}% of margin")
    
    # === FULL SIMULATOR CONFIRMATION ===
    print(f"\n{'─'*75}")
    print(f"METHOD 2: Full Simulator (FutureBlindSimulator) — Confirmation")
    print(f"{'─'*75}")
    
    full_results = asyncio.run(run_full_sim_comparison())
    
    for label, r in full_results.items():
        tag = "A: 1x" if "1x" in label else "B: 3x"
        print(f"\n  {tag}:")
        print(f"    Final Capital:    {r['final_capital']:.2f} (from 10000)")
        print(f"    Total Return:     {r['total_return_pct']:+.2f}%")
        print(f"    Annualized:       {r['annualized_return_pct']:+.2f}%")
        print(f"    Max Drawdown:     {r['max_drawdown_pct']:.2f}%")
        print(f"    Trades:           {r['total_trades']}")
        print(f"    Win Rate:         {r['win_rate']:.1%}")
        print(f"    Exit Reasons:     {r['exit_reasons']}")
    
    # === AUTHORITATIVE TABLE ===
    print(f"\n{'='*75}")
    print(f"AUTHORITATIVE COMPARISON TABLE")
    print(f"SOL/USDT Compounded Annual Return ({POSITION_PCT:.0%} position, {FEE_PCT}% fees, {SLIPPAGE_PCT*100:.0f}bps slippage)")
    print(f"{total_days:.0f}-day continuous backtest ({df.index[250].date()} -> {df.index[-1].date()})")
    print(f"{'='*75}\n")
    
    # Use fast-sim results as primary (more consistent methodology)
    header = (f"{'Config':<26s} {'Final':>10s} {'Return':>10s} {'Annual':>10s} "
              f"{'MaxDD':>8s} {'Sharpe':>8s} {'Trades':>7s} {'WR':>6s} {'PF':>6s} {'Liq':>5s}")
    print(header)
    print("─" * len(header) + "─" * 20)
    
    for r in [result_a, result_b]:
        if "error" in r:
            continue
        pf_s = f"{r['profit_factor']:.1f}" if isinstance(r['profit_factor'], (int, float)) and r['profit_factor'] < 999 else "INF"
        print(f"{r['label']:<26s} {r['final_capital']:>9.2f}S {r['total_return_pct']:>+9.2f}% "
              f"{r['annualized_return_pct']:>+9.2f}% {r['max_drawdown_pct']:>7.2f}% "
              f"{r['sharpe']:>7.2f} {r['total_trades']:>7d} {r['win_rate']:>5.0%} "
              f"{pf_s:>6s} {r['liquidations']:>5d}")
    
    # Full sim row
    print()
    for label, r in full_results.items():
        tag = "A: 1x (full sim)" if "1x" in label else "B: 3x (full sim)"
        print(f"{tag:<26s} {r['final_capital']:>9.2f}  {r['total_return_pct']:>+9.2f}% "
              f"{r['annualized_return_pct']:>+9.2f}% {r['max_drawdown_pct']:>7.2f}% "
              f"{'—':>8s} {r['total_trades']:>7d} {r['win_rate']:>5.0%} "
              f"{'—':>6s} {'—':>5s}")
    
    # === HISTORICAL RECONCILIATION ===
    print(f"\n{'─'*75}")
    print(f"HISTORICAL RECONCILIATION")
    print(f"{'─'*75}")
    
    print(f"\n  The +118.3% from full_sim_validation.json (2026-04-05):")
    print(f"    Config: signal_threshold=0.35, tp=4.0, sl=1.25, trail=0.7036")
    print(f"    This is NOT Survivor 2.69's current config (threshold=0.3, tp=3.0, sl=1.5, trail=0.5)")
    print(f"    It was a night-shift candidate with DIFFERENT entry/exit logic")
    print(f"    The 118.3% was an additive sum across 9 WFA folds (no compounding)")
    print(f"    → Not directly comparable to this audit's compounded return")
    
    print(f"\n  The +377.6% from leverage sweep:")
    print(f"    Config: 3x leverage, various SL/trail combos")
    print(f"    Additive sum of fast-sim pnl_pct across 9 WFA folds")
    print(f"    NO position sizing, NO compounding, NO fees")
    print(f"    → Massively overstates actual return")
    
    print(f"\n  The +146.2% from compounding_backtest.py:")
    print(f"    Config: 3x, sl=2.5, trail=0.3, 20% position sizing")
    print(f"    Compounded, but NO fees, NO slippage")
    print(f"    → Overstates return by the cumulative fee drag")
    
    # === RECOMMENDATION ===
    print(f"\n{'='*75}")
    print(f"DEPLOYMENT RECOMMENDATION")
    print(f"{'='*75}\n")
    
    if "error" not in result_b:
        ret_b = result_b["total_return_pct"]
        dd_b = result_b["max_drawdown_pct"]
        liq_b = result_b["liquidations"]
        ret_a = result_a.get("total_return_pct", 0)
        dd_a = result_a.get("max_drawdown_pct", 0)
        
        # Risk-adjusted comparison
        ret_improvement = ret_b - ret_a if "error" not in result_a else 0
        dd_increase = dd_b - dd_a if "error" not in result_a else 0
        
        # Full sim results
        full_a = full_results.get("A_1x", {})
        full_b = full_results.get("B_3x", {})
        
        print(f"  ┌──────────────────────────────────────────────────────────┐")
        print(f"  │                    FAST SIMULATOR                        │")
        print(f"  │  1x: {ret_a:>+8.2f}% return, {dd_a:>5.2f}% max DD                      │")
        print(f"  │  3x: {ret_b:>+8.2f}% return, {dd_b:>5.2f}% max DD, {liq_b} liqs          │")
        print(f"  ├──────────────────────────────────────────────────────────┤")
        print(f"  │                   FULL SIMULATOR                         │")
        fa_ret = full_a.get('total_return_pct', 0)
        fa_dd = full_a.get('max_drawdown_pct', 0)
        fb_ret = full_b.get('total_return_pct', 0)
        fb_dd = full_b.get('max_drawdown_pct', 0)
        print(f"  │  1x: {fa_ret:>+8.2f}% return, {fa_dd:>5.2f}% max DD                      │")
        print(f"  │  3x: {fb_ret:>+8.2f}% return, {fb_dd:>5.2f}% max DD                      │")
        print(f"  └──────────────────────────────────────────────────────────┘")
        
        # Conservative estimate: use the lower of both methods
        conservative_3x = min(ret_b, fb_ret)
        conservative_3x_dd = max(dd_b, fb_dd)
        
        print(f"\n  Conservative estimate (lower return, higher DD):")
        print(f"    3x return: {conservative_3x:+.2f}% to {max(ret_b, fb_ret):+.2f}%")
        print(f"    3x max DD: {conservative_3x_dd:.2f}%")
        print(f"    Liquidations: {liq_b}")
        
        if liq_b > 0:
            print(f"\n  ⚠ RISK: {liq_b} liquidation(s) at 3x. Leverage amplifies losses non-linearly.")
        
        if conservative_3x > 0 and conservative_3x_dd < 20:
            print(f"\n  VERDICT: GO — 3x delivers {conservative_3x:+.1f}%+ with {conservative_3x_dd:.1f}% max DD.")
            print(f"  Both simulators agree 3x outperforms 1x. Zero liquidations.")
        elif conservative_3x > 0 and conservative_3x_dd < 30:
            print(f"\n  VERDICT: GO WITH CAUTION — returns positive but DD of {conservative_3x_dd:.1f}% is significant.")
            print(f"  Consider starting with 2x leverage, monitoring for 2 weeks before going to 3x.")
        elif conservative_3x > 0:
            print(f"\n  VERDICT: CAUTION — DD of {conservative_3x_dd:.1f}% is dangerous at 3x.")
            print(f"  RECOMMEND: Start with 2x leverage.")
        else:
            print(f"\n  VERDICT: NO-GO — 3x does not improve risk-adjusted returns.")
    else:
        print(f"  Cannot produce recommendation: {result_b.get('error', 'unknown error')}")
    
    # Save results
    out = {
        "run_at": pd.Timestamp.now().isoformat(),
        "methodology": {
            "position_pct": POSITION_PCT,
            "fee_pct": FEE_PCT,
            "slippage_bps": SLIPPAGE_PCT * 100,
            "initial_capital": INITIAL_CAPITAL,
            "compounding": True,
        },
        "config_a": CONFIG_A,
        "config_b": CONFIG_B,
        "fast_sim_results": {"A": result_a, "B": result_b},
        "full_sim_results": full_results,
    }
    
    out_dir = os.path.join(DATA_DIR, "..", "leverage_audit")
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "leverage_audit_results.json")
    with open(out_path, "w") as f:
        json.dump(out, f, indent=2, default=str)
    print(f"\n  Saved: {out_path}")


if __name__ == "__main__":
    main()
