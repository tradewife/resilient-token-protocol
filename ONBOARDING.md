# RTP — Resilient Token Protocol: Agent Onboarding

You are starting work on the Resilient Token Protocol (RTP), a Solana-native, self-funding treasury governed by a modular Rust swarm.

## Your First Task

Read this file for full context, then read `soulcontract.md`, `BUILD_PLAN.md`, and `CLAUDE.md` in that order. After reading all three, scaffold the Treasury program at `rtp/programs/rtp-treasury/` with the four Anchor instructions: `withdraw_fees`, `check_redistribute`, `hydrate_swarm`, `evolve_phase`. Start with the Anchor project init and the Treasury state account struct.

Do not ask permission. Read the files, understand the constraints, and start building.

## What RTP Is (in one paragraph)

Any Solana token project adopts RTP by enabling `TransferFeeConfig` on their mint. From that point, every trade on their token auto-routes a fee to a PDA-owned treasury. A Rust swarm autonomously researches, validates, and executes yield strategies — returning yield back to the project and its token holders. The fee config is immutable once set. The swarm is funded by its own yield. It cannot rug because there's no private key, no governance vote, and no human in the loop for day-to-day operations.

## The Two Layers You'll Work With

### 1. Open-Source (what's in this repo, committed to git)

```
rtp/
├── swarm/src/              # Rust agent swarm (what you'll build)
│   ├── coordinator/        # Message bus + soulguard + lifecycle
│   └── wings/              # Trading, Security, Evolve, Knowledge, Audit, Futureproof
├── programs/rtp-treasury/  # Anchor program on Solana (what you'll build)
├── soulcontract.md         # Constitutional governance — READ THIS FIRST
├── BUILD_PLAN.md           # 5-week hackathon timeline — READ THIS SECOND
├── CLAUDE.md               # Dev guidance — READ THIS THIRD
└── docs/demo-flow.md       # 3-minute hackathon demo
```

### 2. Gitignored (local dev only, NOT in git)

The Python research engine called "fractal-swarm" lives in this directory but is gitignored. It ships as a compiled binary. Source: `tradewife/fractal-swarm.git`.

```
scripts/           # night_shift.py, paper_trader.py, etc.
backtesting/       # future_blind_simulator.py
agents/            # historical_data_collector.py
data/              # OHLCV parquets, results, paper trading state
strategies/        # strategy modules
```

This exists locally so you can run the research pipeline during development. The hackathon submission ships the binary, not the source.

## Where to Start

**Read these three files in order:**

1. `soulcontract.md` — the constitutional constraints everything must obey
2. `BUILD_PLAN.md` — the 5-week plan, especially Week 1 deliverables
3. `CLAUDE.md` — architecture, commands, invariants, design decisions

**Then start with Week 1 from BUILD_PLAN.md:**

The first deliverable is a Treasury program on devnet that can:
1. Receive fees via TransferFeeConfig (token adopts RTP)
2. Withdraw accumulated fees via `token::withdraw_withheld_tokens_from_mint` CPI
3. Check redistribution threshold
4. Execute threshold-triggered split (70% holders / 20% project dev / 10% ecosystem)

The Anchor IDL should have: `withdraw_fees`, `check_redistribute`, `hydrate_swarm`, `evolve_phase`.

## Key Technical Decisions Already Made

- **TransferFeeConfig** is the fee routing mechanism (not a custom program)
- Treasury program calls `token::withdraw_withheld_tokens_from_mint` to pull fees
- PDA owns the treasury — no private key exists
- pump.fun is the most common adoption path but any Solana token can adopt
- SOL fees get converted to USDC for strategy execution
- Redistribution split: 70% holders / 20% project dev / 10% ecosystem
- Phase evolution (Sustenance → Ecosystem → Humanity) is irreversible on-chain
- Squads Multisig secures the PDA upgrade authority

## What Not to Do

- Don't modify `soulcontract.md` core values without going through the amendment protocol
- Don't commit the Python source (scripts/, backtesting/, agents/, data/) — it's gitignored
- Don't use World Coin (toxic sentiment)
- Don't make Hyperliquid central to the narrative — it's an execution venue, Solana is the product
- Don't say "generates yield" — say "researches, validates, and executes yield strategies"
- Don't say "pump.fun fees" — say "token adoption fees" (pump.fun is one example)

## Quick Environment Setup

