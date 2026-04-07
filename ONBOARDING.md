# RTP — Agent Onboarding (Day 3+)

> Read this file. Then read the three context files below. Then start building.

## First task: read these before writing any code

Read these three files in order. They give you the full picture — architecture,
design decisions, current state, and what's coming. Do NOT skip this step.

1. **`CLAUDE.md`** — Architecture, three-layer stack, message flow, invariant
   list, design decisions, command reference. This is the project's brain.
2. **`BUILD_PLAN_v3.md`** — Post-audit remediation schedule, invariant
   enforcement tracker, risk register, weekly timeline through May 11.
3. **`README.md`** — Project overview, demo flow, how RTP works for adopters.
   (Create this file if it doesn't exist — the old one may be stale.)

After reading all three, you'll understand the full context needed for the
tasks below.

## What is this project?

RTP is a Solana-native treasury governed by a Rust agent swarm. Token
projects adopt RTP by enabling TransferFeeConfig — their trading fees
route to a PDA-owned treasury. A 6-wing swarm researches, validates,
and executes yield strategies, returning yield to the project and its
holders. Hackathon deadline: May 11, 2026.

## What's already built?

**Treasury program** (`rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`):
Anchor 1.0 — initialize, withdraw_fees, check_redistribute, hydrate_swarm,
evolve_phase, verify_adoption, create_swarm_vault. All CRITICAL and HIGH
audit findings fixed. Compiles clean. No Anchor integration tests yet.

**Swarm runtime** (`rtp/swarm/src/`):
Coordinator (soulguard, router, lifecycle), Evolve Wing (assessor,
proposer, rollback), Audit Wing (3-agent tribunal with Byzantine
consensus), types system. 88 tests passing. All HIGH/MEDIUM swarm
findings fixed. Trading, Security, Knowledge, Futureproof wings are stubs.

## What needs to happen RIGHT NOW?

Full audit: `docs/SECURITY_AUDIT_2026-04-07.md`
Full schedule: `BUILD_PLAN_v3.md`

Previous sessions fixed 9 of 13 audit findings (C-1, C-2/C-3, H-1, H-2,
H-3, H-4, H-5, M-3, M-4, M-5). Two remain, then we move to tests.

### Fix these in order:

#### 1. M-1: `initialize` doesn't verify TransferFeeConfig [MEDIUM]
**File**: `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`

A treasury can be initialized for a vanilla Token-2022 mint that has no
TransferFeeConfig. `withdraw_fees` will then fail at CPI time with an
unhelpful error. The `verify_adoption` instruction (line ~383) already
has the deserialization logic — inline it into `initialize`.

**Fix**:
- Extract the mint-extension-check logic from `verify_adoption` into a
  helper function (or call it inline).
- In `initialize`, after storing state, deserialize the mint account data
  and verify `TransferFeeConfig` is present with the Treasury PDA as
  `withdraw_withheld_authority`.
- If the mint doesn't have TransferFeeConfig, return a clear error.
- Keep `verify_adoption` as a standalone read-only instruction for
  third-party verification (don't remove it).
- **Do NOT duplicate code** — share the logic between both instructions.

#### 2. I-2: Hardcoded test path [INFO]
**File**: `rtp/swarm/src/coordinator/soulcontract_spec.rs`, line 323

`parse_full_soulcontract` test uses hardcoded absolute path
`/home/kt/kt/tabs/resilient-token-protocol/soulcontract.md` — fails on
CI and any other machine.

**Fix**:
- Replace the hardcoded path with `env!("CARGO_MANIFEST_DIR")` and walk
  up to the repo root (same pattern as `Soulguard::new()` in soulguard.rs).
- Alternatively, use `option_env!` to skip the test gracefully when the
  file isn't found (the test already has an `if path.exists()` guard —
  just fix the path resolution).

#### 3. Stale doc comment [housekeeping]
**File**: `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`, line 70

The `authority` field comment still says "self-referential — no external
authority". H-1 fixed this — it's now the initializer's pubkey.

**Fix**: Change to: `/// The phase authority (set at initialization).`

#### 4. Anchor integration tests
**File**: create `rtp/programs/rtp-treasury/tests/` (doesn't exist yet)

There are ZERO integration tests for the treasury. This blocks any demo.

Write tests for every instruction. Use `@solana/web3.js` + `@coral-xyz/anchor`.
See `Anchor.toml` for program ID and cluster config.

Tests to write (in priority order):
- `initialize` — success path; reject `min_runway_balance < DEFAULT_MIN_RUNWAY`
- `initialize` — reject mint without TransferFeeConfig (tests M-1 fix)
- `verify_adoption` — success with correct mint; fail with vanilla mint
- `create_swarm_vault` — success; reject duplicate init
- `withdraw_fees` — verify `total_fees_withdrawn` increments (tests H-2 fix)
- `check_redistribute` — correct 70/20/10 split; reject wrong recipients
  (tests C-2/C-3 fix); reject when below threshold
- `hydrate_swarm` — success; reject when below runway (tests 90-day invariant)
- `evolve_phase` — reject when vault below SUSTENANCE_CAP (tests C-1 fix);
  success when above; reject at max phase

Reference: `CLAUDE.md` for Solana/Anchor commands, `docs/SECURITY_AUDIT_2026-04-07.md`
for the specific attack vectors each test should prove are fixed.

#### 5. Re-audit checkpoint
After tests pass:
- Walk through all 18 findings in `docs/SECURITY_AUDIT_2026-04-07.md`
- Confirm each fixed finding has a corresponding test
- Update the invariant enforcement table in CLAUDE.md (currently 4/10
  enforced — should be 8/10 after fixes)
- Run `anchor build` — must compile clean
- Run `anchor test` — all passing (or note which need devnet)

## After audit remediation (Week 3+)

Once audit is closed out, the BUILD_PLAN_v3.md Week 3 schedule starts:

1. **bridge.rs** — typed Python↔Rust interface for Trading Wing
2. **Wire Trading Wing** — handle Proposal, ExecutePermit, YieldReport
3. **Knowledge Wing** — in-memory knowledge graph (HashMap/DashMap)
4. **Security Wing** — threat detection, rate limiting
5. **Wire all stubs** — every wing handles at least 2 payload types

See `BUILD_PLAN_v3.md` Week 3 for full details.

## Verify after changes

```bash
# Swarm (must stay green)
cd rtp/swarm && cargo test

# Treasury (must compile)
cd rtp/programs/rtp-treasury && anchor build

# Treasury integration tests (once written)
cd rtp/programs/rtp-treasury && anchor test
```

## Rules

- Read the file before changing it.
- Don't modify `soulcontract.md`.
- Don't use Anchor 0.31 — this is Anchor 1.0.0 with Solana 3.x.
- Don't commit `scripts/`, `backtesting/`, `agents/`, `data/`, `strategies/`.
- Don't load skills/plugins — they've been audited and are not useful.
  (See `~/tabs/SKILL_AUDIT_2026-04-07.md` if curious.)
- Wings NEVER modify each other directly — all through Coordinator.
- Every message passes through soulguard.

## Key files

| File | What |
|------|------|
| `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs` | Treasury program |
| `rtp/swarm/src/coordinator/soulguard.rs` | Soulcontract enforcement |
| `rtp/swarm/src/coordinator/soulcontract_spec.rs` | Spec parser + tests |
| `rtp/swarm/src/coordinator/router.rs` | Message routing |
| `rtp/swarm/src/coordinator/mod.rs` | Coordinator (quality gate pipeline) |
| `rtp/swarm/src/wings/audit/mod.rs` | 3-agent tribunal |
| `rtp/swarm/src/types.rs` | All message/payload types |
| `docs/SECURITY_AUDIT_2026-04-07.md` | Full audit with all 18 findings |
| `BUILD_PLAN_v3.md` | Remediation schedule and invariant tracker |
| `soulcontract.md` | Constitutional constraints |
| `CLAUDE.md` | Architecture, commands, design decisions |
