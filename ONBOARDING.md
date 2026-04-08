# RTP — Agent Onboarding (Week 4)

> Read this file first. Then read the three context files below. Then start building.

## Before writing any code — read these in order

1. **[`CLAUDE.md`](../CLAUDE.md)** — Architecture, three-layer stack, message flow, invariant
   list, design decisions, command reference. This is the project's brain.
2. **[`BUILD_PLAN_v3.md`](../BUILD_PLAN_v3.md)** — Post-audit remediation schedule, invariant
   enforcement tracker, risk register, weekly timeline through May 11 deadline.
3. **[`README.md`](../README.md)** — Project overview, demo flow, wing descriptions.

After reading all three, you'll have the full context needed for the tasks below.

## What is this project?

RTP is a Solana-native, self-funding treasury governed by a modular Rust swarm. Token projects adopt RTP — their trading fees route to the swarm, which autonomously researches, validates, and executes yield strategies — returning yield back to the project and its holders. Funded by its own yield, forever.

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
     │Yield    │ │Threat  │ │Self- │ │Realtime  │ │Intent   │ │Quantum  │
     │gen +    │ │detect  │ │modify │ │knowledge │ │complian.│ │future-  │
     │exec     │ │defend  │ │adapt  │ │graph     │ │safety   │ │proofing │
     └────┬────┘ └────┬───┘ └──┬───┘ └────┬─────┘ └────┬────┘ └────┬────┘
          │           │        │          │            │           │
          └───────────┴────────┴────────┴────────────┴──────────┘
                               │
                    ┌──────────▼──────────────────┐
                    │     SOLANA TREASURY PDA      │
                    │  fees → yield → redistribute │
                    │  self-hydrate → run forever  │
                    └─────────────────────────────┘
