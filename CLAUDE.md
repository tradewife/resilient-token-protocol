# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

**RTP (Resilient Token Protocol)** — a Solana-native, self-funding treasury governed by a modular Rust swarm. Six specialized wings autonomously generate yield, defend against threats, evolve the protocol's own architecture, audit for compliance, accumulate knowledge, and monitor existential risks — all funded by their own yield.

**Hackathon**: Solana Frontier (Colosseum × Canteen), $300k prizes, deadline May 11, 2026.
**License**: MIT

This repo has two layers:
1. **Proven Python yield brain** (shipping) — backtesting, optimization, paper trading
2. **Rust swarm + Solana treasury** (in development) — 6-wing architecture, Coordinator, soulcontract

## Quick Setup

```bash
# Python environment (yield brain)
python -m venv .venv && source .venv/bin/activate
pip install pandas numpy ccxt pyarrow redis

# Night shift (30K configs/night, 9-fold WFA)
python scripts/night_shift.py --skip-fetch

# Paper trading (live Binance)
PYTHONUNBUFFERED=1 python scripts/paper_trader.py

# Full-sim validation
python scripts/validate_night_shift.py --production

# Self-correction
python scripts/evaluator_calibration.py --samples 20
python scripts/discrepancy_detector.py

# Rust (swarm runtime)
cd rtp/swarm && cargo build
cd rtp/programs/rtp-treasury && anchor build
```

## Commands

### Python (Yield Brain)

```bash
# Night shift — main optimization pipeline
python scripts/night_shift.py                      # defaults (4 symbols, 9 folds)
python scripts/night_shift.py --skip-fetch         # use cached data
python scripts/night_shift.py --symbols SOL/USDT   # single symbol

# Paper trading — live market validation
python scripts/paper_trader.py

# Validation — bridge fast sim → full sim
python scripts/validate_night_shift.py --production
python scripts/validate_night_shift.py --symbol SOL/USDT --top 3

# Self-correction — calibration + discrepancy detection
python scripts/evaluator_calibration.py --samples 20
python scripts/evaluator_calibration.py --fast-only
python scripts/discrepancy_detector.py

# Data
python scripts/download_ohlcv.py                   # fetch from Binance (no API key needed)
```

### Rust (Swarm Runtime)

```bash
# Build swarm
cd rtp/swarm && cargo build --release

# Test individual wings
cd rtp/swarm && cargo test --lib trading::tests
cd rtp/swarm && cargo test --lib audit::tests

# Integration tests
cd rtp/swarm && cargo test --test coordinator_integration
```

### Solana (Treasury Program)

```bash
# Build Anchor program
cd rtp/programs/rtp-treasury && anchor build

# Run on devnet
cd rtp/programs/rtp-treasury && anchor test --provider.cluster devnet

# Deploy
cd rtp/programs/rtp-treasury && anchor deploy --provider.cluster devnet
```

## Architecture

### Three-Layer Stack

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│  Treasury PDA: fees → yield → redistribute → self-hydrate       │
│  Phase evolution: Sustenance → Ecosystem → Humanity Fund        │
├─────────────────────────────────────────────────────────────────┤
│                    SWARM RUNTIME (Rust)                          │
│  Coordinator → message bus → 6 wings (trading, security,        │
│  evolve, knowledge, audit, futureproof)                          │
├─────────────────────────────────────────────────────────────────┤
│                    RESEARCH LAYER (Python)                        │
│  Night Shift: 30K configs → WFA → Darwinian → full-sim validate │
│  Paper Trader: live Binance → state persistence → degradation   │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow (Yield Brain)

```
Binance → download_ohlcv.py → data/ohlcv/{SYMBOL}_1h.parquet
                                   ↓
              per_symbol_optimizer (compute_indicators, simulate_trades, _compute_score)
              ┌────────────────┴────────────────┐
              │                                 │
         night_shift (grid search)         paper_trader (live)
         fast sim (~30K combos)            real-time Binance
              │                                 │
              ▼                                 ▼
    ┌─────────────────┐                 data/paper_trading/
    │ validate_       │                   state.json
    │ night_shift.py  │
    │ (full sim bridge)│
    └────────┬────────┘
             │
             ▼
    FutureBlindSimulator (fees + slippage)
             │
             ▼
    data/night_results/YYYY-MM-DD/report.md
```

