# RTP — Resilient Token Protocol

> **Post-governance, commitment-enforced token longevity layer for Solana.**
> Every promise must be enforced, not stated.

RTP transforms "don't rug" from a social promise into a cryptographically enforced, agent-operated system. Any token project adopts RTP and their token structurally cannot rug — the economics are code-enforced, every action is provable on-chain, and price defense is autonomous.

**Category:** Unruggable launch standard · Trust-minimized token primitive

```
Non-RTP token price = SOL macro + founder risk + rug risk + narrative
RTP token price     = SOL macro + narrative
```

## Why Not a DAO?

RTP replaces governance with commitment enforcement.

| Generation | Trust Model | Failure Mode |
|---|---|---|
| **Gen 1** — Teams/Foundations | Trust people | Rug pull |
| **Gen 2** — DAOs | Trust voting | Inefficiency, plutocrat capture |
| **Gen 3** — RTP | Trust code-enforced commitments | Rigidity (acceptable tradeoff) |

---

## How It Works

A token project adopts RTP by enabling `TransferFeeConfig` on their mint. Every trade auto-routes a fee to the RTP Treasury PDA. The treasury is constrained — it can only execute price defense, hedging, and verified redistribution. Nobody can drain it. The protocol is economically irrational to shut down.

```
Token project adopts RTP
         │
         ├── TransferFeeConfig (immutable from mint)
         │       └── Every trade → fee → Treasury PDA
         │
         ├── Treasury (PDA-controlled, constrained)
         │       ├── → Buyback Agent (price floor defense)
         │       ├── → Hedge Agent (correlated SOL-short)
         │       └── → Yield Agent (idle capital deployment)
         │
         └── Verifier Agent
                 └── Every action on-chain, auditable, provable
```

### Three Interlocking Flywheels

```
┌─────────────────┐     ┌──────────────────┐     ┌────────────────────┐
│   Fee Revenue   │     │   Hedge Yield    │     │  Yield / Arbitrage │
│                 │     │                  │     │                    │
│ tx fees →       │     │ correlated short │     │ idle treasury →    │
│ treasury →      │────▶│ pays in drawdowns│────▶│ yield protocols → │
│ buyback pressure│     │ funds buybacks   │     │ compounds reserves │
└─────────────────┘     └──────────────────┘     └────────────────────┘
         ▲                                                │
         └────────────────────────────────────────────────┘
```

**Key insight:** RTP tokens eliminate founder and rug risk, producing structurally higher SOL correlation — making correlated hedging more reliable as a self-reinforcing property.

---

## Core Primitives

### 1. Fee Routing

Immutable fee allocation via SPL Token-2022 `TransferFeeConfig`. Projects set their fee percentage at mint — fees auto-route to the RTP Treasury PDA on every trade. No middlemen, no trust required.

### 2. Autonomous Buybacks

Treasury USDC → token via Jupiter when `price < floor × discount`. The price floor is `treasury_value_usd / circulating_supply`, enforced by Pyth TWAP oracle + circuit breaker. Buybacks are algorithmic and unstoppable.

### 3. Correlated Hedging

SOL-short via Drift Protocol. Because RTP tokens have structurally higher SOL correlation (no founder/rug noise), correlated hedges are more reliable — they pay for price defense during drawdowns, making the system self-funding.

### 4. Circuit Breakers

3-layer protection preventing treasury drain:
- **Cooldown** — minimum time between treasury operations
- **Epoch cap** — max USDC spendable per epoch
- **Velocity limit** — max rate of reserve depletion

### 5. Yield Deployment

Idle treasury capital deployed to Kamino/Marginfi. Yield compounds reserves, increasing buyback capacity and floor price.

### 6. Verification

Every agent action publishes on-chain proof. The Verifier Agent publishes a complete audit trail — every swap, every hedge, every buyback, timestamped and queryable. "Don't trust, verify" is enforced, not aspirational.

---

## Redistribution

Above a configurable threshold, the treasury auto-redistributes:

| Trigger | Split | Enforcement |
|---|---|---|
| Reserves > threshold (default $10k USDC) | 70% holders (pro-rata) / 20% dev / 10% ecosystem | CPI, auditable on-chain |
| Reserves < threshold | 100% reinvest (yield/buybacks) | Risk-gated by agents |

