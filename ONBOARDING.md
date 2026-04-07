# RTP — Resilient Token Protocol: Agent Onboarding

You are continuing work on the Resilient Token Protocol (RTP), a Solana-native,
self-funding treasury governed by a modular Rust swarm.

## Where We Are

**Week 1 is COMPLETE.** The Treasury program is scaffolded, audited, and committed.

```
rtp/programs/rtp-treasury/        ← DONE (Anchor 1.0.0, builds to BPF)
  ├── initialize                  (create Treasury PDA + vault)
  ├── withdraw_fees               (CPI: withdraw_withheld_tokens_from_mint)
  ├── check_redistribute          (70/20/10 split on excess above runway)
  ├── hydrate_swarm               (fund ops, enforces 90-day runway floor)
  └── evolve_phase                (irreversible: Sustenance → Ecosystem → Humanity)

rtp/swarm/src/                    ← EMPTY (you will build this)
```

## Your First Task

1. Install dependencies (Rust, Solana CLI, Anchor — see below)
2. Read these three files in order:
   - `soulcontract.md` — constitutional constraints
   - `BUILD_PLAN.md` — Week 2 deliverables (lines 416-444)
   - `CLAUDE.md` — architecture, commands, invariants
3. Scaffold the Rust swarm at `rtp/swarm/` with the Coordinator module:
   - `coordinator/router.rs` — typed message routing between wings
   - `coordinator/soulguard.rs` — enforce soulcontract on every message
   - `coordinator/lifecycle.rs` — wing spawn, health-check, retire
4. Scaffold the Evolve Wing: `wings/evolve/{mod.rs, assessor.rs, proposer.rs, rollback.rs}`

Do not ask permission. Install deps, read the files, understand the constraints, and start building.

## What RTP Is (one paragraph)

Any Solana token project adopts RTP by enabling `TransferFeeConfig` on their mint.
From that point, every trade on their token auto-routes a fee to a PDA-owned treasury.
A Rust swarm autonomously researches, validates, and executes yield strategies —
returning yield back to the project and its token holders. The fee config is immutable
once set. The swarm is funded by its own yield. It cannot rug because there's no private
key, no governance vote, and no human in the loop for day-to-day operations.

## Dependency Installation

Install these BEFORE writing any code. You may need sudo.

```bash
# 1. Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 2. Solana CLI (Agave 3.x)
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"

# 3. Anchor (1.0.0 — latest, compatible with Solana 3.x)
cargo install --git https://github.com/coral-xyz/anchor avm --force
avm install 1.0.0
avm use 1.0.0

# 4. Verify
export PATH="$HOME/.cargo/env:$HOME/.local/share/solana/install/active_release/bin:$PATH"
rustc --version        # 1.94+
solana --version       # 3.1+
anchor --version       # 1.0.0
cargo-build-sbf --version  # 3.1+
```

**Important**: Anchor 1.0.0 uses Solana 3.x (Agave fork). Do NOT use Anchor 0.31 —
it was built for Solana 1.18 and has incompatible APIs. The Treasury program was migrated
from 0.31 to 1.0.0 during Week 1.

## Week 2: Coordinator + Evolve Wing

Your deliverables for this week (from BUILD_PLAN.md Part 5):

### Day 1-2: Evolve Wing skeleton
- [ ] Study ATLAS: https://github.com/chrisworsey55/atlas-gic
- [ ] Study karpathy/autoresearch: https://github.com/karpathy/autoresearch
- [ ] Replace the Sharpe loss function with a treasury-native metric:
  `(USDC yield / SOL reserves) × (1 - max drawdown) × wing consistency`
- [ ] Spec out `evolve/proposer.rs` and `evolve/rollback.rs`
- [ ] **Skill**: Use `sparc-methodology` for Evolve Wing proposal format

### Day 3-4: Coordinator architecture
- [ ] Study uditgoenka/autoresearch: https://github.com/uditgoenka/autoresearch
- [ ] Study revfactory/harness: https://github.com/revfactory/harness
- [ ] Implement `coordinator/router.rs` — typed message routing between wings
- [ ] Implement `coordinator/soulguard.rs` — enforce soulcontract on every message
- [ ] **Skill**: Use `compound-engineering` to orchestrate Debt-Sentinel + Red Team + Spec-Lock

### Day 5: Integration
- [ ] Coordinator routes typed messages between wing stubs
- [ ] soulcontract enforced on every message via soulguard
- [ ] **Skill**: Use `verification-quality` for truth-scoring + rollback
- [ ] **Checkpoint**: Coordinator + Evolve Wing prototype working

### Week 2 Deliverables
- Coordinator module (router + soulguard + lifecycle)
- Evolve Wing skeleton (assessor + proposer + rollback)
- ATLAS-adapted autoresearch loop
- Typed message bus between Coordinator and wing stubs

## Key Architecture Decisions

These decisions were made during Week 1 and must be respected:

