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

Any launch platform integrates RTP — one function call per token launch. Transfer fees route to a program-owned treasury vault. A cron-driven autonomous agent swarm researches, validates, and executes yield strategies (open/close Flash Trade positions via on-chain CPI), returning yield to the project and holders, forever. There is no RTP token. RTP is infrastructure.

## Why This Is Different

Prior hackathon projects built individual components — treasury managers, AI agents, yield aggregators, backtesting tools. RTP is the first to combine them:

- **Constitutional governance** — soulcontract enforced in Rust (soulguard.rs) AND on-chain (Anchor `require!`). No other project has both.
- **Self-funding economics** — treasury generates its own yield via Flash Trade on-chain perps (CPI via `invoke_signed`), with irreversible phase evolution (Sustenance → Ecosystem → Humanity). No VC dependency.
- **Proven research engine** — 30K configs/night, 9-fold walk-forward validation, fee-aware simulation. Not a backtest screenshot — out-of-sample results across 9 independent time windows.
- **On-chain constraint proof** — the Anchor program deliberately rejects invalid transactions (10+ rejection tests). Constraint rejection IS the demo.
- **Flash Trade on-chain execution** — PDA-signed CPI into Flash Trade's Perpetuals program on Solana. No human keypair. No cross-chain bridge. Fully auditable on Explorer. Mainnet-proven (M1: TX `2bLg1Fu...`, 99,214 CU).

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
| SOL/USDT | +36.9% | **+118.3%** | 78% → **100%** (optimized) | 429 |
| BNB/USDT | +49.6% | — | 67% | 178 |
| ETH/USDT | +48.1% | — | 78% | 155 |
| BTC/USDT | +17.5% | — | 67% | 153 |

*Production = baseline config; Optimized = Survivor 2.69 config (9/9 folds profitable, OOS Sharpe +3.96).*

This is not a backtest screenshot. These are out-of-sample walk-forward results through a fee-aware simulator with 429 real trades across 9 independent time windows.

## Quick Demo

```bash
# Run the full 3-layer demo (dry-run by default)
npx tsx cli/bin/rtp.ts demo

# Run with actual transactions
npx tsx cli/bin/rtp.ts demo --execute

# Legacy shell script (archived to scripts/archive/)
# ./demo.sh
```

See [docs/demo-flow.md](docs/demo-flow.md) for the 3-minute hackathon demo script.

## SDK — One Function Call

```bash
npm install @resilient-protocol/sdk @solana/web3.js @solana/spl-token @coral-xyz/anchor
```

```typescript
import { registerWithRTP } from "@resilient-protocol/sdk";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";

const connection = new Connection("https://api.devnet.solana.com");
const payer = Keypair.generate(); // or use a WalletAdapter from @solana/wallet-adapter-react

const result = await registerWithRTP(connection, payer, {
  mint: new PublicKey("YourExistingMintAddress"),
  platform: "pumpfun",
  name: "Community Token",
  symbol: "CMTY",
});

// result.mint, result.treasuryPDA, result.vaultPDA, result.adopterPDA
```

For browser wallets (e.g. Phantom), pass the wallet adapter directly — no keypair needed:
```typescript
import { useWallet, useConnection } from "@solana/wallet-adapter-react";

const { publicKey, signTransaction } = useWallet();
const { connection } = useConnection();

const result = await registerWithRTP(connection, { publicKey, signTransaction }, config);
```

Core functions: `registerWithRTP()`, `fetchTreasuryState()`, `withdrawAndRedistribute()`, `registerAdopterBeta()`, `fetchAdopterState()`. See [sdk/README.md](sdk/README.md) for details.

## Operator CLI

A unified command-line tool for protocol operators. Consolidates all operational scripts (`fee-crank`, `promote-strategy`, `emergency-freeze`, account derivation) into a single Commander.js CLI.

```bash
# Interactive setup (first time)
npx tsx cli/bin/rtp.ts init

# Derive PDAs (offline, no RPC)
npx tsx cli/bin/rtp.ts accounts derive --mint <MINT_PUBKEY>

# Sweep fees into treasury vault
npx tsx cli/bin/rtp.ts crank fees --mint <MINT_PUBKEY>

# Emergency freeze (authority-gated, requires --yes)
npx tsx cli/bin/rtp.ts freeze --mint <PUBKEY> --authority <KEYPAIR> --yes

# Protocol health overview
npx tsx cli/bin/rtp.ts status --all

# Railway service status
npx tsx cli/bin/rtp.ts status services
```

