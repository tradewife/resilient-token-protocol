# RTP — Agent Onboarding (Week 3+)

> Read this file first. Then read the three context files below. Then start building.

## Before writing any code — read these in order

1. **[`CLAUDE.md`](./CLAUDE.md)** — Architecture, three-layer stack, message flow, invariant
   list, design decisions, command reference. This is the project's brain.
2. **[`BUILD_PLAN_v3.md`](./BUILD_PLAN_v3.md)** — Post-audit remediation schedule, invariant
   enforcement tracker, risk register, weekly timeline through May 11 deadline.
3. **[`README.md`](./README.md)** — Project overview, demo flow, architecture diagram,
   yield brain results, wing descriptions.

After reading all three, you'll have the full context needed for the tasks below.

## What is this project?

RTP is a Solana-native treasury governed by a Rust agent swarm. Token projects adopt RTP
by enabling TransferFeeConfig — their trading fees route to a PDA-owned treasury. A
6-wing swarm researches, validates, and executes yield strategies, returning yield to
the project and its holders. **Hackathon deadline: May 11, 2026.**

## What's already built? (ALL COMPLETE ✅)

### Treasury program — audit-remediated, 15 integration tests passing
**Path**: `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`

Anchor 1.0 with Agave 3.1.12 (`solana-test-validator`). Seven instructions:
`initialize`, `verify_adoption`, `create_swarm_vault`, `withdraw_fees`,
`check_redistribute`, `hydrate_swarm`, `evolve_phase`.

**Audit remediation** — ALL CRITICAL, HIGH, and MEDIUM findings fixed:
- C-1: Phase evolution threshold enforcement (SUSTENANCE_CAP / ECOSYSTEM_CAP)
- C-2/C-3: Recipient account validation (InterfaceAccount with mint + authority constraints)
- H-1: Authority set to initializer pubkey (not self-referential PDA)
- H-2: Vault balance reloaded after CPI (`ctx.accounts.treasury_vault.reload()`)
- H-3: `min_runway_balance == 0` rejected explicitly
- H-4: Deleted `spec()` unreachable method
- H-5: Rollback threshold read from stored spec value
- M-1: TransferFeeConfig verified during `initialize`
- M-3: Dead Rule 2 removed from soulguard
- M-4: Router made `pub(crate)`
- M-5: `stub_review()` rejects EvolveProposals

**Integration tests**: 15 tests in `rtp/programs/rtp-treasury/tests/treasury.ts` (797 lines),
covering every instruction and all audit fixes. All pass.

### Swarm runtime — 88 tests passing
**Path**: `rtp/swarm/src/`

- **Coordinator**: soulguard, router, lifecycle (multi-stage quality gate)
- **Evolve Wing**: assessor, proposer, rollback (complete, tested)
- **Audit Wing**: 3-agent tribunal (Skeptic/UserProxy/Optimizer), Byzantine consensus
- **Types system**: Message, Payload, WingId, Priority (see `rtp/swarm/src/types.rs`)

### Wing stubs (need implementation in Week 3)
- **Trading**: only handles `TradingConfig` → returns `Ack`. Silently drops `Proposal`, `ExecutePermit`, `YieldReport`.
- **Security**: only handles `Heartbeat` → returns `Ack`. Silently drops `SecurityAlert`, `Proposal`.
- **Knowledge**: only handles `KnowledgeQuery` → returns placeholder. Silently drops `Assessment`, `YieldReport`.
- **Futureproof**: only handles `Heartbeat` → returns `Ack`.

> **⚠️ Silent message drops (I-1 from audit)**: All stub wings return `None` for
> unhandled payloads. This means messages are silently lost with no audit trail.
> When implementing each wing, the `None` fallback should return
> `Payload::Error { reason: "Unimplemented: <payload_type>" }` so dropped
> messages are visible in the Coordinator's processing results. Do this as part
> of each wing's implementation, not as a separate task.

## What needs to happen NOW (Week 3: Apr 14–18)

