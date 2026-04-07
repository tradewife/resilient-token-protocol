# RTP — Agent Onboarding (Day 2+)

> Read this file. Then start fixing bugs. No other setup needed.

## What is this project?

RTP is a Solana-native treasury governed by a Rust agent swarm. Token
projects adopt RTP by enabling TransferFeeConfig — their trading fees
route to a PDA-owned treasury. A 6-wing swarm researches, validates,
and executes yield strategies, returning yield to the project and its
holders. Hackathon deadline: May 11, 2026.

## What's already built?

**Treasury program** (`rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`):
Anchor 1.0 — initialize, withdraw_fees, check_redistribute, hydrate_swarm,
evolve_phase, verify_adoption, create_swarm_vault. Compiles but has
critical bugs (see below).

**Swarm runtime** (`rtp/swarm/src/`):
Coordinator (soulguard, router, lifecycle), Evolve Wing (assessor,
proposer, rollback), Audit Wing (3-agent tribunal with Byzantine
consensus), types system. 88 tests passing. Trading, Security,
Knowledge, Futureproof wings are stubs.

## What needs to happen RIGHT NOW?

A security audit found 3 CRITICAL, 5 HIGH, and 5 MEDIUM bugs.
Full audit: `docs/SECURITY_AUDIT_2026-04-07.md`
Full schedule: `BUILD_PLAN_v3.md`

### Fix these in order:

#### 1. C-2/C-3: Recipient account validation is broken [CRITICAL]
**File**: `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`

`holders_recipient` (line 530) is an unchecked `AccountInfo` — anyone
can steal 70% of redistribution. `dev_recipient` constraint (line 537)
compares `AccountInfo.owner` (the Token program ID) against a wallet
pubkey — semantically wrong, always fails.

**Fix**:
- Add `holders_wallet: Pubkey` to `Treasury` state struct (line 69)
- Accept it as a param or account in `initialize()`
- Change `CheckRedistribute` (line 500) — all three recipients become:
```rust
#[account(mut, token::mint = mint, token::authority = treasury.holders_wallet)]
pub holders_recipient: InterfaceAccount<'info, TokenAccount>,
#[account(mut, token::mint = mint, token::authority = treasury.project_dev_wallet)]
pub dev_recipient: InterfaceAccount<'info, TokenAccount>,
#[account(mut, token::mint = mint, token::authority = treasury.ecosystem_wallet)]
pub ecosystem_recipient: InterfaceAccount<'info, TokenAccount>,
```
- Recompute `Treasury::INIT_SPACE` (added a Pubkey = +32 bytes)

#### 2. H-1: Self-referential authority [HIGH]
**File**: same `lib.rs`, line 138

`treasury.authority = treasury.key()` makes the PDA its own authority.
`evolve_phase` requires this PDA to sign, which is impossible. Dead code.

**Fix**: Change line 138 to:
```rust
treasury.authority = ctx.accounts.authority.key();
```

#### 3. C-1: Phase evolution has no threshold enforcement [CRITICAL]
**File**: same `lib.rs`, lines 348-371

`evolve_phase` advances phases with zero balance check. The TODO on
line 367 admits this. Anyone with authority can jump to Humanity with $0.

**Fix**:
- Add `treasury_vault` to `EvolvePhase` account context
- Enforce vault balance against `SUSTENANCE_CAP` / `ECOSYSTEM_CAP`
- Remove `#[allow(dead_code)]` from those constants

#### 4. H-2: Stale vault balance after CPI [HIGH]
**File**: same `lib.rs`, line 198

After `withdraw_withheld_tokens_from_mint` CPI, `vault.amount` is stale.
Delta is always 0. `total_fees_withdrawn` never increments.

**Fix**: After the CPI call (line 195), add:
```rust
ctx.accounts.treasury_vault.reload()?;
```

#### 5. H-3: min_runway_balance = 0 silently defaults [HIGH]
**File**: same `lib.rs`, line 150

Comment says "reject 0 explicitly" but code silently defaults to
`DEFAULT_MIN_RUNWAY`. Either error on 0 or fix the comment.

#### 6. Swarm fixes (after treasury)

- **H-4**: Delete `soulguard.rs` lines 306-309 (`spec()` calls `unreachable!()`)
- **H-5**: `soulguard.rs` line 302 — `exceeds_rollback_threshold()` hardcodes
  `0.05` instead of reading from spec. Store threshold on reload.
- **M-3**: Delete soulguard Rule 2 (lines 131-144) — dead code, Rule 1 covers it
- **M-4**: Make `router` field `pub(crate)` in `coordinator/mod.rs` to prevent
  soulguard bypass via direct `Router::route()` calls
- **M-5**: `audit/mod.rs` `stub_review()` auto-approves `EvolveProposal` — reject it

## Verify after fixes

```bash
# Swarm (must stay green)
cd rtp/swarm && cargo test

# Treasury (must compile — full anchor test needs devnet)
cd rtp/programs/rtp-treasury && anchor build
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
| `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs` | Treasury program — **fix this first** |
| `rtp/swarm/src/coordinator/soulguard.rs` | Soulcontract enforcement |
| `rtp/swarm/src/coordinator/router.rs` | Message routing |
| `rtp/swarm/src/coordinator/mod.rs` | Coordinator (quality gate pipeline) |
| `rtp/swarm/src/wings/audit/mod.rs` | 3-agent tribunal |
| `rtp/swarm/src/types.rs` | All message/payload types |
| `docs/SECURITY_AUDIT_2026-04-07.md` | Full audit with all 18 findings |
| `BUILD_PLAN_v3.md` | Remediation schedule and invariant tracker |
| `soulcontract.md` | Constitutional constraints |
| `CLAUDE.md` | Architecture, commands, design decisions |