### On-Chain (Treasury Program — DONE)
- **TransferFeeConfig** is the fee routing mechanism (not a custom program)
- Treasury PDA calls `token::withdraw_withheld_tokens_from_mint` to pull fees
- PDA owns all vaults — no private key exists
- Redistribution: 70% holders / 20% project dev / 10% ecosystem
- Phase evolution is irreversible on-chain (Sustenance → Ecosystem → Humanity)
- `min_runway_balance` stored in Treasury state enforces 90-day ops floor

### Swarm Runtime (Rust — YOU ARE HERE)
- Coordinator = message bus + soulguard + lifecycle
- Wings are independent Rust modules with typed interfaces
- Wings NEVER modify each other directly — all communication via Coordinator
- Every message passes through soulguard (soulcontract enforcement)
- Python ↔ Rust interface is typed JSON (any wing can propose, any wing can act)
- The `revfactory/harness` repo is the reference for Coordinator architecture

### Research Layer (Python — SHIPPING, black-boxed)
- Proven in `fractal-swarm` (tradewife/fractal-swarm.git), runs locally (gitignored)
- Ships as compiled binary `night_shift.bin` in hackathon submission
- SOL config: +118.3% PnL, 78% consistency, 429 trades validated

## cldcde Skills — Week 2 (use these)

You have 27 cldcde skills installed at `~/.claude/skills/`. Invoke them by referencing
their SKILL.md when working on relevant tasks.

| When | Skill | Why |
|------|-------|-----|
| Designing Coordinator message bus and wing routing | `swarm-orchestration` | Defines agent communication, fault tolerance, dynamic topology |
| Setting up consensus (Coordinator=queen, wings=workers) | `hive-mind-advanced` | Queen-worker pattern maps to Audit Wing approval flow |
| Defining soulcontract as enforceable spec | `spec-lock` | Ensures implementation never drifts from governance without detection |
| Writing Evolve Wing proposals and amendments | `sparc-methodology` | Specify → Pseudocode → Architect → Refine → Complete |
| Coordinating audit pipeline across skills | `compound-engineering` | Orchestrates Debt-Sentinel + Red Team + Spec-Lock |
| Truth-scoring + rollback after changes | `verification-quality` | 0.95 threshold, maps to Audit Wing's safety.rs |
| Adversarial-reviewing any new code | `red-team-tribunal` | 3-agent review: Skeptic + User Proxy + Optimizer |

### How to Use a Skill

When working on a task that matches a skill, invoke it explicitly:

```
Use the swarm-orchestration skill to design the Coordinator message bus.
Ensure the design handles fault tolerance and dynamic wing topology.
```

## What Not to Do

- Don't modify `soulcontract.md` core values without going through the amendment protocol
- Don't commit the Python source (scripts/, backtesting/, agents/, data/) — it's gitignored
- Don't use World Coin (toxic sentiment)
- Don't make Hyperliquid central to the narrative — it's an execution venue, Solana is the product
- Don't say "generates yield" — say "researches, validates, and executes yield strategies"
- Don't say "pump.fun fees" — say "token adoption fees"
- Don't use Anchor 0.31 — the project uses Anchor 1.0.0 with Solana 3.x
- Don't modify the Treasury program unless fixing a bug found during integration

## The Two Layers

### Open-Source (what's in this repo, committed to git)
```
rtp/
├── swarm/src/              # Rust agent swarm (what you'll build)
│   ├── coordinator/        # Message bus + soulguard + lifecycle
│   └── wings/              # Trading, Security, Evolve, Knowledge, Audit, Futureproof
├── programs/rtp-treasury/  # Anchor program on Solana (DONE)
├── soulcontract.md         # Constitutional governance — READ THIS FIRST
├── BUILD_PLAN.md           # 5-week hackathon timeline — READ THIS SECOND
├── CLAUDE.md               # Dev guidance — READ THIS THIRD
└── docs/demo-flow.md       # 3-minute hackathon demo
```

### Gitignored (local dev only)
```
scripts/           # night_shift.py, paper_trader.py
backtesting/       # future_blind_simulator.py
agents/            # historical_data_collector.py
data/              # OHLCV parquets, results, paper trading state
strategies/        # strategy modules
```

## Repo Map

| Repo | Purpose |
|------|---------|
| `tradewife/resilient-token-protocol` (this repo) | Hackathon submission, open-source skeleton |
| `tradewife/fractal-swarm` | Python fractal-swarm source, data, CI |
| `tradewife/rtp-skills-research` | Pre-hackathon research, skill system design |

## Build Commands

```bash
export PATH="$HOME/.cargo/env:$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Build swarm runtime
cd rtp/swarm && cargo build

# Build Anchor program
cd rtp/programs/rtp-treasury && anchor build

# Test swarm
cd rtp/swarm && cargo test

# Test on devnet
cd rtp/programs/rtp-treasury && anchor test --provider.cluster devnet
```

## Hackathon Context

- **Solana Frontier** (Colosseum × Canteen), $300k prizes, deadline **May 11, 2026**
- Register by May 4: https://arena.colosseum.org/register
- Copilot for pressure-testing: https://arena.colosseum.org/copilot