Full schedule: [`BUILD_PLAN_v3.md`](./BUILD_PLAN_v3.md) Week 3 section.
Full audit: [`docs/SECURITY_AUDIT_2026-04-07.md`](./docs/SECURITY_AUDIT_2026-04-07.md).

### Task 1: Create `bridge.rs` — Python↔Rust typed interface
**File**: `rtp/swarm/src/bridge.rs` (new)

The Trading Wing needs to call the Python fractal-swarm binary and receive
typed JSON proposals. This is the bridge between the research layer and
execution layer.

**What to build**:
- Define `BridgeRequest` and `BridgeResponse` structs (serde JSON)
- Function that calls Python binary via `std::process::Command`
- Input: strategy parameters (symbol, config JSON)
- Output: typed `BridgeResponse` with yield estimate, confidence, params
- Error handling: binary not found, malformed output, timeout
- Unit tests with mock binary or captured output

**Reference**: See `rtp/swarm/src/types.rs` for existing `Payload::TradingConfig` and
`Payload::YieldReport` — bridge output should map to these.

### Task 2: Wire Trading Wing beyond stub
**File**: `rtp/swarm/src/wings/trading/mod.rs`

Currently only handles `TradingConfig`. Extend to handle:
- `Proposal` → validate strategy params, submit yield proposal
- `ExecutePermit` → call bridge.rs to execute strategy via Python binary
- `YieldReport` → report execution results back to Coordinator
- `Heartbeat` → report wing health with last execution metrics

**Keep it simple**: in-memory state for the wing (last proposal, last report,
execution count). No external dependencies beyond bridge.rs.

**Important**: The stub's `None` fallback must become `Payload::Error { reason: ... }`
for any unhandled payload type. See the silent-drop warning above.

### Task 3: Knowledge Wing — in-memory knowledge graph
**File**: `rtp/swarm/src/wings/knowledge/mod.rs`

Replace the stub with a functional wing:
- `DashMap` or `HashMap`-based store for strategy results, wing metrics, decisions
- `KnowledgeQuery` → search stored entries by key/context
- `Heartbeat` → report store size and query count
- `YieldReport` → store yield results for cross-wing recall
- `Assessment` → store evolve wing assessments
- Cross-wing query support: any wing can ask "what do we know about X?"

**Do NOT**: use a database, npm packages, or external services. Pure Rust HashMap.

### Task 4: Security Wing — threat detection
**File**: `rtp/swarm/src/wings/security/mod.rs`

Replace the stub with a functional wing:
- `SecurityAlert` → log and track threats (severity, threat description)
- `Heartbeat` → report threat count, last alert timestamp
- Rate-limit tracking: count proposals per wing, flag if above threshold
- `Proposal` → check for suspicious patterns (e.g., SoulcontractAmendment proposals)
- In-memory alert store with timestamp-based expiry

### Task 5: Wire wings to Coordinator + integration tests
**Files**: `rtp/swarm/src/coordinator/mod.rs`, `rtp/swarm/src/coordinator/router.rs`

- Ensure all 6 wings respond to at least 2 payload types each
- Wire Security and Knowledge wing handlers in the Coordinator's routing
- Integration test: send a `Proposal` through Coordinator → Audit → Trading → execute
- Integration test: Knowledge query returns stored yield report data
- Integration test: Security wing flags suspicious proposal pattern

**Reference**: See how Evolve and Audit wings are wired in the Coordinator for the pattern.

### Task 6: Futureproof Wing — minimal heartbeat + deprecation stub
**File**: `rtp/swarm/src/wings/futureproof/mod.rs`

Add at minimum:
- `Heartbeat` → report wing status
- Deprecation check stub: check a hardcoded list of crate versions
- 2 payload types handled

## Verify after every change

```bash
# Swarm — must stay green (88+ tests, growing as you add tests)
cd rtp/swarm && cargo test

# Treasury — must compile clean
cd rtp/programs/rtp-treasury && anchor build

# Treasury integration tests — all 15 must pass
cd rtp/programs/rtp-treasury && anchor test --skip-build --validator legacy --provider.cluster localnet
```