### Swarm Message Flow

```
Trading Wing          Coordinator           Audit Wing
     │                    │                     │
     │  Proposal:         │  Audit Request:     │
     │  {deploy_config,   │  {check: proposal,  │
     │   params,          │   against:soul}     │
     │   confidence:0.9}  │ ───────────────────►│
     │ ─────────────────►│                     │
     │                    │  Audit Response:    │
     │                    │  {approved: true,   │
     │                    │   risk: LOW}        │
     │                    │◄───────────────────│
     │  Execute:          │                     │
     │  {config_applied,  │                     │
     │   monitor: 24h}    │                     │
     │◄───────────────────│                     │
```

### Key Files

#### Python (Yield Brain)

| File | Purpose |
|------|---------|
| `scripts/night_shift.py` | Main pipeline: grid search → WFA → Darwinian → report → validation |
| `scripts/per_symbol_optimizer.py` | Fast simulator: `compute_indicators()`, `simulate_trades()`, `_compute_score()` |
| `scripts/paper_trader.py` | Live paper trader: polls Binance, ADX filter, per-symbol configs |
| `scripts/validate_night_shift.py` | Bridges fast sim → full sim for candidate validation |
| `scripts/run_backtest_r2.py` | Production `MultiTFStrategy` class + `timeframe_signal()` helper |
| `scripts/evaluator_calibration.py` | Compares fast vs full sim on random configs |
| `scripts/discrepancy_detector.py` | Post-night-shift check, flags fast/full sim divergences |
| `scripts/night_config.json` | Night shift config (symbols, folds, experiments, thresholds) |
| `backtesting/future_blind_simulator.py` | `FutureBlindSimulator`: 0.1% fees, 10bps slippage, max 20% position |
| `agents/historical_data_collector.py` | `DataWindow` class feeding data to full simulator |

#### Rust (Swarm Runtime)

| File | Purpose |
|------|---------|
| `rtp/swarm/src/coordinator/router.rs` | Typed message routing between wings |
| `rtp/swarm/src/coordinator/soulguard.rs` | Enforce soulcontract on every message |
| `rtp/swarm/src/coordinator/lifecycle.rs` | Wing spawn, health-check, retire |
| `rtp/swarm/src/wings/trading/executor.rs` | Hyperliquid + Jupiter + Solana CPI |
| `rtp/swarm/src/wings/trading/bridge.rs` | Python ↔ Rust typed interface |
| `rtp/swarm/src/wings/security/` | Vulnerability scanning, threat intel, responder |
| `rtp/swarm/src/wings/evolve/` | Assessor, proposer, rollback |
| `rtp/swarm/src/wings/knowledge/` | Knowledge graph, ingest, recall |
| `rtp/swarm/src/wings/audit/` | Intent compliance, safety, audit log |
| `rtp/swarm/src/wings/futureproof/` | Quantum, deprecation, horizon scanning |

#### Solana (Treasury Program)

| File | Purpose |
|------|---------|
| `rtp/programs/rtp-treasury/` | Anchor: deposit_usdc, check_redistribute, hydrate_swarm, evolve_phase |

#### Governance

| File | Purpose |
|------|---------|
| `soulcontract.md` | Constitutional governance layer — invariants, what can/cannot evolve |
| `BUILD_PLAN.md` | Full 10-part build plan v2.1 (hackathon timeline, skill mapping, links) |
| `third-party-disclosure.md` | MIT framework + sponsor attributions |
| `docs/demo-flow.md` | 3-minute hackathon demo script |

## Key Invariants (enforced on-chain)

1. **PDA owns treasury** — no private key risk
2. **SPL TransferFeeConfig immutable from mint** — fees cannot be revoked
3. **CPI-only transfers** — atomic, verifiable
4. **Agent proposes, human approves irreversible actions**
5. **No SOL liquidation** — USDC-only yield flows
6. **Phase transitions irreversible** — Sustenance → Ecosystem → Humanity
7. **soulcontract amendments require human signature + 24h monitoring**
8. **Auto-rollback if performance degrades > 5% post-amendment**
9. **Self-hydration only if sustenance bucket > 90-day runway**
10. **Yield brain strategies remain black-boxed** (competitive moat)