```

## Hackathon deadline: May 11, 2026

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

### Swarm runtime — 133 tests passing
**Path**: `rtp/swarm/src/`

- **Coordinator**: soulguard, router, lifecycle (multi-stage quality gate)
- **Evolve Wing**: assessor, proposer, rollback (complete, tested)
- **Audit Wing**: 3-agent tribunal (Skeptic/UserProxy/Optimizer), Byzantine consensus
- **Trading Wing**: 5 payload types (TradingConfig, Proposal, ExecutePermit,
  YieldReport, Heartbeat), bridge.rs integration
- **Security Wing**: threat detection, rate-limiting, suspicious proposal
  pattern detection, 1-hour alert expiry
- **Knowledge Wing**: in-memory HashMap store, case-insensitive search, context
  filtering, cross-wing query support
- **Futureproof Wing**: heartbeat with deprecation monitoring (7 crates)
- **bridge.rs**: typed Python↔Rust interface (BridgeRequest/BridgeResponse),
  subprocess call with error handling, `NIGHT_SHIFT_BIN` constant ready for Week 4 swap
- **Types system**: Message, Payload, WingId, Priority (see `rtp/swarm/src/types.rs`)

### I-1 fix: no silent message drops
All 6 wings return `Payload::Error { reason: "Unimplemented: <type>" }` for
unhandled payload types. The `None` return that caused silent drops has been
eliminated from every wing.

## What needs to happen NOW (Week 4: Apr 21–25)

Full schedule: [`BUILD_PLAN_v3.md`](../BUILD_PLAN_v3.md) Week 4 section.

### Task 1: Black-box Python fractal-swarm → night_shift.bin
**File**: `scripts/` (Python, gitignored — ships as compiled binary)
**Output**: `night_shift.bin` at repo root

The Python fractal-swarm already works (night_shift.py, paper_trader.py). This task
packages it into a standalone PyInstaller binary so the Rust swarm can call it through
bridge.rs without a Python runtime.

**What to build**:
- `pyinstaller` spec that produces a single `night_shift.bin` executable
- Binary should accept `--bridge-mode` and read JSON from stdin, write JSON to stdout
- Binary should include all dependencies (pandas, numpy, ccxt) — or a minimal
  interface that only carries the config/params needed for bridge mode
- Test: `bridge.rs` `call_bridge("night_shift.bin", request)` returns a valid
  `BridgeResponse`

**Reference**: See `rtp/swarm/src/bridge.rs` — the Rust side is done.
**Key file**: `scripts/night_shift.py` — the Python entry point for bridge mode.
**Key constraint**: the binary must work on a clean Ubuntu system with no Python install.
  All Python deps must be bundled. Consider using `--hidden-imports` to trim fat.

### Task 2: Encrypted configs (AES, build-time key)
**File**: `rtp/swarm/src/config.rs` (new) or inline in bridge.rs

Protect strategy parameters with AES-256-GCM encryption so they're not visible in
the binary. Build-time key derivation from an environment variable or a `.env` file
that gitignore'd.

**What to build**:
- `ConfigEncryption` struct with encrypt/decrypt using `aes-gcm` crate
- Load key from env var `RTP_CONFIG_KEY` (hex-encoded 256-bit key)
- Store/load encrypted configs from `configs/` directory
- Fallback: if no key, work in plaintext (for development)

**Constraint**: Don't add heavy crypto dependencies. Use the `aes-gcm` crate (pure Rust,
no FFI). The `RTP_CONFIG_KEY` env var is only needed for production builds.

### Task 3: End-to-end devnet demo loop
**Files**: `rtp/swarm/src/` (coordination code), `rtp/programs/` (Anchor)

Wire the full message loop end-to-end on devnet:

1. Token adopts RTP (initialize + verify_adoption on devnet)
2. Fees accumulate (withdraw_fees)
3. Threshold hit → check_redistribute (70/20/10 split)
4. Swarm proposes strategy → Audit tribunal approves → Trading executes via bridge
5. Self-hydration (hydrate_swarm with runway check)

The individual pieces already work (treasury program tests, swarm tests, bridge.rs).
This task is about wiring them together into a runnable demo flow.

**Reference**: [`docs/demo-flow.md`](../docs/demo-flow.md) — the 3-minute demo script.
**Key file**: `rtp/swarm/src/coordinator/mod.rs` — the existing integration tests
  already demonstrate the proposal→audit→execute→permit flow. Extend to include
  yield report ingestion and knowledge storage.

**Devnet RPC**: use Triton One free tier — https://triton.one (always free devnet/testnet).
**Anchor 1.0 caveat**: `anchor test --skip-build` only works with `--validator legacy
  --provider.cluster localnet`. For devnet, you'd use a local `solana-test-validator`
  with the deployed program loaded.

### Task 4: GitHub Actions CI
**File**: `.github/workflows/swarm-ci.yml` (modify existing night_shift.yml or create new)

Add a CI pipeline that runs on every push:
- `cargo build` (swarm compiles clean)
- `cargo test` (all 133+ tests pass)
- `anchor build` (treasury compiles clean)
- `cargo clippy` (no warnings)

**Reference**: See existing `.github/workflows/night_shift.yml` for the project's
CI patterns. Build on that YAML structure — don't create from scratch.

## Key files for Week 4

| File | What |
|------|------|
| [`CLAUDE.md`](../CLAUDE.md) | Architecture, commands, design decisions, invariant list |
| [`BUILD_PLAN_v3.md`](../BUILD_PLAN.md) | Full schedule, risk register, invariant tracker |
| [`README.md`](../README.md) | Project overview, demo flow, yield brain results |
| [`docs/demo-flow.md`](../docs/demo-flow.md) | 3-minute hackathon demo script |
| [`docs/SECURITY_AUDIT_2026-04-07.md`](../docs/SECURITY_AUDIT_2026-04-07.md) | All 18 audit findings with fixes |
| [`CODEREVIEW.md`](../CODEREVIEW.md) | Code review instructions for AI agents |
| [`soulcontract.md`](../soulcontract.md) | Constitutional constraints (DO NOT MODIFY) |
| `rtp/swarm/src/types.rs` | All message/payload types — read before any wing changes |
| `rtp/swarm/src/bridge.rs` | Python↔Rust typed interface — `NIGHT_SHIFT_BIN` constant |
| `rtp/swarm/src/coordinator/mod.rs` | Coordinator quality gate pipeline |
| `rtp/swarm/src/coordinator/router.rs` | Message routing |
| `rtp/swarm/src/coordinator/soulguard.rs` | Soulcontract enforcement on every message |
| `rtp/swarm/src/wings/trading/mod.rs` | Trading Wing (complete ✅) |
| `rtp/swarm/src/wings/security/mod.rs` | Security Wing (complete ✅) |
| `rtp/swarm/src/wings/knowledge/mod.rs` | Knowledge Wing (complete ✅) |
| `rtp/swarm/src/wings/audit/mod.rs` | Audit Wing (complete ✅) |
| `rtp/swarm/src/wings/evolve/` | Evolve Wing (complete ✅) |
| `rtp/swarm/src/wings/futureproof/mod.rs` | Futureproof Wing (complete ✅) |
| `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs` | Treasury program (Anchor) |
| `rtp/programs/rtp-treasury/tests/treasury.ts` | Treasury integration tests (15 tests) |
| `rtp/programs/rtp-treasury/Anchor.toml` | Anchor config (program ID, cluster, test runner) |
| `rtp/programs/rtp-treasury/target/idl/rtp_treasury.json` | IDL for Anchor client |

## Reference links for Week 4 tasks

### PyInstaller packaging
- PyInstaller docs: https://pyinstaller.org/en/stable/
- `--onefile` mode: https://pyinstaller.org/en/stable/specs/onefile.html
- `--hidden-imports`: https://pyinstaller.org/en/stable/specs/hidden-imports.html
- Night shift entry point: `scripts/night_shift.py`

### AES-256-GCM encryption
- `aes-gcm` crate: https://docs.rs/crate/aes-gcm/
- RustCrypto/hmac: https://docs.rs/crate/hmac/ (key derivation)
- Solana encrypt/decrypt pattern: https://solanacookbook.com/guides/advanced/encrypt-sensitive-data.html

### Devnet setup
- Triton One RPC: https://triton.one (free devnet/testnet)
- Solana devnet faucet: https://faucet.solana.com
- TransferFeeConfig docs: https://solana.com/docs/tokens/extensions/transfer-fees

### GitHub Actions CI
- Existing CI pattern: `.github/workflows/night_shift.yml`
- YAML matrix for multiple jobs: https://docs.github.com/en/actions/using-workflows/using-workflows
- Cargo cache: https://github.com/rust-lang/cargo/issues/7841

### Demo
- Demo script: [`docs/demo-flow.md`](../docs/demo-flow.md)
- Build plan demo section: [`BUILD_PLAN_v3.md`](../BUILD_PLAN.md) Week 5

## Verify after every change

```bash
# Swarm — must stay green (133+ tests)
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
| 10| Strategies black-boxed | ✅ | PyInstaller binary design |

