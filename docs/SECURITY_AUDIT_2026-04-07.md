# RTP Security Audit Report

**Auditor**: Senior Rust/Solana Security Auditor (Amp)
**Date**: 2026-04-07 (Hackathon Day 1)
**Scope**: Anchor Treasury Program, Coordinator (soulguard, router), Wings
**Build Status**: Swarm `cargo build` ✅ | `cargo test` 88/88 ✅

---

## Severity Summary Table

| # | Severity | File | Line | Issue |
|---|----------|------|------|-------|
| **C-1** | **CRITICAL** | `lib.rs` (treasury) | 367-368 | Phase evolution has **no on-chain threshold enforcement** — authority can advance to Humanity with $0 in treasury |
| **C-2** | **CRITICAL** | `lib.rs` (treasury) | 529-530 | `holders_recipient` is an unchecked `AccountInfo` — anyone can drain 70% of redistribution to any account |
| **C-3** | **CRITICAL** | `lib.rs` (treasury) | 536-538 | `dev_recipient` owner constraint compares `AccountInfo.owner` (program owner, i.e. Token program ID) against `project_dev_wallet` — always fails or is wrong |
| **H-1** | **HIGH** | `lib.rs` (treasury) | 138 | `treasury.authority = treasury.key()` — self-referential authority means `evolve_phase` requires the PDA itself to sign, which no one can do |
| **H-2** | **HIGH** | `lib.rs` (treasury) | 169-203 | `withdraw_fees` reads `vault.amount` before CPI but Anchor deserializes at entry — stale balance if vault was already reloaded |
| **H-3** | **HIGH** | `lib.rs` (treasury) | 150-158 | `min_runway_balance = 0` silently defaults to `DEFAULT_MIN_RUNWAY` instead of erroring — contradicts the comment "reject 0 explicitly" |
| **H-4** | **HIGH** | `soulguard.rs` | 306-309 | `spec()` method calls `unreachable!()` — will panic if ever called |
| **H-5** | **HIGH** | `soulguard.rs` | 298-302 | `exceeds_rollback_threshold()` hardcodes `0.05` instead of reading from spec — drift from spec is silent |
| **M-1** | **MEDIUM** | `lib.rs` (treasury) | 429 | `initialize` doesn't verify mint has `TransferFeeConfig` — can initialize treasury for a vanilla SPL mint |
| **M-2** | **MEDIUM** | `lib.rs` (treasury) | 425-470 | No `has_one` constraint on `treasury.mint` for Initialize — any mint can be passed after PDA derivation |
| **M-3** | **MEDIUM** | `soulguard.rs` | 131-144 | Rule 2 is dead code — Rule 1 (line 121) already catches all non-Coordinator↔non-Coordinator messages |
| **M-4** | **MEDIUM** | `router.rs` | 132-231 | Router doesn't call soulguard — it's only called in `Coordinator::process()`. Direct `router.route()` calls bypass soulguard |
| **M-5** | **MEDIUM** | `audit/mod.rs` | 123-163 | `stub_review()` auto-approves `EvolveProposal` — bypasses the 3-agent tribunal for the most sensitive payload type |
| **L-1** | **LOW** | `lib.rs` (treasury) | 591-593 | `HydrateSwarm.authority` is a `Signer` but is not checked against any stored authority — anyone can trigger hydration |
| **L-2** | **LOW** | `lib.rs` (treasury) | 198-200 | `vault.amount` in `withdraw_fees` isn't reloaded after CPI — `withdrawn` calculation uses pre-deserialization balance |
| **L-3** | **LOW** | `soulguard.rs` | 83-87 | `env!("CARGO_MANIFEST_DIR")` path resolution is compile-time — won't find `soulcontract.md` in production deployment |
| **L-4** | **LOW** | `Cargo.toml` (swarm) | 5 | `edition = "2024"` — Rust 2024 edition is very new, may cause CI/deployment issues on older toolchains |
| **I-1** | **INFO** | wings/ | — | Trading, Security, Knowledge, Futureproof are stubs — return `None` for most messages. Will silently drop messages at demo |
| **I-2** | **INFO** | `soulcontract_spec.rs` | 323 | Test uses hardcoded absolute path `/home/kt/kt/tabs/...` — will fail on any other machine |
| **I-3** | **INFO** | `lib.rs` (treasury) | 18-19,26-28 | `ECOSYSTEM_BPS`, `SUSTENANCE_CAP`, `ECOSYSTEM_CAP` are `#[allow(dead_code)]` — they're documented but never used programmatically |

