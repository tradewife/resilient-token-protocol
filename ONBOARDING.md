# RTP — Resilient Token Protocol: Agent Onboarding

## Step 1: Install dependencies

Run these now. You may need sudo. Verify all four pass before continuing.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
cargo install --git https://github.com/coral-xyz/anchor avm --force
avm install 1.0.0 && avm use 1.0.0
```

Verify:
```bash
export PATH="$HOME/.cargo/env:$HOME/.local/share/solana/install/active_release/bin:$PATH"
rustc --version && solana --version && anchor --version && cargo-build-sbf --version
```

## Step 2: Read these three files, in this order

1. `soulcontract.md` — the constitutional constraints every line of code must obey
2. `BUILD_PLAN.md` — the 5-week plan. **Focus on Part 5, Week 2 (line 416)**.
3. `CLAUDE.md` — architecture, commands, key invariants, design decisions

## Step 3: Build the swarm runtime

RTP is a Solana-native, self-funding treasury governed by a modular Rust swarm.
Any token project adopts RTP by enabling TransferFeeConfig on their mint. Trading fees
auto-route to a PDA-owned treasury. A Rust swarm researches, validates, and executes
yield strategies — returning yield back to the project and its holders.

**Week 1 is done.** The Treasury program is at `rtp/programs/rtp-treasury/` (Anchor 1.0.0, builds to BPF).

**You are building Week 2:** the Rust swarm runtime at `rtp/swarm/`.

The directory exists but is empty scaffolding. Your job:

### Coordinator (`rtp/swarm/src/coordinator/`)

The Coordinator is the message bus. Wings never talk to each other — everything goes through here.

| File | What it does | Reference |
|------|-------------|-----------|
| `router.rs` | Typed message routing between wings — proposal → audit → approve → execute | https://github.com/revfactory/harness |
| `soulguard.rs` | Enforces soulcontract.md on every message. If a wing proposes something that violates an invariant, it gets rejected before any wing sees it. | Read `soulcontract.md` "What Cannot Evolve" |
| `lifecycle.rs` | Wing spawn, health-check, retire. Manages wing registration and fault tolerance. | https://github.com/kevinrgu/autoagent |

### Evolve Wing (`rtp/swarm/src/wings/evolve/`)

The only wing that can modify how other wings work. Every change is a diff that goes through the Coordinator and must pass the Audit Wing.

| File | What it does | Reference |
|------|-------------|-----------|
| `assessor.rs` | Benchmark wing performance, identify bottlenecks and regressions | Treasury-native metric: `(USDC yield / SOL reserves) × (1 - max drawdown) × wing consistency` |
| `proposer.rs` | Architecture change proposals — new modules, refactors, config changes | https://github.com/karpathy/autoresearch (Modify/Verify/Keep spec) |
| `rollback.rs` | If a change degrades performance > 5%, revert within minutes | https://github.com/chrisworsey55/atlas-gic (Darwinian loop) |

### Skills to use (invoke by name when the task matches)

| Task | Skill |
|------|-------|
| Designing the Coordinator message bus | `swarm-orchestration` |
| Coordinator=queen, wings=workers consensus | `hive-mind-advanced` |
| soulcontract as enforceable spec, not just docs | `spec-lock` |
| Evolve Wing proposal format | `sparc-methodology` |
| Orchestrating audit across multiple skills | `compound-engineering` |
| Truth-scoring + rollback | `verification-quality` |

Invoke a skill like this: *"Use the swarm-orchestration skill to design the Coordinator message bus."*

## Rules

- Wings NEVER modify each other directly. All communication through Coordinator.
- Every message passes through soulguard (soulcontract enforcement).
- Python ↔ Rust interface is typed JSON. Any wing can propose; any wing can act.
- Don't modify `soulcontract.md` core values.
- Don't use Anchor 0.31 — this project uses Anchor 1.0.0 with Solana 3.x.
- Don't say "generates yield" — say "researches, validates, and executes yield strategies".
- Don't say "pump.fun fees" — say "token adoption fees".
- Don't commit `scripts/`, `backtesting/`, `agents/`, `data/`, `strategies/` — gitignored.
- Don't modify the Treasury program unless fixing a bug found during integration.
- Don't use World Coin.
- Don't make Hyperliquid central to the narrative — it's an execution venue, Solana is the product.

## Repo structure

```
rtp/
├── swarm/src/                    ← YOU BUILD THIS
│   ├── coordinator/              ← router.rs, soulguard.rs, lifecycle.rs
│   └── wings/
│       ├── evolve/               ← assessor.rs, proposer.rs, rollback.rs
│       ├── trading/              (Week 4)
│       ├── security/             (Week 3)
│       ├── knowledge/            (Week 3)
│       ├── audit/                (Week 3)
│       └── futureproof/          (Week 5)
├── programs/rtp-treasury/        ← DONE
├── soulcontract.md               ← READ FIRST
├── BUILD_PLAN.md                 ← READ SECOND
├── CLAUDE.md                     ← READ THIRD
└── docs/demo-flow.md
```

## Week 2 checkpoint

By end of Week 2, the Coordinator should route typed messages between wing stubs,
soulguard should reject messages that violate the soulcontract, and the Evolve Wing
should be able to propose and roll back changes. The swarm should compile with `cargo build`.