## Invariant enforcement tracker (current: 9/10 ✅)

| # | Invariant | Status | Notes |
|---|-----------|--------|-------|
| 1 | PDA owns treasury | ✅ | Enforced in lib.rs |
| 2 | TransferFeeConfig immutable | ✅ | M-1: verified in initialize |
| 3 | CPI-only transfers | ✅ | All token ops via CPI |
| 4 | Agent proposes, human approves | ✅ | H-1 authority + C-1 thresholds |
| 5 | No SOL liquidation | ✅ | USDC-only design |
| 6 | Phase transitions irreversible | ✅ | One-way enum advancement |
| 7 | Soulcontract amend = human sig | ⚠️ | Week 5: reload signature check |
| 8 | Auto-rollback >5% degradation | ✅ | H-5: threshold from spec |
| 9 | Self-hydration >90-day runway | ✅ | Enforced in hydrate_swarm |
| 10 | Strategies black-boxed | ✅ | PyInstaller binary design |

Target: 9/10 enforced now. Invariant 7 is planned for Week 5.

**Note on invariant 8**: `soulguard.exceeds_rollback_threshold()` reads the cached
threshold via `try_read()` and falls back to hardcoded `0.05` if the `RwLock` is
poisoned. This is a minor robustness issue — a poisoned lock indicates a bug
elsewhere, and the silent fallback masks it. **Not a blocker for Week 3** but
should be hardened (log a warning on poisoned lock) before Week 5 security sweep.

**Note on M-2 (from security audit)**: `Initialize` has no explicit `has_one = mint`
constraint on the Treasury account. The code review (2026-04-08) concluded this is
**safe by construction** — Anchor's `init` with `seeds = [TREASURY_SEED, mint.key()]`
guarantees the PDA is derived from the correct mint. No fix needed.

**Note on `soulguard.set_risk_budget()`**: Silently clamps values > 1.0 without
error or log. If a caller passes an invalid budget, it is dropped with no feedback.
Not a blocker, but should return `Result` or log a warning.

## Code review findings (2026-04-08)

Full review is in `CODEREVIEW.md`. Summary of findings relevant to Week 3:

| Severity | Finding | File | Action for Week 3 |
|----------|---------|------|-------------------|
| MEDIUM | Stub wings silently drop messages (return `None`) | `wings/*/mod.rs` | Return `Payload::Error` for unhandled types — do as part of wing implementation |
| MEDIUM | `set_risk_budget()` silent clamp at 1.0 | `soulguard.rs:266` | Not a blocker; return `Result` or log if time permits |
| MEDIUM | `exceeds_rollback_threshold` falls back to 0.05 on poisoned lock | `soulguard.rs:293` | Not a blocker; log warning before Week 5 |
| LOW | Phase threshold parser case-sensitive (`k` vs `K`) | `soulcontract_spec.rs:148` | Use `.to_lowercase().contains("k")` if time permits |
| LOW | CI `git pull --rebase || true` swallows failures | `night_shift.yml:89` | Improve echo message |

**No CRITICAL or HIGH findings.** Treasury program patches are solid. Swarm architecture
is clean. The codebase is ready for Week 3 work.

## After Week 3 (Weeks 4–6 at a glance)

### Week 4 (Apr 21–25): Full Loop + Black-Boxing
- Black-box Python fractal-swarm → `night_shift.bin` (PyInstaller)
- Encrypted configs (AES, build-time key)
- End-to-end devnet demo: adopt → fees → redistribute → swarm executes → yield
- GitHub Actions CI (`cargo build` + `cargo test` + `anchor build`)
- **Skill load**: `github-workflow-automation` (YAML templates only — ignore ruv-swarm/claude-flow refs)

