# CLAUDE.md

Guidance for Claude Code when working with this repository.

## Project Overview

**RTP (Resilient Token Protocol)** — a post-governance, commitment-enforced token longevity layer for Solana. Turns "don't rug" from a social promise into a cryptographically enforced, agent-operated system. Any token project adopts RTP and their token structurally cannot rug.

**Core thesis**: `RTP token price = SOL macro + narrative` (no founder risk, no rug risk premium).

**Hackathon**: Solana Frontier (Colosseum × Canteen), $300k prizes, deadline May 11, 2026.
**License**: MIT
**Category**: Unruggable launch standard · Trust-minimized token primitive

This repo has two layers:
1. **On-chain treasury program** (Anchor) — fee routing, price floor, buybacks, hedging, redistribution
2. **Agent swarm** (Rust) — allocator, executor, verifier operating the treasury autonomously
3. **Yield brain** (Python, gitignored) — proven strategy research engine feeding the swarm

## The Product (Not the Swarm)

RTP is a **token standard**, not a trading bot. The swarm is the enforcement layer. The product promise is:

1. **Fee routing** — immutable TransferFeeConfig auto-routes fees to Treasury PDA
2. **Price floor** — TWAP-enforced, circuit-breaker-protected buyback trigger
3. **Correlated hedging** — SOL-short via Drift, structurally reliable because RTP tokens have higher SOL correlation
4. **Circuit breakers** — cooldown, epoch cap, velocity limit prevent treasury drain
5. **Verification** — every agent action published on-chain, provable, auditable
6. **Redistribution** — above threshold: 70% holders / 20% dev / 10% ecosystem

## Quick Setup

```bash
# Python environment (yield brain — gitignored, local dev only)
python -m venv .venv && source .venv/bin/activate
pip install pandas numpy ccxt pyarrow redis

# Night shift (30K configs/night, 9-fold WFA)
python scripts/night_shift.py --skip-fetch

# Paper trading (live Binance)
PYTHONUNBUFFERED=1 python scripts/paper_trader.py

# Rust (agent swarm)
cd rtp/swarm && cargo build

# Solana (treasury program)
cd rtp/programs/rtp-treasury && anchor build
cd rtp/programs/rtp-treasury && anchor test --provider.cluster devnet
```

## Commands

### Python (Yield Brain — gitignored, local only)

```bash
python scripts/night_shift.py --skip-fetch
python scripts/paper_trader.py
python scripts/validate_night_shift.py --production
python scripts/evaluator_calibration.py --samples 20
python scripts/discrepancy_detector.py
python scripts/download_ohlcv.py
```

### Rust (Agent Swarm)

```bash
cd rtp/swarm && cargo build --release
cd rtp/swarm && cargo test --lib agents::tests
cd rtp/swarm && cargo test --test coordinator_integration
```

### Solana (Treasury Program)

```bash
cd rtp/programs/rtp-treasury && anchor build
cd rtp/programs/rtp-treasury && anchor test --provider.cluster devnet
cd rtp/programs/rtp-treasury && anchor deploy --provider.cluster devnet
```

## Architecture

### Three Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│  Treasury PDA: fees → floor → buyback → hedge → redistribute   │
│  Invariants: PDA ownership, TransferFeeConfig immutability,     │
│  circuit breakers, on-chain verification                        │
├─────────────────────────────────────────────────────────────────┤
│                    AGENT SWARM (Rust)                            │
│  Allocator → Executor → Verifier                                │
│  Typed message bus, soulcontract enforcement, skill system      │
├─────────────────────────────────────────────────────────────────┤
│                    RESEARCH LAYER (Python — gitignored)           │
│  Night Shift: 30K configs → WFA → Darwinian → full-sim validate │
│  Paper Trader: live Binance → state persistence → degradation   │
└─────────────────────────────────────────────────────────────────┘
```

### Core Primitives

| Primitive | Mechanism | On-Chain |
|-----------|-----------|----------|
| Fee Routing | TransferFeeConfig → Treasury PDA | ✅ |
| Price Floor | treasury_usd / circulating_supply, Pyth TWAP | ✅ |
| Buybacks | Jupiter CPI when price < floor × discount | ✅ |
| Hedging | Drift Protocol SOL-short perps | ✅ |
| Yield | Kamino/Marginfi idle capital deployment | ✅ |
| Circuit Breakers | Cooldown + epoch cap + velocity limit | ✅ |
| Redistribution | 70/20/10 split above threshold | ✅ |
| Verification | Verifier agent publishes proof | ✅ |

### Key Files

#### Solana (Treasury Program)

| File | Purpose |
|------|---------|
| `rtp/programs/rtp-treasury/` | Anchor: deposit_usdc, check_floor, execute_buyback, hedge, redistribute, evolve_phase |

#### Rust (Agent Swarm)

| File | Purpose |
|------|---------|
| `rtp/swarm/src/coordinator/router.rs` | Typed message routing between agents |
| `rtp/swarm/src/coordinator/soulguard.rs` | Enforce soulcontract on every message |
| `rtp/swarm/src/coordinator/lifecycle.rs` | Agent spawn, health-check, retire |
| `rtp/swarm/src/agents/allocator.rs` | Inflow routing per immutable rules |
| `rtp/swarm/src/agents/executor.rs` | Jupiter swaps, Drift hedging, Kamino yield |
| `rtp/swarm/src/agents/verifier.rs` | On-chain proof publication |
| `rtp/swarm/src/skills/` | Atomic skill definitions (trigger→action→proof) |

#### Python (Yield Brain — gitignored)

| File | Purpose |
|------|---------|
| `scripts/night_shift.py` | 30K configs/night, 9-fold WFA, Darwinian evolution |
| `scripts/per_symbol_optimizer.py` | Fast simulator |
| `scripts/paper_trader.py` | Live paper trader |
| `scripts/validate_night_shift.py` | Fast sim → full sim bridge |
| `backtesting/future_blind_simulator.py` | 0.1% fees, 10bps slippage, ground truth |

#### Governance

| File | Purpose |
|------|---------|
| `soulcontract.md` | Constitutional governance — invariants, what can/cannot evolve |
| `BUILD_PLAN.md` | Full build plan v3.0 |
| `third-party-disclosure.md` | MIT framework + sponsor attributions |
| `docs/demo-flow.md` | 3-minute hackathon demo |

## Key Invariants (enforced on-chain)

1. **PDA owns treasury** — no private key risk
2. **TransferFeeConfig immutable from mint** — fees cannot be revoked post-adoption
3. **Circuit breakers prevent drain** — cooldown, epoch cap, velocity limit
4. **Price floor enforced by TWAP oracle** — not a suggestion, a CPI-enforced trigger
5. **No SOL liquidation** — USDC-only flows for risk management
6. **Every action verified** — Verifier publishes on-chain proof
7. **Phase transitions irreversible** — Sustenance → Ecosystem → Humanity
8. **soulcontract amendments require human signature + 24h monitoring**
9. **Auto-rollback if performance degrades > 5% post-amendment**

## Three Flywheels

```
Fee Revenue → Hedge Yield → Yield/Arbitrage → compounds reserves → more buyback pressure
     ▲                                                                   │
     └───────────────────────────────────────────────────────────────────┘
