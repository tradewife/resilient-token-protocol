# RTP — Resilient Token Protocol

A Solana-native, self-funding treasury governed by a modular Rust swarm. Any token project adopts RTP — their trading fees route to the swarm, which autonomously researches, validates, and executes yield strategies (30K configs/night, 9-fold walk-forward validation, fee-aware simulation) — returning yield back to the project and its holders. Funded by its own yield, forever.

```
                    ┌─────────────────────────────┐
                    │     RTP SWARM COORDINATOR    │
                    │   (soulcontract.md governance)│
                    └──────────┬──────────────────┘
                               │
          ┌────────────┬───────┼───────┬───────────┬────────────┐
          │            │       │       │           │            │
     ┌────▼────┐ ┌────▼───┐ ┌▼─────┐ ┌▼────────┐ ┌▼────────┐ ┌▼────────┐
     │TRADING  │ │SECURITY│ │EVOLVE│ │KNOWLEDGE │ │AUDIT    │ │FUTURE   │
     │WING     │ │WING    │ │WING  │ │WING      │ │AUDIT    │ │PROOF    │
     │         │ │        │      │ │          │ │         │ │WING     │
     │Yield    │ │Threat  │ │Self- │ │Realtime  │ │Intent   │ │Quantum  │
     │gen +    │ │detect  │ │modify │ │knowledge │ │complian.│ │future-  │
     │exec     │ │defend  │ │adapt  │ │graph     │ │safety   │ │proofing │
     └────┬────┘ └────┬───┘ └──┬───┘ └────┬─────┘ └────┬────┘ └────┬────┘
          │           │        │          │            │           │
          └───────────┴────────┴────────┴────────────┴───────────┘
                               │
                    ┌──────────▼──────────────────┐
                    │     SOLANA TREASURY PDA      │
                    │  fees → yield → redistribute │
                    │  self-hydrate → run forever  │
                    └─────────────────────────────┘
```

## The One-Liner

Any launch platform integrates RTP — one function call per token launch. Transfer fees route to a program-owned treasury vault. An autonomous swarm generates yield, returns it to the project and holders, forever. There is no RTP token. RTP is infrastructure.

## Why This Is Different

Prior hackathon projects built individual components — treasury managers, AI agents, yield aggregators, backtesting tools. RTP is the first to combine them:

- **Constitutional governance** — soulcontract enforced in Rust (soulguard.rs) AND on-chain (Anchor `require!`). No other project has both.
- **Self-funding economics** — treasury generates its own yield via Hyperliquid perps, with irreversible phase evolution (Sustenance → Ecosystem → Humanity). No VC dependency.
- **Proven research engine** — 30K configs/night, 9-fold walk-forward validation, fee-aware simulation. Not a backtest screenshot — out-of-sample results across 9 independent time windows.
- **On-chain constraint proof** — the Anchor program deliberately rejects invalid transactions (10+ rejection tests). Constraint rejection IS the demo.
- **Hyperliquid execution** — EIP-712 signed orders from Rust, fills on HL testnet, USDC yield deposited to Solana PDA. No prior Colosseum project integrates Hyperliquid.

## Language Architecture

```
Research & Testing          Live Execution
(Python — fast iterate)     (Rust — fast runtime)

backtest ◄──────────────► deploy
optimize                   execute
simulate                   sign
hypothesize                respond

Python fractal-swarm         Rust swarm runtime
(proven, 30K configs/night) (safe, concurrent, on-chain)
```

Python owns the research loop — it's where we prove things work before risking capital. Rust owns execution — it's where latency, memory safety, and concurrent on-chain interaction matter. The two share a typed interface (JSON schema or protobuf) so any wing can propose and any wing can act.

## Why This Works

Most "autonomous agent" projects are marketing wrappers around a simple bot. RTP has a working research engine underneath:

- **30,000** parameter combinations tested per symbol per night
- **9-fold** expanding-window walk-forward validation
- **Full-sim ground truth** — 0.1% fees, 10bps slippage, max 20% position
- **Self-correction** — detects when fast sim diverges from reality and self-heals
- **Proven results** — all four symbols validated profitable with real market data