## Black-Box / Open-Source Split

**Open (MIT, judges see full source)**:
- Swarm architecture (Coordinator, wings, message bus)
- Treasury program (Anchor)
- soulcontract.md
- Demo flow + disclosure

**Black-boxed (proprietary binary)**:
- `night_shift.bin` — PyInstaller binary of yield brain
- `configs/encrypted/` — AES-encrypted strategy params
- `loss_function.bin` — Treasury-native scoring
- Research pipeline internals

## Critical: Fast Sim Calibration

The fast simulator (`per_symbol_optimizer`) MUST match the full simulator exactly. Three invariants discovered the hard way:

1. **ATR formula**: `std(returns, 20h) × price` — NOT True Range
2. **MR entry condition**: `rsi < 35 and daily_trend == bullish` — NOT `bull_count >= min_alignment`
3. **Sharpe annualization**: `sqrt(n_trades / total_hours × 8760)` — NOT `sqrt(24 × 365)`

If you change anything in `_compute_score()` or `simulate_trades()`, run `evaluator_calibration.py` to verify directional agreement.

## Night Shift Pipeline Phases

1. **Phase 1: Data** — load cached parquet (Binance geo-blocked on GitHub, data in repo)
2. **Phase 2: WFA Folds** — expanding-window, non-overlapping, 9 folds × 36-day test windows
3. **Phase 2b: Production Baseline** — evaluate current config as reference
4. **Phase 3: Coarse Grid** — ~30K parameter combinations per symbol
5. **Phase 3b: Fine Refinement** — top 100 per symbol on all folds
6. **Phase 4: Darwinian Evolution** — 5 generations, mutate best candidates
7. **Phase 4b: BB Mean Reversion** — separate strategy grid search
8. **Phase 4c: Custom Experiments** — configurable param sweeps from `night_config.json`
9. **Phase 5: Regime Analysis** — ADX, volatility percentile, correlations
10. **Phase 6: Morning Report** — markdown + JSON report with top candidates
11. **Phase 7: Auto-Validation** — top 3 through full FutureBlindSimulator
12. **Phase 8: Discrepancy Detection** — compare fast/full sim, flag divergences

## Self-Correction Architecture

Three independent modules (no LLM needed):

1. **`evaluator_calibration.py`** — N random configs, measures sign agreement and PnL correlation
2. **`discrepancy_detector.py`** — tracks consecutive flags per symbol, skips Darwinian after 2 bad nights
3. **Phase 8 in `night_shift.py`** — calls discrepancy detector automatically

## Validation Pipeline

1. **Night shift fast sim** — coarse grid (~30K combos) → fine refinement → Darwinian
2. **Three-layer overfitting detection** — IS-OOS gap, OOS consistency, parameter fragility
3. **Full-sim validation** — top candidates through FutureBlindSimulator (fees + slippage)
4. **Discrepancy detection** — compare fast/full sim rankings, flag divergent symbols
5. **Paper trading** — live market validation with real Binance data

## Yield Brain Results

| Symbol | Production PnL | Optimized PnL | Consistency | Trades |
|--------|---------------|--------------|-------------|--------|
| SOL/USDT | +36.9% | **+118.3%** | 78% | 429 |
| BNB/USDT | +49.6% | — | 67% | 178 |
| ETH/USDT | +48.1% | — | 78% | 155 |
| BTC/USDT | +17.5% | — | 67% | 153 |

Active symbols: BTC/USDT, ETH/USDT, SOL/USDT, BNB/USDT. XRP was dropped (net negative across all WFA folds).

## Fee Flow

```
pump.fun trade (0.25% fee) → 0.05% creator fee (SOL) → RTP Treasury PDA
                                                              │
                                                              ├─ 90% → Yield strategies (USDC via Jupiter)
                                                              └─ 10% → Ecosystem SOL reserves (compounds)
```

Redistribution at threshold: 70% holders / 20% dev / 10% ecosystem.