```bash
# Demo flow in 3 minutes:
# 1. Token project adopts RTP → trades generate fees
# 2. Fees flow to Treasury PDA → swarm operates
# 3. Price dips below floor → autonomous buyback fires
# 4. Verify: holders protected, SOL untouched, every tx on-chain
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│                                                                 │
│  RTP Treasury Program                                           │
│  ├── Receive fees (TransferFeeConfig → Treasury PDA)            │
│  ├── Price floor (Pyth TWAP oracle + circuit breaker)           │
│  ├── Buyback execution (Jupiter CPI)                            │
│  ├── Hedging (Drift Protocol perps)                             │
│  ├── Yield deployment (Kamino/Marginfi)                         │
│  ├── Threshold redistribution (70/20/10 split)                  │
│  ├── Phase evolution (sustenance → ecosystem → humanity)        │
│  └── Self-hydration (fund swarm ops from yield)                 │
│                                                                 │
│  On-chain invariants (enforced, not stated):                    │
│  ├── PDA owns treasury (no private key risk)                    │
│  ├── TransferFeeConfig immutable from mint (no fee revocation)  │
│  ├── Circuit breakers prevent treasury drain                    │
│  └── Every action verified and auditable                        │
├─────────────────────────────────────────────────────────────────┤
│                    AGENT SWARM (Rust)                            │
│                                                                 │
│  Coordinator — routes tasks, mediates, enforces soulcontract     │
│  ├── Agent message bus (typed, async, signed)                   │
│  ├── soulcontract enforcement (immutable constraints)            │
│  └── Agent lifecycle (spawn, health-check, retire)              │
│                                                                 │
│  Agent Roles (each independently testable):                      │
│  ├── Allocator  — reads inflows, routes funds per rules         │
│  ├── Executor   — swaps, LP management, hedging via Jupiter     │
│  └── Verifier   — publishes proof of every action on-chain      │
├─────────────────────────────────────────────────────────────────┤
│                    RESEARCH LAYER (Python)                        │
│                                                                 │
│  Yield Brain (proven, shipping)                                 │
│  ├── Night Shift — 30K configs/night, 9-fold WFA, Darwinian    │
│  ├── Full Simulator — fees, slippage, realistic execution       │
│  ├── Self-correction — calibration + discrepancy detection      │
│  └── Paper Trader — live market validation                      │
└─────────────────────────────────────────────────────────────────┘
```

### Language Architecture

```
Research & Testing          Live Execution
(Python — fast iterate)     (Rust — fast runtime)

backtest ◄──────────────► deploy
optimize                   execute
simulate                   sign
hypothesize                respond

Python yield brain         Rust agent swarm
(proven, 30K configs/night) (safe, concurrent, on-chain)
```

Python owns the research loop — it's where we prove strategies work before risking capital. Rust owns execution — latency, memory safety, concurrent on-chain interaction. The two share a typed JSON interface.

## The Swarm

The swarm is the operational layer that makes the token longevity promise autonomous. Three core agent roles:

### Allocator Agent

Reads inflows from fee routing, evaluates current market conditions (regime detection via the yield brain), and routes treasury funds per the soulcontract's immutable rules. Never improvises — only executes within defined constraints.

### Executor Agent

Places trades via Jupiter (swaps), Drift (hedging), Kamino/Marginfi (yield). Every execution is signed, timestamped, and submitted to the Verifier for on-chain proof publication.

### Verifier Agent

The root-of-trust. Publishes cryptographic proof of every agent action. Any deviation from the soulcontract triggers circuit breaker activation. Every decision the swarm ever made is reconstructable.

### Skill System

Every agent behavior is defined as an atomic, deterministic skill:

**Trigger → Inputs → Action → Constraints → Outputs → Proof**

| Domain | Skills |
|---|---|
| **Inflow & Routing** | fee_capture, revenue_split, treasury_deposit |
| **Market Actions** | buyback_execute, regime_detect, price_floor_check |
| **Treasury Strategies** | hedge_basket_manage, yield_deploy, drawdown_response |
| **Verification** | tx_verify, proof_publish, invariant_check, audit_log |
| **Coordination** | consensus, heartbeat, escalation, lifecycle |

Each skill has an economic guarantee, failure mode, detection mechanism, and fallback. The Verifier certifies skills as VERIFIED (≥0.90), RESTRICTED (0.75–0.89), or REJECTED (<0.75).

## Yield Brain (Proven)

The Trading subsystem is not vaporware. It ships today with validated results:

- **30,000** parameter combinations tested per symbol per night
- **9-fold** expanding-window walk-forward validation
- **Full-sim ground truth** — 0.1% fees, 10bps slippage, max 20% position
- **Self-correction** — detects when fast sim diverges from reality and self-heals

| Symbol | Production PnL | Optimized PnL | Consistency | Trades |
|--------|---------------|--------------|-------------|--------|
| SOL/USDT | +36.9% | **+118.3%** | 78% | 429 |
| BNB/USDT | +49.6% | — | 67% | 178 |
| ETH/USDT | +48.1% | — | 78% | 155 |
| BTC/USDT | +17.5% | — | 67% | 153 |

Out-of-sample walk-forward results through a fee-aware simulator with 429 real trades across 9 independent time windows. This research engine feeds strategy proposals to the Executor Agent.

## Fee Flow

```
pump.fun trade (0.25% fee) → 0.05% creator fee (SOL) → RTP Treasury PDA
                                                              │
                                                              ├─ 90% → Yield strategies (USDC via Jupiter)
                                                              └─ 10% → Ecosystem SOL reserves (compounds)
```