| Symbol | Production PnL | Optimized PnL | Consistency | Trades |
|--------|---------------|--------------|-------------|--------|
| SOL/USDT | +36.9% | **+118.3%** | 78% | 429 |
| BNB/USDT | +49.6% | — | 67% | 178 |
| ETH/USDT | +48.1% | — | 78% | 155 |
| BTC/USDT | +17.5% | — | 67% | 153 |

This is not a backtest screenshot. These are out-of-sample walk-forward results through a fee-aware simulator with 429 real trades across 9 independent time windows.

## Quick Demo

```bash
./demo.sh    # Runs all three layers end-to-end
```

See [docs/demo-flow.md](docs/demo-flow.md) for the 3-minute hackathon demo script.

## SDK — One Function Call

```bash
npm install @resilient-protocol/sdk @solana/web3.js @solana/spl-token @coral-xyz/anchor
```

```typescript
import { createRTPToken } from "@resilient-protocol/sdk";
import { Connection, Keypair } from "@solana/web3.js";

const connection = new Connection("https://api.devnet.solana.com");
const payer = Keypair.generate(); // or use a WalletAdapter from @solana/wallet-adapter-react

const result = await createRTPToken(connection, payer, {
  name: "Community Token",
  symbol: "CMTY",
  supply: 1_000_000_000,
  feeBps: 200,  // 2% transfer fee → treasury vault
});

// result.mint, result.treasuryPDA, result.vaultPDA
```

For browser wallets (e.g. Phantom), pass the wallet adapter directly — no keypair needed:
```typescript
import { useWallet, useConnection } from "@solana/wallet-adapter-react";

const { publicKey, signTransaction } = useWallet();
const { connection } = useConnection();

const result = await createRTPToken(connection, { publicKey, signTransaction }, config);
```

Three functions — that's the entire SDK: `createRTPToken()`, `fetchTreasuryState()`, `withdrawAndRedistribute()`. See [sdk/README.md](sdk/README.md) for details.

## Live on Devnet

Treasury program deployed and operational on Solana devnet (Apr 11 2026).