```

RTP tokens have structurally higher SOL correlation (no founder/rug noise) → correlated hedges are more reliable → self-reinforcing property.

## Fee Flow

```
Token trade → TransferFeeConfig fee → Treasury PDA
                                        │
                                        ├─ Floor defense (buybacks via Jupiter)
                                        ├─ Hedging (SOL-short via Drift)
                                        ├─ Yield (idle capital → Kamino/Marginfi)
                                        └─ Redistribution (70% holders / 20% dev / 10% ecosystem)
```

## Phased Evolution

| Phase | Threshold | Behavior |
|-------|-----------|----------|
| **1: Sustenance** | < $50k | Self-hydrate, reinvest all yield |
| **2: Ecosystem** | $50k–$1M | Auto-provide LP to top RTP-adopting tokens |
| **3: Humanity** | > $1M | USDC grants to Solana public-goods projects |

## Yield Brain Results (Proven)

| Symbol | Production PnL | Optimized PnL | Consistency | Trades |
|--------|---------------|--------------|-------------|--------|
| SOL/USDT | +36.9% | **+118.3%** | 78% | 429 |
| BNB/USDT | +49.6% | — | 67% | 178 |
| ETH/USDT | +48.1% | — | 78% | 155 |
| BTC/USDT | +17.5% | — | 67% | 153 |

Active symbols: BTC/USDT, ETH/USDT, SOL/USDT, BNB/USDT.

## Critical: Fast Sim Calibration

Three invariants for the fast simulator (must match full sim):
1. **ATR formula**: `std(returns, 20h) × price` — NOT True Range
2. **MR entry condition**: `rsi < 35 and daily_trend == bullish` — NOT `bull_count >= min_alignment`
3. **Sharpe annualization**: `sqrt(n_trades / total_hours × 8760)` — NOT `sqrt(24 × 365)`

If you change `_compute_score()` or `simulate_trades()`, run `evaluator_calibration.py` to verify.

## Design Decisions

- **Post-governance, not governance** — commitment enforcement replaces voting
- **Price floor over buyback schedule** — TWAP oracle triggers, not cron
- **Correlated hedging is a feature, not a risk** — higher SOL correlation makes hedges reliable
- **Circuit breakers are layered** — cooldown (time) + epoch cap (amount) + velocity (rate)
- **Median OOS Sharpe** (not mean) — prevents fold outliers from dominating
- **Per-fold Sharpe winsorized at ±100**
- **Agents never modify each other directly** — all via Coordinator
- **Python ↔ Rust interface** is typed JSON
- **Paper trader has no Redis dependency** for runtime
- **Virtual environment**: `.venv/` with Python 3.13.3

## Tech Stack

| Layer | Technology |
|---|---|
| **On-chain** | Solana, Anchor (Rust), Token-2022, Pyth Network |
| **Agent swarm** | Rust, typed message bus, WASM sandbox |
| **DeFi integrations** | Jupiter (swaps/buybacks), Drift (hedging), Kamino/Marginfi (yield) |
| **Multisig** | Squads Protocol v4 |
| **Research** | Python, pandas, ccxt, pyarrow |

## Sponsored Hackathon Resources

| Sponsor | Use | Link |
|---------|-----|------|
| Phantom Connect | Agentic wallet + CASH stablecoin | https://docs.phantom.app/phantom-connect/introduction |
| Squads Multisig | Treasury PDA security | https://docs.squads.so |
| MoonPay Agents | Agent money movement | https://www.moonpay.com/developers/agents |
| Solana MCP | AI dev assistant for Anchor | https://github.com/solana-developers/solana-mcp |

## GitHub

- **This repo**: `git@github.com:tradewife/resilient-token-protocol.git` (hackathon submission)
- **Research repo**: `git@github.com:tradewife/rtp-skills-research.git` (pre-hackathon research)
- **Source repo**: `git@github.com:tradewife/fractal-swarm.git` (Python yield brain + data, separate)