---

## Pass 1 — Security & Correctness (Detailed Writeups)

### C-1: Phase Evolution Has No On-Chain Threshold Enforcement [CRITICAL]

**File**: `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs:348-371`
**Impact**: The `evolve_phase` instruction advances phases (Sustenance → Ecosystem → Humanity) with **zero validation of treasury balance**. The TODO at lines 367-368 acknowledges this. Any authority can call `evolve_phase` and jump to Humanity phase with an empty treasury. Phase transitions are **irreversible** — this permanently breaks the economic model.

**The README claims**: "Phase transitions are irreversible on-chain" and thresholds are $50k/$1M. The program enforces irreversibility but **not the thresholds**.

**Fix**: Add an oracle account (Pyth/Switchboard) to `EvolvePhase` and enforce:
```rust
let price = oracle.get_price()?;
let vault_value_usd = vault.amount * price / 10u64.pow(decimals);
match treasury.phase {
    Phase::Sustenance => require!(vault_value_usd >= SUSTENANCE_CAP, ...),
    Phase::Ecosystem => require!(vault_value_usd >= ECOSYSTEM_CAP, ...),
    _ => {}
}
```

For the hackathon devnet build, a simpler approach is to enforce vault token balance against the thresholds directly (treating the token as 1:1 USDC):
```rust
let vault_balance = ctx.accounts.treasury_vault.amount;
match treasury.phase {
    Phase::Sustenance => require!(vault_balance >= SUSTENANCE_CAP, TreasuryError::BelowThreshold),
    _ => {}
}
```

### C-2: `holders_recipient` Is Unchecked — Drain Risk [CRITICAL]

**File**: `lib.rs:529-530`
**Impact**: `holders_recipient` is a bare `AccountInfo` with only `/// CHECK: transfer destination, no data read`. There's no validation that this is a legitimate holder distribution pool. An attacker calls `check_redistribute` with their own token account as `holders_recipient` and receives 70% of the excess. This is a **direct fund theft vector**.

**Fix**: Either:
1. Make `holders_recipient` a PDA derived from the treasury (like the vault), or
2. Store the `holders_recipient` pubkey in `Treasury` state at initialization and add a `has_one` constraint, or
3. Use `InterfaceAccount<'info, TokenAccount>` with `token::mint = mint` at minimum.

Option 2 is recommended — add `holders_wallet: Pubkey` to `Treasury` state, accept it during `initialize`, and constrain with `has_one`.

### C-3: `dev_recipient`/`ecosystem_recipient` Owner Check Is Wrong [CRITICAL]

**File**: `lib.rs:536-538, 544-546`
**Impact**: The constraint `*dev_recipient.to_account_info().owner == treasury.project_dev_wallet` compares the **program that owns the account** (the Token program ID, e.g., `TokenkegQ...`) against `treasury.project_dev_wallet` (a wallet pubkey). These will **never match** in practice, making `check_redistribute` permanently uncallable. If somehow bypassed or reworked, the check is semantically wrong — it should compare the token account's **authority/owner field**, not the account's on-chain program owner.

**Fix**: Use Anchor's `token::authority` constraint:
```rust
#[account(
    mut,
    token::mint = mint,
    token::authority = treasury.project_dev_wallet,
)]
pub dev_recipient: InterfaceAccount<'info, TokenAccount>,
```

Same for `ecosystem_recipient`.

### H-1: Self-Referential Authority Makes `evolve_phase` Uncallable [HIGH]