### Week 5 (Apr 28–May 2): Polish + Hardening
- Demo rehearsal (3 minutes) — see [`docs/demo-flow.md`](./docs/demo-flow.md)
- **Skill load**: `walkthrough` (builtin) — generate Mermaid diagrams for README
- **Skill load**: `code-review` (builtin) — formal diff review of treasury program
- Final security sweep, README polish, video recording

### Week 6 (May 5–11): Submission
- Register individually (deadline May 4)
- Final `cargo test` + `anchor test` runs
- Submit to Colosseum
- **Deadline: May 11**

## Rules

- **Read the file before changing it.**
- **Don't modify `soulcontract.md`.**
- **Don't use Anchor 0.31** — this is Anchor 1.0.0 with Solana 3.x (Agave 3.1.12).
- **Don't commit** `scripts/`, `backtesting/`, `agents/`, `data/`, `strategies/`.
- **Don't load skills/plugins** unless explicitly told to in the task above.
  (See `~/tabs/SKILL_AUDIT_2026-04-07.md` — most are stubs or mocks.)
- **Wings NEVER modify each other directly** — all cross-wing communication via Coordinator.
- **Every message passes through soulguard** — the Coordinator enforces this.
- **Wings must never silently drop messages** — unhandled payloads must return
  `Payload::Error { reason: "..." }`, not `None`. The `None` return in current stubs
  is a known issue (I-1) that gets fixed as each wing is implemented.
- **Token-2022 gotchas** (if you touch treasury tests):
  - `transferCheckedWithFee` stores withheld fees in the **DESTINATION** token account
  - Use `sendAndConfirmTransaction` directly — NOT the `@solana/spl-token` wrapper
  - Add 200-300ms sleeps after `.rpc()` calls that perform CPI before reading state
  - CPI from Anchor to Token-2022 requires `mut` on accounts the CPI marks as writable

## Key files

| File | What |
|------|------|
| [`CLAUDE.md`](./CLAUDE.md) | Architecture, commands, design decisions, invariant list |
| [`BUILD_PLAN_v3.md`](./BUILD_PLAN_v3.md) | Full schedule, risk register, invariant tracker |
| [`README.md`](./README.md) | Project overview, demo flow, yield brain results |
| [`soulcontract.md`](./soulcontract.md) | Constitutional constraints (DO NOT MODIFY) |
| [`docs/SECURITY_AUDIT_2026-04-07.md`](./docs/SECURITY_AUDIT_2026-04-07.md) | All 18 audit findings with fixes |
| [`docs/demo-flow.md`](./docs/demo-flow.md) | 3-minute hackathon demo script |
| `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs` | Treasury program (Anchor) |
| `rtp/programs/rtp-treasury/tests/treasury.ts` | Treasury integration tests (15 tests) |
| `rtp/swarm/src/types.rs` | All message/payload types — **read this before building wings** |
| `rtp/swarm/src/coordinator/mod.rs` | Coordinator quality gate pipeline |
| `rtp/swarm/src/coordinator/router.rs` | Message routing — wire new wings here |
| `rtp/swarm/src/coordinator/soulguard.rs` | Soulcontract enforcement on every message |
| `rtp/swarm/src/coordinator/lifecycle.rs` | Wing spawn, health-check, retire |
| `rtp/swarm/src/wings/trading/mod.rs` | Trading Wing (stub → Week 3) |
| `rtp/swarm/src/wings/security/mod.rs` | Security Wing (stub → Week 3) |
| `rtp/swarm/src/wings/knowledge/mod.rs` | Knowledge Wing (stub → Week 3) |
| `rtp/swarm/src/wings/evolve/mod.rs` | Evolve Wing (complete ✅) |
| `rtp/swarm/src/wings/audit/mod.rs` | Audit Wing (complete ✅) |
| `rtp/swarm/src/wings/futureproof/mod.rs` | Futureproof Wing (stub) |
| `rtp/programs/rtp-treasury/Anchor.toml` | Anchor config (program ID, cluster, test runner) |
| `rtp/programs/rtp-treasury/target/idl/rtp_treasury.json` | IDL for Anchor client |