Target: 9/10 enforced now. Invariant 7 is planned for Week 5.

## Code review findings (2026-04-08)

Full review is in `CODEREVIEW.md`. Summary of findings relevant to Week 4:

| Severity | Finding | File | Status |
|----------|---------|------|--------|
| MEDIUM | Stub wings silently drop messages (return `None`) | `wings/*/mod.rs` | ✅ Fixed in Week 3 |
| MEDIUM | `set_risk_budget()` silent clamp at 1.0 | `soulguard.rs:266` | Not a blocker; return `Result` or log if time permits |
| MEDIUM | `exceeds_rollback_threshold` falls back to 0.05 on poisoned lock | `soulguard.rs:293` | Not a blocker; log warning before Week 5 |
| LOW | Phase threshold parser case-sensitive (`k` vs `K`) | `soulcontract_spec.rs:148` | Use `.to_lowercase().contains("k")` if time permits |
| LOW | CI `git pull --rebase || true` swallows failures | `night_shift.yml:89` | Improve echo message |
| INFO | No subprocess timeout in bridge.rs | `bridge.rs` | Add tokio::time::timeout in Week 4 |

**No CRITICAL or HIGH findings.** Treasury program patches are solid. Swarm architecture
is clean. The codebase is ready for Week 4 work.

## After Week 4 (Weeks 5–6 at a glance)

### Week 5 (Apr 28 – May 2): Polish + Hardening
- Demo rehearsal (3 minutes) — see [`docs/demo-flow.md`](../docs/demo-flow.md)
- soulguard reload with signature verification (or document as production TODO)
- Final security sweep, README polish, video recording
- Colosseum Copilot final check: https://arena.colosseum.org/copilot

### Week 6 (May 5–11): Submission
- Register individually (deadline May 4 — DO THIS FIRST)
- Final `cargo test` + `anchor test` runs
- Submit to Colosseum: https://arena.colosseum.org/register
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
  `Payload::Error { reason: "..." }`, not `None`.
- **Token-2022 gotchas** (if you touch treasury tests):
  - `transferCheckedWithFee` stores withheld fees in the **DESTINATION** token account
  - Use `sendAndConfirmTransaction` directly — NOT the `@solana/spl-token` wrapper
  - Add 200-300ms sleeps after `.rpc()` calls that perform CPI before reading state
  - CPI from Anchor to Token-2022 requires `mut` on accounts the CPI marks as writable