**File**: `lib.rs:138`
**Impact**: `treasury.authority = treasury.key()` means the treasury PDA is its own authority. The `EvolvePhase` constraint at line 617 requires `phase_authority.key() == treasury.authority`, meaning the PDA itself must be the signer. But a PDA can only sign via CPI with seeds — there's no instruction that does this. `evolve_phase` is **dead code** as written.

**Fix**: Accept an `authority` pubkey during `initialize` (e.g., a Squads multisig) and store it:
```rust
treasury.authority = ctx.accounts.authority.key(); // or a dedicated governance key
```

### H-2: Stale Vault Balance in `withdraw_fees` [HIGH]

**File**: `lib.rs:176, 198`
**Impact**: `vault.amount` is deserialized by Anchor at instruction entry. After the CPI `withdraw_withheld_tokens_from_mint` mutates the vault's lamports/token balance, `vault.amount` still holds the **pre-CPI value**. The delta calculation `vault.amount.saturating_sub(balance_before)` will always be 0. `total_fees_withdrawn` is never incremented.

**Fix**: Reload the vault after CPI:
```rust
ctx.accounts.treasury_vault.reload()?;
let withdrawn = ctx.accounts.treasury_vault.amount.saturating_sub(balance_before);
```

### H-3: `min_runway_balance = 0` Silently Defaults Instead of Rejecting [HIGH]

**File**: `lib.rs:150-158`
**Impact**: The comment says "reject 0 explicitly" but the code does `if min_runway_balance == 0 { treasury.min_runway_balance = DEFAULT_MIN_RUNWAY }`. This silently accepts 0 and substitutes a default, which contradicts the stated intent and could confuse integrators.

**Fix**: Either error on 0 (`require!(min_runway_balance > 0, ...)`) or remove the misleading comment.

### H-4: `spec()` Calls `unreachable!()` [HIGH]

**File**: `soulguard.rs:306-309`
**Impact**: The `pub async fn spec()` method will panic at runtime if any code path calls it. It's public API — any future integration that calls `soulguard.spec()` will crash the swarm.

**Fix**: Remove the method entirely since `spec_snapshot()` exists as the replacement.

### H-5: `exceeds_rollback_threshold()` Ignores Parsed Spec [HIGH]

**File**: `soulguard.rs:298-302`
**Impact**: Hardcodes `0.05` instead of reading from `self.spec`. If soulcontract.md specifies a different threshold, the enforcement will silently diverge. The drift detection won't catch this since it only checks constraint names, not threshold values.

**Fix**: Make this async and read from the spec, or cache the threshold in a separate field on reload.

---

## Pass 2 — Code Quality & Hackathon Readiness

### M-1: `initialize` Doesn't Verify Mint Has `TransferFeeConfig`

**File**: `lib.rs:429`
**Impact**: A treasury can be initialized for a vanilla Token-2022 mint that has no TransferFeeConfig. `withdraw_fees` will then fail at CPI time with an unhelpful error. The `verify_adoption` instruction exists but is separate and optional.

**Fix**: Call the same deserialization logic from `verify_adoption` inside `initialize`, or add a constraint that checks for the extension.

### M-3: Soulguard Rule 2 Is Unreachable Dead Code

**File**: `soulguard.rs:131-144`
**Impact**: Rule 1 (line 121) rejects when `message.to != Coordinator AND message.from != Coordinator`. Rule 2 (line 133) checks `message.from != Coordinator AND message.to != Coordinator AND message.to != Audit` — the first two conditions are identical to Rule 1, so Rule 2 is never reached. Harmless but misleading during review.

### M-4: Router Bypass Allows Soulguard Circumvention

**File**: `router.rs:132-231`
**Impact**: `Router::route()` is public and doesn't call soulguard. Only `Coordinator::process()` enforces the pipeline. Any code that obtains a `Router` reference can bypass soulguard entirely.

**Fix**: Make the router `pub(crate)` or enforce soulguard within the router itself.

### M-5: `stub_review()` Auto-Approves Evolve Proposals

**File**: `audit/mod.rs:151-160`
**Impact**: `EvolveProposal` payloads are auto-approved with `RiskLevel::Medium`. If the stub is used in the demo path (which is likely since the full tribunal requires wiring), architecture changes bypass the safety net entirely.