14 commands across 7 groups: `init`, `deploy`, `register`, `crank`, `strategy`, `freeze`/`unfreeze`, `accounts`, `status`, `demo`. All support `--json`, `--quiet`, `--cluster <devnet|mainnet>`. See [cli/README.md](cli/README.md) for the full reference.

## Live on Devnet

Treasury program deployed and operational on Solana devnet (Apr 11 2026).

| Item | Value |
|------|-------|
| Program ID | `8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB` |
| Treasury PDA | Per-mint — demo: `7oZTJWYBDjzqmbfRs5YkTv53CDa6vESAzfyjK3yhYshc` |
| Explorer | [View demo treasury](https://explorer.solana.com/address/7oZTJWYBDjzqmbfRs5YkTv53CDa6vESAzfyjK3yhYshc?cluster=devnet) |
| Init tx | [View transaction](https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet) |

8/8 on-chain steps completed including live redistribution (70/20/10 split).

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│                                                                 │
│  RTP Treasury Program                                           │
│  ├── Receive fees — TransferFeeConfig (withheld) + platform creator fees (SOL)  │
│  ├── Strategy lifecycle (register → update → suspend/retire)    │
│  ├── Hydration gate (only Live strategies receive funding)      │
│  ├── Flash Trade CPI: invoke_signed → open/close perps positions │
│  ├── Threshold-triggered redistribution (70/20/10 split)        │
│  ├── Self-hydration CPI (fund swarm ops from yield)            │
│  ├── Ecosystem auto-invest (excess → top RTP token LPs)        │
│  └── Phase evolution (sustenance → ecosystem → humanity fund)   │
│                                                                 │
│  Invariants (enforced on-chain):                                │
│  ├── PDA owns treasury (no private key risk)                    │
│  ├── Per-token isolation — each mint gets its own PDA + vault   │
│  ├── SPL TransferFeeConfig (fee % immutable from mint)          │
│  ├── CPI-only transfers (atomic, verifiable)                    │
│  ├── Flash Trade CPI-only execution (invoke_signed, no human key)│
│  ├── SOL never liquidated — committed via Composability, not sold│
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
│  ├── Knowledge Wing  — persistent knowledge store               |
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
| Live execution on Flash Trade (mainnet CPI) | Rust + Solana | **Done** — Treasury PDA invoke_signed, positions opened/closed on-chain, mainnet-proven |
| Degradation detection + auto-recalibration trigger | Rust | Planned |
| Strategy lifecycle (hypothesis → validate → deploy → retire) | Both | **Built** — PromotionGate + RetirementGate + DecayMonitor (7 tests) |
| On-chain lifecycle enforcement (StrategyRecord PDA) | Solana | **Built** — register/update/retire instructions, hydrate gate (17 tests) |
| Flash Trade CPI instructions (open/close/emergency) | Solana | **Built** — 9/9 CPI tests, 3 instruction handlers, PDA-signed |

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

Builds and maintains a persistent knowledge store spanning every aspect of the project — market data, strategy performance, security events, architectural decisions, and external research. Knowledge is serialized to disk after every write, surviving process restarts and enabling cross-cycle recall.

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

The whole pipeline runs autonomously via Railway cron. No human in the loop until deployment.

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

## Treasury Capital Model — On-Chain SOL Cycle

The protocol operates on a single transparent cycle: **SOL in, Flash Trade CPI on Solana, SOL back out.** No cross-chain bridge. No USDC in-flight. Everything stays on Solana.

```
SOL fees → Treasury PDA → invoke_signed → Flash Trade CPI (on-chain perps) → SOL returned → Treasury PDA → redistribute
```

| Step | Asset | Location | Mechanism |
|------|-------|----------|-----------|
| 1. Fees arrive | SOL | Treasury PDA (Solana) | Platform creator fees (Pump.fun, Bags.fm, Raydium) → treasury PDA |
| 2. Execute strategies | SOL | Flash Trade (on-chain CPI) | Treasury PDA invoke_signed → Composability swap-and-open → perps position on Solana |
| 3. Yield returns | SOL | Treasury PDA (Solana) | Close position → SOL returned to treasury vault via CPI |
| 4. Redistribute | SOL | Treasury PDA | 70% holders / 20% dev / 10% ecosystem (on-chain) |

**Why this model:**
- **Single asset, single chain** — the treasury PDA holds SOL, Flash Trade positions are on Solana, yield returns as SOL. No bridge. No USDC. No cross-chain risk.
- **PDA-signed execution** — the Treasury PDA signs via `invoke_signed`. No human keypair exists for trading. The program IS the only authority.
- **Fully auditable** — every position open/close is an on-chain transaction visible on Solana Explorer.

> **Devnet note:** Flash Trade uses Pyth oracles which are mainnet-only. CPI execution works on mainnet with micro positions (~$11 USDC minimum). Constraint logic tests (frozen, strategy gate, position limits) run on local validator. See `FLASHTRADE-PDA-UPGRADE-SPEC.md` for M1 mainnet proofs.

**Self-sustaining threshold:** when SOL yield exceeds ops cost by 10x sustained over 90 days, external seed capital is no longer required and can be returned or recycled to the ecosystem fund.

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

Any Solana token project can adopt RTP by enabling `TransferFeeConfig` on their mint and setting the Treasury PDA as the fee recipient. The fee percentage and withdraw authority are immutable once set on the mint — but the mechanism for routing platform creator fees (SOL) to the treasury varies:

- **Pump.fun**: one-time post-launch fee redirect to treasury PDA
- **Bags.fm**: multi-claimer fee sharing, updateable anytime via API
- **Raydium**: creator fees go to pool_creator wallet, forwarded to RTP

```
Token project adopts RTP
  │
  ├── Enable TransferFeeConfig on mint (fee % + withdraw authority immutable)
  │       └── Every trade → withheld fee → Treasury PDA vault
  │
  ├── pump.fun (most common)
  │       └── 0.25% PumpSwap fee → 0.05% creator fee (SOL) → Treasury PDA (one-time redirect)
  │
  ├── Bags.fm
  │       └── Multi-claimer fee sharing → Treasury PDA (updateable anytime)
  │
  └── Raydium
          └── Creator fees → pool_creator wallet → forward to Treasury PDA (manual)

SOL reserves held in treasury PDA. Treasury PDA invoke_signed → Flash Trade CPI
opens positions on Solana. SOL returned on position close. Single asset, single chain, fully auditable.
```

**Why projects adopt**: Their fees don't just sit in a wallet — the swarm puts them to work. Yield flows back to the project and its holders automatically. No trust required. The community's SOL is never sold.

**Rug-proof by design**: SPL TransferFeeConfig fee percentage is immutable once minted. PDA owns treasury (no private key). All transfers via CPI (atomic, verifiable). Platform fee routing is separate — Pump.fun allows one redirect, Bags.fm is updateable, Raydium is manual.

### Per-Token Isolation — No Shared Pool, No Honeypot

Every token that adopts RTP gets its **own isolated treasury** — a separate PDA, vault, and adopter record. There is no shared pool. One token's trading loss cannot affect another's reserves.

```
Token A adopts RTP          Token B adopts RTP          Token C adopts RTP
      │                           │                           │
Treasury PDA_A              Treasury PDA_B              Treasury PDA_C
Vault PDA_A                 Vault PDA_B                 Vault PDA_C
AdopterRecord_A             AdopterRecord_B             AdopterRecord_C
      │                           │                           │
  SOL fees_A                 SOL fees_B                 SOL fees_C
      │                           │                           │
  Flash Trade CPI_A         Flash Trade CPI_B         Flash Trade CPI_C
  (invoke_signed,            (invoke_signed,            (invoke_signed,
   isolated capital)          isolated capital)          isolated capital)
      │                           │                           │
  Yield_A → SOL_A            Yield_B → SOL_B            Yield_C → SOL_C
      │                           │                           │
  70/20/10 split_A           70/20/10 split_B           70/20/10 split_C
```

**On-chain proof:** `register_adopter` and `record_fee_deposit` instructions are live on devnet. Each token's `AdopterRecord` PDA (`seeds: ["adopter", token_mint]`) tracks its own fees independently. The `Treasury` PDA (`seeds: ["treasury", mint]`) is per-mint. See `scripts/compute_adopter_yield_share.ts` for the attribution formula.

**Why isolated treasuries, not a shared pool:**
- **Exploit isolation** — if one token's strategy fails or is exploited, only that token's treasury is affected. Other tokens are untouched.
- **No honeypot** — aggregating many tokens' fees into one pool creates a high-value target. Per-token PDAs distribute risk.
- **Transparent attribution** — each token's yield is directly verifiable on its own PDA via Solana Explorer. No pro-rata math required.
- **No cross-subsidization** — Token A's losses cannot eat Token B's reserves.

**The swarm copy-trades the same validated strategy across all tokens.** One research engine (Night Shift, 30K configs) discovers the optimal config. One Coordinator dispatches it. Each token's capital executes independently — same strategy, isolated execution. Each token's Treasury PDA signs its own Flash Trade CPI via `invoke_signed`. This is the production architecture for multi-token scaling.

**Phase 1 (current demo):** Single adopter, single treasury PDA, full redistribution cycle proven on devnet.
**Phase 2 (production scaling):** Per-token copy-trading dispatcher — the Trading Wing iterates over all registered adopters, executing the validated strategy for each token's isolated capital.

## Capital Flow

```
                     ┌────────────────────────────┐
                     │ TOKEN PROJECT TRADING FEES  │
                     │ (pump.fun, or any token)    │
                     └──────────┬─────────────────┘
                                │ Creator fees (SOL) — platform-dependent routing
                                ▼
                     ┌──────────▼─────────────────┐
                     │     SOLANA TREASURY PDA     │
                     │                             │
                     │  SOL reserves               │
                     │  ├─ Never liquidated        │
                     │  └─ Committed via invoke_   │
                     │       signed to Flash Trade  │
                     └──────────┬─────────────────┘
                                │ Treasury PDA invoke_signed (no human key)
                                ▼
                     ┌──────────▼─────────────────┐
                     │ FLASH TRADE PERPETUALS      │
                     │ (on-chain Solana CPI)       │
                     │                             │
                     │  SOL input via Composability│
                     │  ├─ swap-and-open (atomic)  │
                     │  ├─ Pool-to-peer perps      │
                     │  └─ Up to 100x leverage     │
                     └──────────┬─────────────────┘
                                │ SOL returned on close (via CPI)
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

Single asset throughout (SOL). Single chain (Solana). No cross-chain bridge. PDA-signed execution. Judges verify the full SOL balance and all Flash Trade positions on Solana Explorer.

## What This Is Not

- **Not a token** — there is no RTP token. RTP is pure infrastructure that serves the tokens that adopt it
- **Not a vault** — no custody of user funds, no withdrawal interface
- **Not dependent on LLMs** — core loop is deterministic Python; LLMs optional for hypothesis generation
- **Not just a trading bot** — it's infrastructure that any launch platform can integrate
- **Not requiring venture infrastructure** — runs on a single machine, no database, no Kubernetes
- **Not liquidating community SOL** — SOL is committed to Flash Trade positions via on-chain CPI (Composability swap-and-open), never sold on the open market. Positions are on Solana, fully auditable.

## What We Already Have (Proven)

The Trading Wing's research layer is shipping today. Everything else is scaffolding.

| Component | Wing | Layer | Status |
|-----------|------|-------|--------|
| Night Shift Optimizer (30K configs → WFA → Darwinian) | Trading | Python | **Shipping** |
| Full Simulator (0.1% fees, 10bps slippage, ground truth) | Trading | Python | **Shipping** |
| Paper Trader (live Binance, ADX filter, state persistence) | Trading | Python | **Shipping** |
| Self-Correction (fast sim vs full sim calibration) | Trading | Python | **Shipping** |
| CI Pipeline (nightly cron, auto-commit, 300min timeout) | Trading | Infra | **Shipping** |
| Devnet Loop (6h cron, real chain execution, LLM mutations, config chaining) | Evolve | Infra | **Shipping** |
| SOL Optimized Config (+118.3% PnL, 78% consistency, 429 trades) | Trading | Python | **Shipping** |
| Treasury Program (Anchor: deposit, distribute, hydrate, evolve, Flash Trade CPI) | — | Solana | **Built** (audit remediated, M0–M5 complete) |
| soulcontract.md (constitutional governance layer) | — | Governance | **Defined** |
| Python ↔ Rust Bridge (typed JSON, bridge-mode subprocess) | Trading | Both | **Built** |
| Coordinator (soulguard + router + lifecycle) | — | Rust | **Built** (312 tests) |
| Evolve Wing (assessor + proposer + rollback) | Evolve | Rust | **Built** |
| Audit Wing (3-agent tribunal, Byzantine consensus) | Audit | Rust | **Built** |
| Trading Wing (Flash Trade CPI execution, REST API client, in-memory state) | Trading | Rust | **Built** |
| Security Wing (threat detection, rate-limiting, alert tracking) | Security | Rust | **Built** |
| Knowledge Wing (in-memory graph, cross-wing queries) | Knowledge | Rust | **Built** |
| Future-proof Wing (deprecation monitoring, heartbeat) | Future-proof | Rust | **Built** |
| Operator CLI (14 commands, onboarding wizard, demo, status, Railway) | — | TypeScript | **Built** |

## Project Structure

```
rtp/
├── swarm/                           # Rust swarm runtime
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                  # Re-exports, module declarations
│   │   ├── types.rs                # Message, Payload, WingId, Priority
│   │   ├── bridge.rs               # Python ↔ Rust typed subprocess interface
│   │   ├── chain_client.rs          # On-chain client — ChainConfig, ExecutionMode, open/close builders, submit/simulate
│   │   ├── config.rs               # Swarm configuration
│   │   ├── demo.rs                 # End-to-end demo loop (8-step pipeline)
│   │   ├── coordinator/
│   │   │   ├── mod.rs              # Multi-stage quality gate pipeline
│   │   │   ├── router.rs           # Typed routing, retry, proposal→audit flow
│   │   │   ├── soulguard.rs        # Enforce soulcontract on every message
│   │   │   ├── soulcontract_spec.rs # Parse soulcontract.md → constraints
│   │   │   └── lifecycle.rs        # Wing spawn, health-check, retire
│   │   └── wings/
│   │       ├── trading/mod.rs      # Flash Trade CPI execution, REST client, in-memory state
│   │       ├── security/mod.rs     # Threat detection, rate-limiting, alerts
│   │       ├── evolve/
│   │       │   ├── mod.rs          # Darwinian loop orchestration + LLM proposer
│   │       │   ├── assessor.rs     # Treasury-native performance scoring
│   │       │   ├── proposer.rs     # SPARC-inspired proposal lifecycle
│   │       │   └── rollback.rs     # Auto-revert on >5% degradation
│   │       ├── knowledge/mod.rs    # Persistent knowledge store (JSON file-backed), cross-wing queries
│   │       ├── audit/mod.rs        # 3-agent tribunal (Byzantine consensus)
│   │       └── futureproof/mod.rs  # Deprecation monitoring, heartbeat
│   └── src/bin/
│       ├── demo.rs                 # One-shot demo binary (5 judge points)
│       └── daemon.rs               # Autonomous devnet loop (real chain execution, 6h CI cron)
│
├── programs/                        # Solana (Anchor)
│   └── rtp-treasury/              # Deposit, distribute, hydrate, evolve, strategy lifecycle
│
├── cli/                             # Operator CLI (TypeScript / Commander.js)
│   ├── bin/rtp.ts                  # Entry point
│   ├── src/commands/               # 14 commands across 7 groups
│   ├── src/config.ts               # Config loading (~/.rtp/config.json)
│   ├── src/lib/                    # RPC, Railway, safety helpers
│   └── tests/                      # Unit tests (config, keypair, format)
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
- ✅ Knowledge Wing (persistent knowledge store, file-backed, cross-wing queries)
- ✅ Futureproof Wing (deprecation monitoring, heartbeat)
- ✅ Treasury program on devnet (Anchor 1.0, audit remediated)
- ✅ Python ↔ Rust typed bridge (`rtp/swarm/src/bridge.rs`)
- ✅ End-to-end demo loop (`rtp/swarm/src/demo.rs`, 8-step pipeline)
- ✅ Autonomous devnet loop (`rtp-daemon` binary, real chain execution via chain_client.rs, 6h CI cron, LLM mutations)
- ✅ Test suite: 312 tests, 0 failures, 0 clippy warnings (anchor: 32 passing, 9/9 Flash Trade CPI tests).

### Phase 2: End-to-End Integration + Full Loop

Wire remaining wings and complete the end-to-end demo flow.

- Sentinel dashboard deployed to GitHub Pages (live URL for judges)
- Phantom SDK "Fund Treasury" button (SOL → USDC swap via Jupiter routing)
- Knowledge Wing hardening beyond in-memory store
- Security Wing hardening beyond in-memory alert/rate-limit logic

### Phase 3: Polish + Submission

Demo rehearsal, video, hardening, Colosseum submission by May 11.

### Phase 4: Eternal Autonomy

All wings operational. Human role reduced to soulcontract amendments. The swarm runs, improves, defends, and evolves — funded by its own yield. Flash Trade CPI loop operational: Treasury PDA signs positions on-chain, yield returns as SOL, redistribution runs at threshold.

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

# Operator CLI — full 8-step demo (replaces demo.sh)
npx tsx cli/bin/rtp.ts demo

# Operator CLI — interactive setup
npx tsx cli/bin/rtp.ts init

# Operator CLI — derive PDAs, check status, emergency freeze
npx tsx cli/bin/rtp.ts accounts derive --mint <PUBKEY>
npx tsx cli/bin/rtp.ts status --all
```

## Hackathon

**SWARM — Canteen × Colosseum**

| Criterion | Weight | What Judges Want | How RTP Delivers |
|---|---|---|---|
| **Functionality** | ? | Working demo with real transactions | Live: adopt→fees→swarm→Flash Trade CPI→yield→redistribute on mainnet |
| **Potential Impact** | ? | Project with lasting real-world value | Any Solana token can adopt — unruggable yield standard |
| **Novelty** | ? | Novel approach, original architecture | Six-wing modular swarm + token adoption model |
| **UX** | ? | Great demo experience | Phantom Connect + MCP server (agentic wallet), devnet treasury live, 3-min demo |
| **Open-source** | ? | Clean, well-documented repo | Full swarm arch + treasury program (MIT), clean repo history |
| **Business Plan** | ? | Viable business model | Adoption fees → self-funding swarm → yield back to holders |

### Demo Flow (3 minutes)

1. "A token project adopts RTP — creator fees (SOL) route to a per-mint treasury PDA"
2. "This is our night shift — it tested 30,000 strategy configs last night, fully autonomous"
3. "Here's the best one — +118% PnL, 78% consistency, 9 independent validation folds"
4. "The Trading Wing proposes deployment — the Audit Wing checks it against the soulcontract"
5. "Approved — the Treasury PDA signs a Flash Trade CPI call via invoke_signed, the position opens on Solana, yield flows back to the project and holders"
6. "At threshold, it auto-redistributes — 70% holders, 20% project dev, 10% ecosystem — all on-chain"

## Third-Party Components

| Component | License | Use |
|-----------|---------|-----|
| atlas-gic | MIT | Multi-agent Darwinian loop — Evolve Wing autoresearch |
| karpathy/autoresearch | MIT | Core Modify/Verify/Keep loop specification |
| uditgoenka/autoresearch | MIT | Claude-native implementation |
| Phantom Connect | Open-source | Browser wallet for dashboard (freeze/unfreeze, wallet connect). MCP server archived behind feature flag. |
| Flash Trade | Open-source program | On-chain Solana perps DEX — execution venue. CPI via invoke_signed, REST API for queries. |
| CASH | Third-party | Stablecoin (not currently used) |
| Squads Multisig | Sponsored | Treasury PDA security + multisig authority |
| Swig | Sponsored | Programmable smart wallets for wing message bus |
| MoonPay Agents | Sponsored | VC capital on-ramp → treasury USDC deposit |

## License

MIT
