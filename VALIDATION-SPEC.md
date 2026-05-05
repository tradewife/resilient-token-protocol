# VALIDATION-SPEC.md — Leverage Audit & Fresh Session Handoff

> **Status:** SUPERSEDED. The 3x leverage audit was completed and the approach was extended to a full 3x-10x leverage optimization. The current deployed config is 9x leverage with Calmar=44.89. See `data/night_results/2026-05-05/leverage_report.md` for the latest results. This file is retained for historical reference.

> **Purpose:** This file hands off the leverage validation task to a fresh agent session. The previous session produced conflicting PnL numbers and needs a clean audit before deploying 3x leverage to production.

---

## Project Onboard

You are working on **RTP (Resilient Token Protocol)** — a Solana treasury protocol with an autonomous trading system. This task is purely in the **Python research layer**. You do NOT need to touch Rust, Solana, or the Anchor program.

### What RTP Does (context only)

Token projects route trading fees to a treasury PDA. An autonomous Rust trader (`rtp-trader`) executes a validated strategy called **Survivor 2.69** on Flash Trade (on-chain Solana perps). The Python research layer finds and validates profitable strategies before they go live.

### The Live Strategy

- **Name**: Survivor 2.69 (SOL/USDT)
- **Signal**: Multi-timeframe confluence (1h/4h/1d SMA trends + RSI + Bollinger + momentum)
- **Current production params**: `signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36h, trailing_stop=0.5`
- **Runs on**: Flash Trade REST API, 0.20 SOL per trade, currently 1x leverage

### What The Previous Session Did

1. Added leverage modeling to the fast simulator
2. Ran a leverage grid sweep finding 3x leverage with wider stops (sl_atr=2.5, trail=0.3) as optimal
3. Updated the Rust trader defaults to 3x leverage + sl_atr=2.5 + trail=0.3
4. Ran autoresearch, full-sim validation, and sensitivity sweep — all passed

### The Problem

**Three conflicting PnL numbers** were reported and the previous session confused them. The 3x leverage change was committed to the Rust trader BUT the numbers need a clean audit before deploying live.

| Reported PnL | Source | What It Actually Is |
|---|---|---|
| +118.3% | `data/night_results/2026-04-05/full_sim_validation.json` | Full simulator at 1x, 20% sizing, WITH fees+slippage. **OLD config** (sl=1.0, trail=0.7) — NOT Survivor 2.69's current params. |
| +377.6% | `data/leverage_sweep/leverage_sweep_SOL_USDT_report.json` | Additive sum of `pnl_pct` across 9 WFA folds at 3x. NO position sizing (100% per trade), NO compounding, NO fees. Fast simulator. |
| +146.2% | `research/simulation/compounding_backtest.py` output | Compounded at 3x, 20% sizing, NO fees. Fast simulator on full 365-day data. |

**These use different: position sizing, compounding methods, cost models, strategy params, and data splits. None are directly comparable.**

---

## Your Task

### Goal

Produce **one authoritative comparison table** of expected returns at 1x vs 3x leverage using a **single consistent methodology**: the full simulator (`FutureBlindSimulator`) with fees (0.1%), slippage (10bps), and 20% position sizing, on the same data.

### Step 1: Understand The Measurement Systems

Read these files to understand how PnL is computed in each:

| File | Purpose |
|---|---|
| `research/optimization/per_symbol_optimizer.py` | **Fast simulator** — `simulate_trades()` returns raw `pnl_pct` per trade. `compute_metrics()` does `total_pnl_pct = sum(pnls)` (additive, no compounding, no position sizing). Leverage multiplies `pnl_pct` by `leverage` param. |
| `research/simulation/future_blind_simulator.py` | **Full simulator** — `FutureBlindSimulator` with fees, slippage, and `_calculate_position_size()` which caps at 20% of capital. This IS the realistic simulator. |
| `research/simulation/run_backtest_r2.py` | `MultiTFStrategy` — the full-sim strategy class. Has `_compute_score()` and `analyze()` with full entry/exit logic. |
| `research/simulation/compounding_backtest.py` | Simple compounding walk — multiplies capital by `(1 + position_pct * pnl_pct/100)` per trade. Uses fast sim trades, no fees. |
| `research/simulation/leverage_sweep.py` | Grid sweep — calls `evaluate_on_fold()` from night_shift.py, sums `oos_pnl` across folds. This is where the 377.6% came from. |
| `research/validation/validate_night_shift.py` | Full-sim validation — runs `FutureBlindSimulator` per fold, compounds via `final_balance = 10000 * (1 + sum(pnls)/100)`. Has a `validate_leveraged()` function that applies leverage as post-hoc PnL multiplier. |

### Step 2: Run The Full Simulator With Identical Methodology

You need to run `FutureBlindSimulator` (with fees + slippage) on the **full 365-day data** (not split into folds) for both configs, and compare final compounded capital.

**Config A (current production):**

