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
     │WING     │ │WING    │ │WING  │ │WING      │ │WING     │ │PROOF    │
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

Any token project adopts RTP, their trading fees feed a swarm that researches, validates, and executes yield strategies — returning yield back to the project and its token holders, autonomously, verifiably, and forever.

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

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│                                                                 │
│  RTP Treasury Program                                           │
│  ├── Receive fees (TransferFeeConfig from adopting token projects) │
│  ├── Threshold-triggered redistribution (70/20/10 split)        │
│  ├── Self-hydration CPI (fund swarm ops from yield)            │
│  ├── Ecosystem auto-invest (excess → top RTP token LPs)        │
│  └── Phase evolution (sustenance → ecosystem → humanity fund)   │
│                                                                 │
│  Invariants (enforced on-chain):                                │
│  ├── PDA owns treasury (no private key risk)                    │
│  ├── SPL TransferFeeConfig (fees immutable from mint)           │
│  ├── CPI-only transfers (atomic, verifiable)                    │
│  └── Agent can propose, human must approve irreversible actions │
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
| Live execution on Hyperliquid + Jupiter | Rust | Planned |
| Degradation detection + auto-recalibration trigger | Rust | Planned |
| Strategy lifecycle (hypothesis → validate → deploy → retire) | Both | Planned |

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

## soulcontract.md

A constitutional governance layer that sits above all agent loops. Agents propose, humans approve.

**What can evolve**: strategy parameters, risk thresholds, execution venue weights, redistribution splits, strategy portfolio composition.

**What cannot evolve**: core values, human-sovereign control, self-modification of the contract without human approval, any action that increases max risk budget without consent.

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
  │       └── Every trade → fee → Treasury PDA
  │
  ├── pump.fun (most common)
  │       └── 0.25% PumpSwap fee → 0.05% creator fee (SOL) → Treasury PDA
  │
  └── Any Solana token
          └── Custom fee % set at mint → routes to Treasury PDA
```

**Why projects adopt**: Their fees don't just sit in a wallet — the swarm puts them to work. Yield flows back to the project and its holders automatically. No trust required.

**Rug-proof by design**: SPL TransferFeeConfig is immutable once minted. PDA owns treasury (no private key). All transfers via CPI (atomic, verifiable). Mint authority renounced post-launch.

## Capital Flow

```
                     ┌────────────────────────────┐
                     │ TOKEN PROJECT TRADING FEES  │
                     │ (pump.fun, or any token)    │
                     └──────────┬─────────────────┘
                                │ TransferFeeConfig (immutable)
                                ▼
                     ┌──────────▼─────────────────┐
                     │     SOLANA TREASURY PDA     │
                     │                             │
                     │  ┌─ Sustenance PDA (ops)   │
                     │  ├─ Ecosystem Fund (SOL)    │
                     │  └─ Redistribution (USDC)   │
                     └──────────┬─────────────────┘
                                │
                  reserves > threshold?
                     /          \
                   NO            YES
                   /              \
          ┌──────────────┐  ┌──────────────────┐
          │  Reinvest    │  │  Redistribute    │
          │  (Yield)     │  │  70% holders     │
          └──────┬───────┘  │  20% project dev │
                 │          │  10% ecosystem   │
     ┌───────────▼────────┐ └────────┬─────────┘
     │  YIELD BRAIN       │          │
     │                    │          │
     │  Night Shift       │          │
     │  (30K configs/     │          │
     │   symbol/night)    │          │
     │                    │          │
     │  Paper Trader      │          │
     │  (live validation) │          │
     │                    │          │
     │  Feedback Loop     │          │
     │  (degradation      │          │
     │   + recalibration) │          │
     └──────────┬─────────┘          │
                │                    │
     ┌──────────▼─────────┐          │
     │  EXECUTION         │          │
     │  ├─ Hyperliquid    │          │
     │  ├─ Jupiter swaps  │          │
     └─ └─ Solana lending │          │
     └──────────┬─────────┘          │
                │                    │
                └──► USDC yield ────►┘