| Item | Value |
|------|-------|
| Program ID | `8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB` |
| Treasury PDA | Per-mint — demo: `FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF` |
| Explorer | [View demo treasury](https://explorer.solana.com/address/FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF?cluster=devnet) |
| Redistribution tx | [View transaction](https://explorer.solana.com/tx/9HzWgBfwYxs5ModdjF5mT6gdTfayQq8mMYipopyHfGPmYqk6KESHFqgDrc9Mcie573ttcdPqMHSyJP5nNBKK3bR?cluster=devnet) |

8/8 on-chain steps completed including live redistribution (70/20/10 split).

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│                                                                 │
│  RTP Treasury Program                                           │
│  ├── Receive fees (TransferFeeConfig from adopting token projects) │
│  ├── Strategy lifecycle (register → update → suspend/retire)    │
│  ├── Hydration gate (only Live strategies receive funding)      │
│  ├── Phantom bridge: SOL → USDC (fund HL working capital)     │
│  ├── Phantom bridge: USDC yield → SOL (return to treasury)    │
│  ├── Threshold-triggered redistribution (70/20/10 split)        │
│  ├── Self-hydration CPI (fund swarm ops from yield)            │
│  ├── Ecosystem auto-invest (excess → top RTP token LPs)        │
│  └── Phase evolution (sustenance → ecosystem → humanity fund)   │
│                                                                 │
│  Invariants (enforced on-chain):                                │
│  ├── PDA owns treasury (no private key risk)                    │
│  ├── SPL TransferFeeConfig (fees immutable from mint)           │
│  ├── CPI-only transfers (atomic, verifiable)                    │
│  ├── SOL never liquidated — bridged via Phantom, never sold     │
│  ├── Agent can propose, human must approve irreversible actions │
│  └── Treasury cannot fund Suspended/Retired strategies          │
├─────────────────────────────────────────────────────────────────┤
│                    SWARM RUNTIME (Rust)                          │
│                                                                 │
│  Coordinator ── routes tasks, mediates between wings            │
│  ├── Wing message bus (typed, async, signed)                    │
│  ├── soulcontract enforcement (immutable constraints)           │
│  └── Wing lifecycle (spawn, health-check, retire)               │
│                                                                 │
│  Wings (independently testable, concurrent Rust modules)         │
│  ├── Trading Wing    — yield generation + execution             │
│  ├── Security Wing   — threat detection + defense               │
│  ├── Evolve Wing     — self-modification + adaptation           │
│  ├── Knowledge Wing  — realtime knowledge graph                 |
│  ├── Audit Wing      — efficiency + safety + intent compliance  │
│  └── Future-proof Wing — quantum + existential monitoring      │
├─────────────────────────────────────────────────────────────────┤
│                    RESEARCH LAYER (Python)                        │
│                                                                 │
│  Yield Brain (proven, shipping)                                 │
│  ├── Night Shift — 30K configs/night, 9-fold WFA, Darwinian    │
│  ├── Full Simulator — fees, slippage, realistic execution       │
│  ├── Self-correction — calibration + discrepancy detection      │
│  ├── Promotion Gates — statistical + regime + consensus checks  │
│  ├── Decay Monitor — hard stops + soft decay + auto-retirement  │
│  └── Paper Trader — live market validation                      │
│                                                                 │
│  Shared interface (typed JSON) between Python research          │
│  and Rust execution. Any wing can propose; any wing can act.    │
└─────────────────────────────────────────────────────────────────┘
```

## The Swarm Wings

Each wing is an independent Rust module with a defined interface, its own state, and the ability to propose actions to other wings via the Coordinator. Wings never modify each other directly — all cross-wing communication goes through typed, signed messages mediated by the soulcontract.

### Trading Wing

The only wing that touches capital. Responsible for generating yield.

| Capability | Layer | Status |
|-----------|-------|--------|
| Strategy research (30K configs/night, 9-fold WFA) | Python | **Shipping** |
| Full-sim validation (fees, slippage, 429 trades validated) | Python | **Shipping** |
| Self-correction (fast sim vs full sim calibration) | Python | **Shipping** |
| Paper trading (live Binance, ADX filter, state persistence) | Python | **Shipping** |
| Live execution on Hyperliquid (testnet) | Rust | **Done** — EIP-712 signed, round-trip verified, PnL tracked |
| Degradation detection + auto-recalibration trigger | Rust | Planned |
| Strategy lifecycle (hypothesis → validate → deploy → retire) | Both | **Built** — PromotionGate + RetirementGate + DecayMonitor (7 tests) |
| On-chain lifecycle enforcement (StrategyRecord PDA) | Solana | **Built** — register/update/retire instructions, hydrate gate (17 tests) |

### Security Wing

Proactively monitors, detects, and responds to threats before they become incidents. Never sleeps.

| Capability | Description |
|-----------|-------------|
| Vulnerability scanning | Continuous audit of own code, dependencies, and on-chain program |
| Threat intelligence | Ingest security advisories, exploit databases, Solana program analyses |
| Runtime defense | Monitor tx patterns for anomalous behavior, front-running, oracle manipulation |
| Incident response | Automated containment — halt affected components, propose rollback |
| Attack surface reduction | Flag unused authority, unnecessary CPI paths, excessive compute |

### Evolve Wing

Modifies the swarm's own architecture in bounded, human-approved increments. The only wing that can change how other wings work.

| Capability | Description |
|-----------|-------------|
| Self-assessment | Benchmark wing performance, identify bottlenecks and regressions |
| Architecture proposals | Propose new modules, refactor existing ones, retire dead code |
| Dependency evolution | Evaluate new crates, upgrade paths, deprecation risks |
| Config optimization | Propose parameter changes to other wings based on performance data |
| Rollback orchestration | If a change degrades performance > threshold, revert within minutes |

The Evolve Wing cannot act without soulcontract compliance. Every modification is a diff that the Audit Wing reviews before the Coordinator allows execution.

### Knowledge Wing

Builds and maintains a realtime knowledge graph spanning every aspect of the project — market data, strategy performance, security events, architectural decisions, and external research.

| Capability | Description |
|-----------|-------------|
| Market knowledge | Regime states, volatility patterns, correlation shifts, liquidity changes |
| Strategy knowledge | What worked, what failed, under what conditions, and why |
| Institutional memory | Every architectural decision, every rollback reason, every calibration fix |
| Research ingestion | Monitor Arxiv, security advisories, Solana RFCs, DeFi protocol updates |
| Cross-wing queries | Any wing can ask "what do we know about X?" and get a cited answer |

### Audit Wing

Checks that every wing, every transaction, and every proposed change complies with the soulcontract. It is the constraint layer that makes autonomy safe.

| Capability | Description |
|-----------|-------------|
| Intent compliance | Does this action serve the protocol's stated purpose, or has it drifted? |
| Efficiency audit | Is this wing using resources proportionally to its value? |
| Safety audit | Does this proposed change violate any invariant? Increase any risk budget? |
| Activity logging | Every action by every wing, timestamped, signed, and queryable |
| Amendment verification | When the soulcontract is amended, verify the change was human-signed |

The Audit Wing is the reason RTP can be autonomous without being uncontrolled. It cannot be bypassed — it is woven into the Coordinator's message routing.

### Future-proof Wing

Monitors existential and technological risks on the horizon — things that aren't problems yet but will be. This wing thinks in years, not hours.

| Capability | Description |
|-----------|-------------|
| Quantum threat monitoring | Track post-quantum cryptography progress, Solana's migration timeline |
| Protocol deprecation | Monitor Solana runtime changes, Anchor version churn, CPI breakage |
| Regulatory horizon | Track evolving crypto regulation across jurisdictions |
| Technological disruption | New consensus mechanisms, alternative L1s, cross-chain evolution |
| Capability assessment | When a new threat class emerges, assess which wings need updates |

## How the Yield Brain Works

The Trading Wing's research layer is not a black box. Here is exactly what it does every night:

1. **Load** 9,600 hours of hourly OHLCV data per symbol
2. **Split** into 9 expanding-window walk-forward folds (36-day out-of-sample windows)
3. **Grid search** ~30,000 parameter combinations (signal thresholds, stop losses, hold times, trailing stops)
4. **Score** each combination across all folds: median OOS Sharpe × consistency × (1 - overfitting) × drawdown factor
5. **Evolve** the top 100 through 5 generations of Darwinian mutation
6. **Detect overfitting** with three independent checks: IS/OOS gap, fold consistency, parameter fragility
7. **Validate** the top candidates through the full simulator (fees, slippage, realistic execution)
8. **Self-correct**: compare fast sim vs full sim rankings, flag divergences, skip symbols that disagree
9. **Output** a confidence-scored proposal to the Coordinator for Rust-side deployment

The whole pipeline runs autonomously via GitHub Actions. No human in the loop until deployment.

## Wing Communication

Wings never touch each other. All cross-wing interaction goes through the Coordinator, which enforces the soulcontract on every message:

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

Every message is typed, signed, and persisted. The Audit Wing can reconstruct the full causal chain of any decision the swarm ever made.

## Self-Hydration

The swarm funds its own operations from generated yield, making it economically irrational to shut down.

```
Yield (USDC)
    │
    ├── 90% → Treasury reserves → redistribution at threshold
    │
    └── 10% → Sustenance PDA
              │
              ├── RPC costs (~$0.01/call)
              ├── Transaction fees (~$0.001/tx on Solana)
              └── Compute (night shift CI runs)
              │
              ├── balance < 90-day burn rate?
              │     └── divert 20% (emergency)
              │
              └── balance > target?
                    └── reduce to 5%, redirect surplus
```

At $10k reserves generating 20-50% annual yield, ops cost is ~$100-200/mo. The system runs forever on its own yield.

## Treasury Capital Model — Unified SOL Cycle

The protocol operates on a single transparent cycle: **SOL in, USDC on Hyperliquid, SOL back out.**

```
SOL fees → Phantom bridge → USDC → HL perps → USDC yield → Phantom bridge → SOL → Treasury PDA → redistribute
```

| Step | Asset | Location | Mechanism |
|------|-------|----------|-----------|
| 1. Fees arrive | SOL | Treasury PDA (Solana) | TransferFeeConfig from adopting token projects |
| 2. Fund trading | SOL → USDC | Phantom bridge (mainnet) | Trustless swap at oracle price |
| 3. Execute strategies | USDC | HL clearinghouse | USDC-margined perps, EIP-712 signed |
| 4. Yield returns | USDC → SOL | Phantom bridge (mainnet) | Trustless swap at oracle price |
| 5. Redistribute | SOL | Treasury PDA | 70% holders / 20% dev / 10% ecosystem (on-chain) |

**Why this model:**
- **Single asset on-chain** — the treasury PDA only holds SOL. Judges verify the full balance on Solana Explorer.
- **USDC only in-flight** — Hyperliquid positions are USDC-margined. SOL is never at liquidation risk on HL.
- **Trustless conversion** — the Phantom bridge handles SOL↔USDC without custodial risk.
- **Auditable** — every step produces an on-chain signature or API receipt.

> **Devnet note:** The Phantom SOL↔USDC bridge is mainnet-only. On devnet, the HL clearinghouse is funded directly via faucet, and `devnet_fund_stub()` simulates the bridge for demo narrative. The treasury PDA holds SOL on devnet as it would in production.

**Self-sustaining threshold:** when USDC yield exceeds ops cost by 10x sustained over 90 days, external seed capital is no longer required and can be returned or recycled to the ecosystem fund.

## soulcontract.md

A constitutional governance layer that sits above all agent loops. Agents propose, humans approve.

**What can evolve**: strategy parameters, risk thresholds, execution venue weights, redistribution splits, strategy portfolio composition.

**What cannot evolve**: core values, human-sovereign control, self-modification of the contract without human approval, any action that increases max risk budget without consent, SOL liquidation policy.

**Amendment protocol**: propose as diff → human signs commit → 24h monitoring window → auto-rollback if performance degrades > 5%.

## Phased Evolution

| Phase | Threshold | Behavior |
|-------|-----------|----------|
| **1: Sustenance** | < $50k | Self-hydrate, reinvest all yield |
| **2: Ecosystem** | $50k–$1M | Auto-provide LP to top RTP-adopting tokens |
| **3: Humanity** | > $1M | USDC grants to Solana projects aligned with human betterment |

Phase transitions are **irreversible** — enforced on-chain. The protocol grows up, never down.

## Fee Routing

Any Solana token project can adopt RTP by enabling `TransferFeeConfig` on their mint and setting the Treasury PDA as the fee recipient. From that point, every trade on their token auto-routes a fee to the swarm. The fee config is immutable once set — it cannot be revoked.

```
Token project adopts RTP
  │
  ├── Enable TransferFeeConfig on mint (immutable)
  │       └── Every trade → fee (SOL) → Treasury PDA
  │
  ├── pump.fun (most common)
  │       └── 0.25% PumpSwap fee → 0.05% creator fee (SOL) → Treasury PDA
  │
  └── Any Solana token
          └── Custom fee % set at mint → routes to Treasury PDA

SOL reserves held in treasury PDA. Phantom bridge converts SOL→USDC to fund
Hyperliquid positions. USDC yield converts back to SOL via Phantom bridge.
Single asset on-chain, trustless conversion, fully auditable.
```

**Why projects adopt**: Their fees don't just sit in a wallet — the swarm puts them to work. Yield flows back to the project and its holders automatically. No trust required. The community's SOL is never sold.

**Rug-proof by design**: SPL TransferFeeConfig is immutable once minted. PDA owns treasury (no private key). All transfers via CPI (atomic, verifiable). Mint authority renounced post-launch.

### Multi-Token Attribution (Phase 2 Architecture)

RTP is designed to serve multiple token projects simultaneously. Fee attribution
uses a proportional model: each adopter's share of generated yield equals their
proportion of total fees contributed to the treasury.

```
AdopterRecord PDA (per token mint)
seeds: ["adopter", token_mint]
├── fees_contributed_lamports ← incremented on every fee deposit
└── deposit_count

Treasury (shared)
└── total_fees_received_lamports ← sum of all adopter contributions

At redistribution:
  adopter_yield_share = (fees_contributed / total_fees_received) × yield_pool

TokenA contributed 600 SOL → receives 60% of yield pool
TokenB contributed 400 SOL → receives 40% of yield pool

Each adopter's yield share is then distributed to that token's holders
via a balance snapshot at redistribution time.
```

**On-chain proof:** `register_adopter` and `record_fee_deposit` instructions
are live on devnet. The `AdopterRecord` PDA is queryable for any registered
token mint. See `scripts/compute_adopter_yield_share.ts` for the attribution formula.

**Phase 1 (current demo):** Single adopter, single treasury, full redistribution cycle proven on devnet.
**Phase 2:** Factory pattern — `initialize_vault` per adopter, per-adopter yield isolation.

## Capital Flow

```
                     ┌────────────────────────────┐
                     │ TOKEN PROJECT TRADING FEES  │
                     │ (pump.fun, or any token)    │
                     └──────────┬─────────────────┘
                                │ TransferFeeConfig (immutable, SOL)
                                ▼
                     ┌──────────▼─────────────────┐
                     │     SOLANA TREASURY PDA     │
                     │                             │
                     │  SOL reserves               │
                     │  ├─ Never liquidated        │
                     │  └─ Bridged to USDC via     │
                     │       Phantom (mainnet)     │
                     └──────────┬─────────────────┘
                                │ SOL → USDC (Phantom bridge)
                                ▼
                     ┌──────────▼─────────────────┐
                     │  HYPERLIQUID CLEARINGHOUSE  │
                     │                             │
                     │  USDC working capital       │
                     │  ├─ USDC-margined perps     │
                     │  ├─ EIP-712 signed orders   │
                     │  └─ Yield compounds         │
                     └──────────┬─────────────────┘
                                │ USDC yield → SOL (Phantom bridge)
                                ▼
                     ┌──────────▼─────────────────┐
                     │     SOLANA TREASURY PDA     │
                     │                             │
                     │  reserves > threshold?      │
                     │     /          \            │
                     │   NO            YES         │
                     │   /              \          │
                     │  Reinvest    Redistribute   │
                     │  (Yield)     70% holders    │
                     │              20% project    │
                     │              10% ecosystem  │
                     └─────────────────────────────┘
```

Single asset on-chain (SOL). USDC only exists in-flight on Hyperliquid. Trustless conversion via Phantom bridge. Judges verify the full SOL balance on Solana Explorer.

## What This Is Not

- **Not a token** — there is no RTP token. RTP is pure infrastructure that serves the tokens that adopt it
- **Not a vault** — no custody of user funds, no withdrawal interface
- **Not dependent on LLMs** — core loop is deterministic Python; LLMs optional for hypothesis generation
- **Not just a trading bot** — it's infrastructure that any launch platform can integrate
- **Not requiring venture infrastructure** — runs on a single machine, no database, no Kubernetes
- **Not liquidating community SOL** — SOL is bridged to USDC via Phantom (trustless), never sold on the open market. SOL on the treasury PDA is never at risk of liquidation.

## What We Already Have (Proven)

The Trading Wing's research layer is shipping today. Everything else is scaffolding.

| Component | Wing | Layer | Status |
|-----------|------|-------|--------|
| Night Shift Optimizer (30K configs → WFA → Darwinian) | Trading | Python | **Shipping** |
| Full Simulator (0.1% fees, 10bps slippage, ground truth) | Trading | Python | **Shipping** |
| Paper Trader (live Binance, ADX filter, state persistence) | Trading | Python | **Shipping** |
| Self-Correction (fast sim vs full sim calibration) | Trading | Python | **Shipping** |
| CI Pipeline (nightly cron, auto-commit, 300min timeout) | Trading | Infra | **Shipping** |
| Devnet Loop (6h cron, LLM mutations, config chaining) | Evolve | Infra | **Shipping** |
| SOL Optimized Config (+118.3% PnL, 78% consistency, 429 trades) | Trading | Python | **Shipping** |
| Treasury Program (Anchor: deposit, distribute, hydrate, evolve) | — | Solana | **Built** (audit remediated) |
| soulcontract.md (constitutional governance layer) | — | Governance | **Defined** |
| Python ↔ Rust Bridge (typed JSON, bridge-mode subprocess) | Trading | Both | **Built** |
| Coordinator (soulguard + router + lifecycle) | — | Rust | **Built** (307 tests) |
| Evolve Wing (assessor + proposer + rollback) | Evolve | Rust | **Built** |
| Audit Wing (3-agent tribunal, Byzantine consensus) | Audit | Rust | **Built** |
| Trading Wing (bridge-backed execution, in-memory state) | Trading | Rust | **Built** |
| Security Wing (threat detection, rate-limiting, alert tracking) | Security | Rust | **Built** |
| Knowledge Wing (in-memory graph, cross-wing queries) | Knowledge | Rust | **Built** |
| Future-proof Wing (deprecation monitoring, heartbeat) | Future-proof | Rust | **Built** |

## Project Structure

```
rtp/
├── swarm/                           # Rust swarm runtime
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                  # Re-exports, module declarations
│   │   ├── types.rs                # Message, Payload, WingId, Priority
│   │   ├── bridge.rs               # Python ↔ Rust typed subprocess interface
│   │   ├── config.rs               # Swarm configuration
│   │   ├── demo.rs                 # End-to-end demo loop (8-step pipeline)
│   │   ├── coordinator/
│   │   │   ├── mod.rs              # Multi-stage quality gate pipeline
│   │   │   ├── router.rs           # Typed routing, retry, proposal→audit flow
│   │   │   ├── soulguard.rs        # Enforce soulcontract on every message
│   │   │   ├── soulcontract_spec.rs # Parse soulcontract.md → constraints
│   │   │   └── lifecycle.rs        # Wing spawn, health-check, retire
│   │   └── wings/
│   │       ├── trading/mod.rs      # Bridge-backed execution, in-memory state
│   │       ├── security/mod.rs     # Threat detection, rate-limiting, alerts
│   │       ├── evolve/
│   │       │   ├── mod.rs          # Darwinian loop orchestration + LLM proposer
│   │       │   ├── assessor.rs     # Treasury-native performance scoring
│   │       │   ├── proposer.rs     # SPARC-inspired proposal lifecycle
│   │       │   └── rollback.rs     # Auto-revert on >5% degradation
│   │       ├── knowledge/mod.rs    # In-memory knowledge graph, cross-wing queries
│   │       ├── audit/mod.rs        # 3-agent tribunal (Byzantine consensus)
│   │       └── futureproof/mod.rs  # Deprecation monitoring, heartbeat
│   └── src/bin/
│       ├── demo.rs                 # One-shot demo binary (5 judge points)
│       └── daemon.rs               # Autonomous devnet loop (6h CI cron)
│
├── programs/                        # Solana (Anchor)
│   └── rtp-treasury/              # Deposit, distribute, hydrate, evolve, strategy lifecycle
│
├── soulcontract.md                  # Constitutional governance
├── third-party-disclosure.md        # MIT framework disclosures
├── data/
│   ├── ohlcv/
│   ├── night_results/
│   ├── paper_trading/
│   ├── calibration/
│   ├── discrepancies/
│   └── devnet-cycles/              # Autonomous cycle output (auditable trail)
│
├── research/                       # Python research layer
│   ├── promotion_criteria.py       # PromotionGate, RetirementGate, StrategyStatus, DecayRisk
│   ├── strategy_library.md         # 15 strategies (S01–S15)
│   ├── dead_ends.md                # Failure memory log + retirement criteria
│   └── validation/
│       ├── validate_night_shift.py # WFA validator + promotion eligibility
│       ├── promotion_checker.py    # Evaluates validation result against PromotionGate
│       ├── decay_monitor.py        # DecayMonitor: hard stops + soft decay tracking
│       └── test_decay_monitor.py   # 7 pytest tests for lifecycle gates
│
└── .github/workflows/
    ├── night_shift.yml             # Nightly research pipeline
    ├── swarm-ci.yml                # Rust build + test + clippy + anchor build
    └── devnet-loop.yml             # 6h autonomous devnet cycle
```

## Development Phases

The swarm is designed so that each phase adds independent, testable capability. Nothing depends on everything being built first.

### Phase 0: Proven Foundation (Shipping)

The Python fractal-swarm — already running, already profitable, already autonomous.

- Night shift optimizer (30K configs, 9-fold WFA, Darwinian evolution)
- Full-sim validation (0.1% fees, 10bps slippage, ground truth)
- Paper trader (live Binance, state persistence, ADX filter)
- Self-correction (calibration + discrepancy detection)
- CI pipeline (nightly cron, auto-commit)

### Phase 1: Treasury + Coordinator + All Wings (Current)

Treasury program audit-remediated. All 6 wings built. Coordinator with full quality gate pipeline. Bridge connecting Python research to Rust execution. Autonomous devnet loop running.

- ✅ Coordinator + typed message bus + soulguard + spec-based drift detection
- ✅ Evolve Wing (assessor + proposer + rollback with 5% degradation threshold)
- ✅ Audit Wing (3-agent tribunal: Skeptic/UserProxy/Optimizer, Byzantine consensus)
- ✅ Trading Wing (bridge-backed execution, 5 payload types, in-memory state)
- ✅ Security Wing (threat detection, rate-limiting, suspicious-proposal flagging)
- ✅ Knowledge Wing (in-memory knowledge graph, cross-wing queries)
- ✅ Futureproof Wing (deprecation monitoring, heartbeat)
- ✅ Treasury program on devnet (Anchor 1.0, audit remediated)
- ✅ Python ↔ Rust typed bridge (`rtp/swarm/src/bridge.rs`)
- ✅ End-to-end demo loop (`rtp/swarm/src/demo.rs`, 8-step pipeline)
- ✅ Autonomous devnet loop (`rtp-daemon` binary, 6h CI cron, LLM mutations)
- ✅ Test suite: 307 tests, 0 failures, 0 clippy warnings (anchor: 34 passing).

### Phase 2: End-to-End Integration + Full Loop

Wire remaining wings and complete the end-to-end demo flow.

- Sentinel dashboard deployed to GitHub Pages (live URL for judges)
- Phantom SDK "Fund Treasury" button (SOL → USDC swap via Jupiter routing)
- Knowledge Wing hardening beyond in-memory store
- Security Wing hardening beyond in-memory alert/rate-limit logic

### Phase 3: Polish + Submission

Demo rehearsal, video, hardening, Colosseum submission by May 11.

### Phase 4: Eternal Autonomy

All wings operational. Human role reduced to soulcontract amendments. The swarm runs, improves, defends, and evolves — funded by its own yield. SOL collateral loop operational via Phantom-native integration.

## Quick Start

```bash
# Set up environment
python -m venv .venv && source .venv/bin/activate
pip install pandas numpy ccxt pyarrow redis

# Run the fractal-swarm (night shift)
python -m research.orchestration.night_shift --skip-fetch

# Live paper trading
PYTHONUNBUFFERED=1 python -m research.live.paper_trader

# Validate candidates through full simulator
python -m research.validation.validate_night_shift --production

# Check system calibration
python -m research.optimization.evaluator_calibration --samples 20

# Run one-shot demo (5 judge points)
cargo run --bin rtp-demo

# Run autonomous devnet cycle (6h CI cron)
cargo run --bin rtp-daemon
```

## Hackathon

**SWARM — Canteen × Colosseum**

| Criterion | Weight | What Judges Want | How RTP Delivers |
|---|---|---|---|
| **Functionality** | ? | Working demo with real transactions | Live: adopt→fees→swarm→yield→redistribute on devnet |
| **Potential Impact** | ? | Project with lasting real-world value | Any Solana token can adopt — unruggable yield standard |
| **Novelty** | ? | Novel approach, original architecture | Six-wing modular swarm + token adoption model |
| **UX** | ? | Great demo experience | Phantom Connect + MCP server (agentic wallet), devnet treasury live, 3-min demo |
| **Open-source** | ? | Clean, well-documented repo | Full swarm arch + treasury program (MIT), clean repo history |
| **Business Plan** | ? | Viable business model | Adoption fees → self-funding swarm → yield back to holders |

### Demo Flow (3 minutes)

1. "A token project adopts RTP — TransferFeeConfig set, fees auto-route to the treasury"
2. "This is our night shift — it tested 30,000 strategy configs last night, fully autonomous"
3. "Here's the best one — +118% PnL, 78% consistency, 9 independent validation folds"
4. "The Trading Wing proposes deployment — the Audit Wing checks it against the soulcontract"
5. "Approved — the Rust executor places the trade, yield flows back to the project and holders"
6. "At threshold, it auto-redistributes — 70% holders, 20% project dev, 10% ecosystem — all on-chain"

## Third-Party Components

| Component | License | Use |
|-----------|---------|-----|
| atlas-gic | MIT | Multi-agent Darwinian loop — Evolve Wing autoresearch |
| karpathy/autoresearch | MIT | Core Modify/Verify/Keep loop specification |
| uditgoenka/autoresearch | MIT | Claude-native implementation |
| Phantom Connect | Sponsored | Agentic wallet + CASH stablecoin flows |
| CASH | Sponsored | Stablecoin for treasury transactions |
| Squads Multisig | Sponsored | Treasury PDA security + multisig authority |
| Swig | Sponsored | Programmable smart wallets for wing message bus |
| MoonPay Agents | Sponsored | VC capital on-ramp → treasury USDC deposit |

## License

MIT