```python
{
    "signal_threshold": 0.3, "min_alignment": 3, "take_profit_atr": 3.0,
    "stop_loss_atr": 1.5, "max_hold_hours": 36, "time_decay_hours": 12,
    "trailing_stop_atr": 0.5, "score_flip_delay_hrs": 0,
    # leverage: 1x (no multiplier in full sim — it trades unleveraged)
}
```

**Config B (proposed 3x):**

```python
{
    "signal_threshold": 0.3, "min_alignment": 3, "take_profit_atr": 3.0,
    "stop_loss_atr": 2.5, "max_hold_hours": 36, "time_decay_hours": 12,
    "trailing_stop_atr": 0.3, "score_flip_delay_hrs": 0,
    # leverage: 3x (apply 3x multiplier to each trade's PnL post-hoc)
}
```

**Key data file:** `data/ohlcv/SOL_USDT_1h.parquet` — 8,760 hourly candles (Apr 2025 - Apr 2026)

**Key challenge:** `FutureBlindSimulator` doesn't natively model leverage. The existing approach in `validate_night_shift.py::validate_leveraged()` applies it as a post-hoc multiplier on round-trip PnLs. Verify this is correct (it should be — leverage is a linear P&L amplifier, not a signal change).

### Step 3: Also Run Fast-Sim Compounding With Fees

For a secondary check, modify or run the fast simulator with a fee-adjusted PnL per trade (`pnl_pct - 0.1% fee on entry + exit`) and compound at 20% position sizing. This should roughly match the full simulator.

### Step 4: Produce The Authoritative Table

Expected output format:

```
SOL/USDT Compounded Annual Return (20% position sizing, 0.1% fees, 10bps slippage)
365-day continuous backtest (Apr 2025 -> Apr 2026)

Config                    Final Capital   Total Return   Annualized   Max DD   Sharpe   Trades
------------------------------------------------------------------------------------------
A: 1x (sl=1.5, tr=0.5)   xxx SOL         +xx.x%         +xx.x%       x.x%     x.xx     xxx
B: 3x (sl=2.5, tr=0.3)   xxx SOL         +xx.x%         +xx.x%       x.x%     x.xx     xxx
```

### Step 5: Verify Against Historical Data

The +118.3% from `data/night_results/2026-04-05/full_sim_validation.json` was for an OLD config (sl=1.0, trail=0.7). Read that file to confirm what params produced it. If your Config A run at 1x gives a different number, explain why (different params, different data period, etc.).

### Step 6: Recommend Whether To Deploy 3x

Based on the clean numbers, give a clear go/no-go recommendation with the actual risk metrics.

---

## Key Files Map

```
research/
  optimization/
    per_symbol_optimizer.py    # Fast simulator -- simulate_trades(), compute_metrics()
  simulation/
    future_blind_simulator.py  # Full simulator -- fees, slippage, position sizing
    run_backtest_r2.py         # MultiTFStrategy -- full-sim strategy class
    compounding_backtest.py    # Simple compounding walk (created this session)
    leverage_sweep.py          # Leverage grid sweep (created this session)
    sensitivity_sweep.py       # Param sensitivity sweep
  validation/
    validate_night_shift.py    # Full-sim validation + validate_leveraged()
    validate_optimized.py      # Optimized config validation
  orchestration/
    night_shift.py             # Main pipeline -- grid search, WFA, Darwinian
    night_config.json           # Pipeline config (includes leverage experiments now)
    autoresearch.py            # Self-improving loop
  strategy_library.md          # 15 strategy cards
  dead_ends.md                 # Failure memory log

rtp/swarm/src/trader/
  mod.rs                       # TraderConfig -- default leverage changed to 3.0
  strategy.rs                  # StrategyParams -- defaults changed (sl=2.5, trail=0.3)
  executor.rs                  # Flash Trade REST API executor
  indicators.rs                # Technical indicator computations
  candles.rs                   # Candle buffer for warmup

data/
  ohlcv/SOL_USDT_1h.parquet    # 8,760 hourly candles
  leverage_sweep/               # Sweep results from this session
    leverage_sweep_SOL_USDT.csv
    leverage_sweep_SOL_USDT_report.json  # Where 377.6% came from
  night_results/2026-04-05/
    full_sim_validation.json   # Where 118.3% came from (OLD config)
  sensitivity_sol_survivor_2_69_lev3.csv  # Sensitivity sweep results
  autoresearch_state.json      # Current autoresearch state (3x SOL)
  autoresearch_log.json        # 50 experiment log from this session
```

---

## Important Caveats

- **The Rust trader defaults were already changed** (3x leverage, sl=2.5, trail=0.3) but the live Railway service has NOT been redeployed yet. It still runs the old 1x config.
- **Do NOT modify any Rust files** -- this is purely a Python research validation task.
- **Python environment**: use `source .venv/bin/activate` before running any Python.
- The fast simulator's `simulate_trades()` now has a `leverage` param that multiplies `pnl_pct` -- this was added this session and works correctly (tested: 3x gives exactly 3x PnL amplification).
- 325 Rust tests pass with the updated strategy defaults.