### I-1: Four Wings Are Stubs That Drop Messages

Trading only handles `TradingConfig`. Security and Futureproof only handle `Heartbeat`. Knowledge only handles `KnowledgeQuery`. Any other message type returns `None` and is silently lost. At demo time, if a judge sends a proposal through the Trading Wing and expects execution, nothing happens.

### I-2: Hardcoded Absolute Path in Test

`soulcontract_spec.rs:323` — path `/home/kt/kt/tabs/resilient-token-protocol/soulcontract.md` will fail on CI and any other machine. Use `env!("CARGO_MANIFEST_DIR")` or skip the test conditionally.

---

## Pass 3 — Architecture Gaps

### Does the Trust Model Hold Under Adversarial Conditions?

**Partially.** The swarm trust model (Coordinator as sole mediator, soulguard gating) is sound in design but has these gaps:

1. **Soulguard bypass**: `Router::route()` is public and doesn't call soulguard. Only `Coordinator::process()` enforces the pipeline. Any code that obtains a `Router` reference can bypass soulguard entirely. The router should be `pub(crate)` at minimum.

2. **Audit bypass via stub**: The `stub_review()` path auto-approves non-critical proposals without tribunal review. In production, there must be no code path that routes around the tribunal.

3. **spec reload without signature check**: `soulguard.reload()` takes any file path and replaces the active spec. The soulcontract says "amendments require human cryptographic signature" but there's no signature verification on reload — any wing that can trigger a reload can change the rules.

### Is the Python ↔ Rust Bridge Typed Correctly?

**There is no `bridge.rs` file.** The CLAUDE.md references `rtp/swarm/src/wings/trading/bridge.rs` for the Python↔Rust typed interface, but this file doesn't exist. The bridge is an architecture gap — currently the Python yield brain has no typed interface to the Rust swarm. For the hackathon, this is the **highest integration risk**.

### Are the 10 On-Chain Invariants Actually Enforced?

| # | Invariant | Enforced? | Notes |
|---|-----------|-----------|-------|
| 1 | PDA owns treasury | ✅ | PDA seeds properly derived |
| 2 | TransferFeeConfig immutable | ⚠️ | `verify_adoption` checks it but is READ-ONLY and optional — not called during `initialize` |
| 3 | CPI-only transfers | ✅ | All transfers use `transfer_checked` CPI |
| 4 | Agent proposes, human approves irreversible | ❌ | `evolve_phase` authority is self-referential PDA (uncallable), and no human approval step exists on-chain |
| 5 | No SOL liquidation | ✅ | By construction — only token transfers |
| 6 | Phase transitions irreversible | ✅ | Match statement only allows forward transitions |
| 7 | Soulcontract amendments require human sig | ⚠️ | Off-chain only (swarm rejects in-memory), no on-chain enforcement |
| 8 | Auto-rollback >5% degradation | ⚠️ | Off-chain only (Evolve Wing), threshold is hardcoded not read from spec |
| 9 | Self-hydration only if >90-day runway | ✅ | `hydrate_swarm` enforces `min_runway_balance` |
| 10 | Yield brain strategies black-boxed | ✅ | By construction — ships as binary |

**Bottom line**: 4 of 10 invariants are fully enforced on-chain. 3 are partial (off-chain only or optional). 3 have bugs that prevent enforcement. A judge who reads the README and tests the program will find that phase thresholds, holder distribution, and dev/ecosystem routing are all broken or unprotected.

---

## Priority Fix List (Hackathon)

1. **C-2/C-3**: Fix recipient account validation in `CheckRedistribute` (blocks demo)
2. **C-1**: Add vault-balance check for `evolve_phase` (devnet: treat token as 1:1 USDC)
3. **H-1**: Fix authority to be the initializer, not the PDA itself
4. **H-2**: Add `vault.reload()` after CPI in `withdraw_fees`
5. **H-4**: Delete the `spec()` method
6. **M-1**: Call `verify_adoption` from within `initialize` or add TransferFeeConfig check
7. Create `bridge.rs` with at minimum a typed JSON schema for Python↔Rust