## Phased Evolution

| Phase | Threshold | Behavior |
|-------|-----------|----------|
| **1: Sustenance** | < $50k | Self-hydrate, reinvest all yield |
| **2: Ecosystem** | $50k–$1M | Auto-provide LP to top RTP-adopting tokens |
| **3: Humanity** | > $1M | USDC grants to Solana public-goods projects |

Phase transitions are **irreversible** on-chain.

## cldcde Skills (installed)

27 relevant skills mapped to RTP wings (see BUILD_PLAN.md Part 4 for full mapping):

- **Tier 1 (Critical)**: swarm-orchestration, hive-mind-advanced, spec-lock, red-team-tribunal, compound-engineering, verification-quality
- **Tier 2 (High)**: agentdb-memory-patterns, agentdb-advanced, reasoningbank-agentdb, agentdb-learning, sparc-methodology, ultra-planner, debt-sentinel, swarm-advanced, stream-chain, fpef-analyzer
- **Tier 3 (Medium)**: hooks-automation, performance-analysis, github-workflow-automation, github-release-management, ae-ltd-skill-builder, flow-nexus-swarm, mcp-universal-manager
- **Tier 4 (Low)**: prologue, ae-proof-agent, agentic-jujutsu, skill-builder, multi-platform-architect

Installed at `~/.claude/skills/` and `~/.freecode/plugins/`.

## Sponsored Hackathon Resources

| Sponsor | Use | Link |
|---------|-----|------|
| Phantom Connect | Agentic wallet + CASH stablecoin | https://docs.phantom.app/phantom-connect/introduction |
| Squads Multisig | Treasury PDA security | https://docs.squads.so |
| Swig | Programmable smart wallets | https://docs.swig.fi |
| MoonPay Agents | Agent money movement | https://www.moonpay.com/developers/agents |
| Solana MCP | AI dev assistant for Anchor | https://github.com/solana-developers/solana-mcp |
| Arcium | Encrypted computation (stretch) | https://docs.arcium.com |

Not using: World Coin (toxic sentiment).

## Design Decisions

- **Median OOS Sharpe** (not mean) — prevents single-fold outliers from dominating
- **Per-fold Sharpe winsorized at ±100** — prevents tiny-sample Sharpe from going to ±8000+
- **Fragility is a penalty, not rejection** — `survivor *= 1/(1+fragility)`
- **Survivor Score**: `avg_oos_sharpe × consistency × (1-overfitting) × dd_factor × trade_factor × fragility_penalty`
- **Wings never modify each other directly** — all cross-wing communication via Coordinator
- **Python ↔ Rust interface** is typed JSON — any wing can propose, any wing can act
- **Paper trader has no Redis dependency** for runtime (only full sim validation path needs it)
- **Virtual environment**: `.venv/` with Python 3.13.3, ccxt 4.5.46, pandas 3.0.2

## CI/CD

- **Night shift**: GitHub Actions cron at 14:00 UTC (midnight AEST)
- **Binance is geo-blocked on GitHub runners** — OHLCV data in `data/ohlcv/`, `fetch_fresh_data` defaults to `false`
- **Data refresh**: run `download_ohlcv.py` locally, commit updated parquets, push before midnight
- **Workflow**: `.github/workflows/night_shift.yml` (300 min timeout, Python 3.12)
- **Dependencies in CI**: `pandas numpy ccxt pyarrow redis`

## GitHub

- **This repo**: `git@github.com:tradewife/fractal-swarm.git` (SSH)
- **PAT stored**: `~/.config/gh/config.yml` (for `workflow_dispatch` triggers)
- **Separate hackathon repo**: `tradewife/resilient-token-protocol` (clean submission, open-source skeleton only)

## Hackathon Submission Structure

The `resilient-token-protocol` repo contains only the open-source skeleton:
- `rtp/swarm/` — full Rust swarm source
- `rtp/programs/rtp-treasury/` — full Anchor program
- `soulcontract.md`, `BUILD_PLAN.md`, `third-party-disclosure.md`
- Binary placeholders for yield brain (not source)

The `fractal-swarm` repo (this repo) retains the full Python yield brain source, data, and CI pipeline.