```bash
# Rust + Solana
rustup update stable
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
anchor install

# Python (for local fractal-swarm dev)
python -m venv .venv && source .venv/bin/activate
pip install pandas numpy ccxt pyarrow redis

# Verify
anchor build        # from rtp/programs/rtp-treasury/
anchor test --provider.cluster devnet
```

## Hackathon Context

- **Solana Frontier** (Colosseum × Canteen), $300k prizes, deadline **May 11, 2026**
- Register by May 4: https://arena.colosseum.org/register
- Copilot for pressure-testing: https://arena.colosseum.org/copilot
- Rules: https://colosseum.com/legal/Solana%20Frontier%20Hackathon%20Rules.pdf
- Resources: https://colosseum.com/frontier/resources

## Repo Map

| Repo | Purpose |
|------|---------|
| `tradewife/resilient-token-protocol` (this repo) | Hackathon submission, open-source skeleton |
| `tradewife/fractal-swarm` | Python fractal-swarm source, data, CI |
| `tradewife/rtp-skills-research` | Pre-hackathon research, skill system design |

## cldcde Skills — When to Use Them

You have 27 cldcde skills installed at `~/.claude/skills/`. These are prompt-based
skills that give you specialized capabilities. Invoke them by referencing their
SKILL.md when working on relevant tasks. Do NOT skip them — they encode
patterns and constraints the project depends on.

### Week 1: Treasury Program (use these NOW)

| When | Skill | Why |
|------|-------|-----|
| Designing the Coordinator message bus and wing routing | `swarm-orchestration` | Defines how agents communicate, fault tolerance, dynamic topology |
| Setting up consensus topology (Coordinator=queen, wings=workers) | `hive-mind-advanced` | Queen-worker pattern maps to Audit Wing approval flow |
| Defining `soulcontract.md` as enforceable spec, not just docs | `spec-lock` | Ensures implementation never drifts from governance without detection |
| Adversarial-reviewing the treasury program before committing | `red-team-tribunal` | 3-agent review: Skeptic + User Proxy + Optimizer. Every wing proposal must pass |

### Week 2: Coordinator + Evolve Wing

| When | Skill | Why |
|------|-------|-----|
| Writing Evolve Wing proposals and amendments | `sparc-methodology` | Specify → Pseudocode → Architect → Refine → Complete. Every change follows this |
| Coordinating audit pipeline across multiple skills | `compound-engineering` | Orchestrates Debt-Sentinel + Red Team + Spec-Lock into unified workflow |
| Truth-scoring + rollback after changes | `verification-quality` | 0.95 threshold scoring, maps to Audit Wing's safety.rs |

### Week 3: Knowledge + Security Wings

| When | Skill | Why |
|------|-------|-----|
| Designing the knowledge store for trades and decisions | `agentdb-memory-patterns` | Session memory (trades) + long-term memory (strategy history, patterns) |
| Distributed knowledge across market data, strategies, security | `agentdb-advanced` | Multi-database coordination design |
| Modeling circuit breaker / anti-pattern detection | `debt-sentinel` | Anomaly detection with hooks — model for Runtime Defense |
| Incident response and root cause analysis | `fpef-analyzer` | Find-Prove-Evidence-Fix framework |
| Pre/post message hooks for Coordinator routing | `hooks-automation` | Every message triggers pre-check (Audit) and post-check (logging) |

### Week 4: Full Loop + CI

| When | Skill | Why |
|------|-------|-----|
| Extending CI to include Rust tests + Anchor builds | `github-workflow-automation` | Night shift CI currently only runs Python |
| Benchmarking wing performance | `performance-analysis` | Self-assessment for identifying what to evolve |
| Sequential pipeline enforcement | `stream-chain` | Proposal → audit → approve → execute as typed chain |

### Week 5: Polish + Submission

| When | Skill | Why |
|------|-------|-----|
| Navigating all tools during final dev push | `prologue` | Ecosystem navigation |
| Competitive analysis vs other submissions | `ae-proof-agent` | Positioning for judges |
| Final adversarial review of entire codebase | `red-team-tribunal` | Run again as final sweep before submission |

### How to Use a Skill

When working on a task that matches a skill above, invoke it explicitly:

```
Use the swarm-orchestration skill to design the Coordinator message bus.
Ensure the design handles fault tolerance and dynamic wing topology.
```

The agent will read the skill's SKILL.md and apply its methodology, constraints,
and patterns to the task. This produces better, more consistent output than
working without the skill.