```

## What This Is Not

- **Not a meme coin** — RTP is infrastructure, not a token (initially)
- **Not a vault** — no custody of user funds, no withdrawal interface
- **Not dependent on LLMs** — core loop is deterministic Python; LLMs optional for hypothesis generation
- **Not just a trading bot** — it's a token standard that any Solana project can adopt
- **Not requiring venture infrastructure** — runs on a single machine, no database, no Kubernetes

## What We Already Have (Proven)

The Trading Wing's research layer is shipping today. Everything else is scaffolding.

| Component | Wing | Layer | Status |
|-----------|------|-------|--------|
| Night Shift Optimizer (30K configs → WFA → Darwinian) | Trading | Python | **Shipping** |
| Full Simulator (0.1% fees, 10bps slippage, ground truth) | Trading | Python | **Shipping** |
| Paper Trader (live Binance, ADX filter, state persistence) | Trading | Python | **Shipping** |
| Self-Correction (fast sim vs full sim calibration) | Trading | Python | **Shipping** |
| CI Pipeline (nightly cron, auto-commit, 300min timeout) | Trading | Infra | **Shipping** |
| SOL Optimized Config (+118.3% PnL, 78% consistency, 429 trades) | Trading | Python | **Shipping** |
| Treasury Program (Anchor: deposit, distribute, hydrate, evolve) | — | Solana | **Built** (audit in progress) |
| soulcontract.md (constitutional governance layer) | — | Governance | **Defined** |
| Coordinator (soulguard + router + lifecycle) | — | Rust | **Built** (88 tests) |
| Evolve Wing (assessor + proposer + rollback) | Evolve | Rust | **Built** (88 tests) |
| Audit Wing (3-agent tribunal, Byzantine consensus) | Audit | Rust | **Built** (88 tests) |
| Security Wing | Security | Rust | Stub |
| Knowledge Wing | Knowledge | Rust | Stub |
| Future-proof Wing | Future-proof | Rust | Stub |

## Project Structure

```
rtp/
├── swarm/                           # Rust swarm runtime
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── types.rs                # Message, Payload, WingId, Priority
│   │   ├── coordinator/
│   │   │   ├── mod.rs              # Multi-stage quality gate pipeline
│   │   │   ├── router.rs           # Typed routing, retry, proposal→audit flow
│   │   │   ├── soulguard.rs        # Enforce soulcontract on every message
│   │   │   ├── soulcontract_spec.rs # Parse soulcontract.md → constraints
│   │   │   └── lifecycle.rs        # Wing spawn, health-check, retire
│   │   └── wings/
│   │       ├── trading/mod.rs      # Stub — TradingConfig handler
│   │       ├── security/mod.rs     # Stub — heartbeat handler
│   │       ├── evolve/
│   │       │   ├── mod.rs          # Darwinian loop orchestration
│   │       │   ├── assessor.rs     # Treasury-native performance scoring
│   │       │   ├── proposer.rs     # SPARC-inspired proposal lifecycle
│   │       │   └── rollback.rs     # Auto-revert on >5% degradation
│   │       ├── knowledge/mod.rs    # Stub — query handler
│   │       ├── audit/mod.rs        # 3-agent tribunal (Byzantine consensus)
│   │       └── futureproof/mod.rs  # Stub — heartbeat handler
│
├── programs/                        # Solana (Anchor)
│   └── rtp-treasury/              # Deposit, distribute, hydrate, evolve
│
├── soulcontract.md                  # Constitutional governance
├── BUILD_PLAN.md                   # Full build plan v2.2
├── BUILD_PLAN_v3.md                # Post-audit remediation plan
├── third-party-disclosure.md        # MIT framework disclosures
├── data/
│   ├── ohlcv/
│   ├── night_results/
│   ├── paper_trading/
│   ├── deployments/
│   ├── calibration/
│   └── audit/
│
└── .github/workflows/
    └── night_shift.yml
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

### Phase 1: Treasury + Coordinator + Core Wings (Current)

Treasury program, Coordinator, Evolve Wing, and Audit Wing are built. Security audit completed — fixing critical findings before demo.

- ✅ Coordinator + typed message bus + soulguard + spec-based drift detection
- ✅ Evolve Wing (assessor + proposer + rollback with 5% degradation threshold)
- ✅ Audit Wing (3-agent tribunal: Skeptic/UserProxy/Optimizer, Byzantine consensus)
- ✅ Treasury program on devnet (Anchor 1.0)
- 🔧 Treasury audit remediation (3 CRITICAL, 5 HIGH — see `docs/SECURITY_AUDIT_2026-04-07.md`)
- ⬜ Python ↔ Rust typed bridge (`bridge.rs`)
- ⬜ Trading Wing beyond stub

### Phase 2: All Wings Functional + Full Loop

Wire remaining wings and complete the end-to-end demo flow.

- Knowledge Wing (in-memory store + cross-wing queries)
- Security Wing (threat detection + rate limiting)
- Trading Wing executor (bridge.rs → Python binary → typed JSON proposals)
- Full loop: Python proposes → Audit tribunal → Coordinator routes → execute on Solana

### Phase 3: Polish + Submission

Demo rehearsal, video, hardening, Colosseum submission by May 11.

### Phase 4: Eternal Autonomy

All wings operational. Human role reduced to soulcontract amendments. The swarm runs, improves, defends, and evolves — funded by its own yield.

## Quick Start

```bash
# Set up environment
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt

# Run the fractal-swarm (night shift)
python scripts/night_shift.py --skip-fetch

# Live paper trading
PYTHONUNBUFFERED=1 python scripts/paper_trader.py

# Validate candidates through full simulator
python scripts/validate_night_shift.py --production

# Check system calibration
python scripts/evaluator_calibration.py --samples 20
```

## Hackathon

**SWARM — Canteen × Colosseum**

| Criterion | Weight | What Judges Want | How RTP Delivers |
|---|---|---|---|
| **Functionality** | ? | Working demo with real transactions | Live: adopt→fees→swarm→yield→redistribute on devnet |
| **Potential Impact** | ? | Project with lasting real-world value | Any Solana token can adopt — unruggable yield standard |
| **Novelty** | ? | Novel approach, original architecture | Six-wing modular swarm + token adoption model |
| **UX** | ? | Great demo experience | Phantom Connect + CASH wallet flows, 3-min demo |
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
| MoonPay Agents | Sponsored | Agent money movement infrastructure |

## License

MIT