**Rug-proof by design**: SPL TransferFeeConfig is immutable once minted. PDA owns treasury (no private key). All transfers via CPI (atomic, verifiable). Circuit breakers prevent drain. Mint authority renounced post-launch.

## Phased Evolution

| Phase | Threshold | Behavior |
|-------|-----------|----------|
| **1: Sustenance** | < $50k | Self-hydrate, reinvest all yield |
| **2: Ecosystem** | $50k–$1M | Auto-provide LP to top RTP-adopting tokens |
| **3: Humanity** | > $1M | USDC grants to Solana public-goods projects |

Phase transitions are **irreversible** on-chain. The protocol grows up, never down.

## soulcontract.md

Constitutional governance layer. **What can evolve**: strategy parameters, risk thresholds, redistribution splits, hedge weights. **What cannot**: core values, human sovereign control, fee immutability, PDA ownership, phase reversal. **Amendment protocol**: propose as diff → human signs → 24h monitoring → auto-rollback if performance degrades > 5%.

## Project Structure

```
rtp/
├── swarm/                           # Rust agent swarm
│   ├── src/
│   │   ├── coordinator/            # Message bus + soulguard
│   │   ├── agents/
│   │   │   ├── allocator/          # Inflow routing per rules
│   │   │   ├── executor/           # Jupiter, Drift, Kamino execution
│   │   │   └── verifier/           # On-chain proof publication
│   │   └── skills/                 # Atomic skill definitions
│   └── tests/
│
├── programs/                        # Solana (Anchor)
│   └── rtp-treasury/              # Fee routing, buybacks, hedging, redistribution
│
├── soulcontract.md                  # Constitutional governance
├── BUILD_PLAN.md                   # Full build plan v3.0
├── third-party-disclosure.md        # MIT framework + sponsor attributions
├── docs/demo-flow.md               # 3-minute hackathon demo
│
└── data/                            # (gitignored — local development)
```

## Development Phases

### Phase 0: Proven Foundation (Shipping)

The Python yield brain — already running, already profitable, already autonomous. Night shift optimizer, full-sim validation, paper trader, self-correction, CI pipeline.

### Phase 1: Token Longevity Core (Hackathon Target)

The product. Treasury program on devnet with fee routing, price floor, circuit breakers, and one autonomous buyback.

- Treasury program (Anchor): fee receive → floor check → buyback CPI → redistribute
- Price floor: Pyth TWAP oracle + circuit breaker enforcement
- TransferFeeConfig integration for adopting tokens
- Basic allocator + executor + verifier agents (Rust stubs)
- Demo: token adopts RTP → trade → fee flows → floor defended → buyback fires

### Phase 2: Hedging + Yield

Make the treasury self-reinforcing.

- Correlated SOL-short via Drift Protocol
- Yield deployment to Kamino/Marginfi
- Regime detection (yield brain informs hedge weights)
- Three flywheels operational

### Phase 3: Full Swarm + Evolution

All agents operational. Self-assessment, architecture proposals, rollback. Human role reduced to soulcontract amendments.

### Phase 4: Eternal Autonomy

All phases operational. The protocol runs, defends, hedges, yields, and verifies — funded by its own fees.

## Hackathon

**SWARM — Canteen × Colosseum** · $300k prizes · May 11, 2026

| Criterion | What Judges Want | How RTP Delivers |
|---|---|---|
| **Functionality** | Working demo with real transactions | Token adopts RTP → fee → buyback → verify on devnet |
| **Potential Impact** | Lasting real-world value | Unruggable launch standard — every Solana token can use it |
| **Novelty** | Novel approach, original architecture | Post-governance commitment enforcement, correlated hedging |
| **UX** | Great demo experience | 3-min demo: adopt → trade → defend → verify |
| **Open-source** | Clean, well-documented repo | Full swarm + treasury (MIT), clean repo history |
| **Business Plan** | Viable model | Fee routing → self-funding treasury → ecosystem flywheel |

## Third-Party Components

| Component | License | Use |
|-----------|---------|-----|
| atlas-gic | MIT | Multi-agent Darwinian loop — evolve agent strategies |
| karpathy/autoresearch | MIT | Modify/Verify/Keep loop specification |
| Phantom Connect | Sponsored | Agentic wallet + CASH stablecoin flows |
| Squads Multisig | Sponsored | Treasury PDA security + multisig authority |
| Drift Protocol | Open | Correlated SOL hedging via perps |
| Pyth Network | Open | TWAP oracle for price floor enforcement |
| Jupiter | Open | Swap aggregator for buybacks + yield routing |
| Kamino/Marginfi | Open | Yield deployment for idle treasury capital |

## License

MIT

---

<sub>RTP — because "trust me bro" is not a tokenomics strategy.</sub>
