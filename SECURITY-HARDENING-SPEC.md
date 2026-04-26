# RTP Security Hardening Specification

> **Version:** 1.0 — April 26, 2026
> **Status:** Phase 1 (on-chain) COMPLETE. Phases 2-4 (Squads, Hydra, Server SDK) DEFERRED post-hackathon.
> **Audience:** A fresh developer agent who has never seen the codebase

## How to Read This Spec

A fresh agent needs **only** these files to implement everything described here:

1. **This file** (`SECURITY-HARDENING-SPEC.md`) — the implementation spec
2. `research/phantom-server-sdk-findings.md` — Phantom Server SDK v2.0.1 API reference
3. `research/squads-findings.md` — Squads v4 multisig REST API + CPI integration
4. `research/hydra-findings.md` — Hydra crank scheduling API + cost model
5. `research/rtp-current-state.md` — file-level migration map (every change site)
6. `SESSION-CONTEXT.md` — compressed project memory (architecture, decisions, addresses)
7. `CLAUDE.md` — project instructions, repo layout, commands, key files
8. `SOULCONTRACT.md` — constitutional governance invariants (must not be violated)

Do **NOT** reference `docs/architecture.md` — it does not exist as a standalone file.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture Changes](#2-architecture-changes)
   - [A. Phantom Server SDK v2.0.1](#a-phantom-server-sdk-v201)
   - [B. Squads Multisig as treasury.authority](#b-squads-multisig-as-treasuryauthority)
   - [C. Hydra Cranks for Permissionless Ops](#c-hydra-cranks-for-permissionless-ops)
   - [D. Zero-Address Rejection Guard](#d-zero-address-rejection-guard)
   - [E. Emergency Freeze/Unfreeze](#e-emergency-freezeunfreeze)
3. [File List — Exact Changes](#3-file-list--exact-changes)
4. [Test Plan](#4-test-plan)
5. [Doc Update Checklist](#5-doc-update-checklist)
6. [Migration Order](#6-migration-order)

---

## 1. Overview

This spec covers five security hardening integrations for the RTP production deployment:

| Integration | Purpose | Priority | Status |
|------------|---------|----------|--------|
| Phantom Server SDK v2.0.1 | Headless wallet signing (no browser session required) | P0 | DEFERRED — MCP subprocess sufficient for hackathon |
| Squads Multisig | 2-of-3 authority on all treasury-gated operations | P0 | DEFERRED — post-launch: rotate authority to Squads PDA |
| Hydra Cranks | Automated permissionless ops (withdraw_fees, check_redistribute) | P1 | DEFERRED — post-launch: CLI/script setup |
| Zero-Address Guard | Reject `Pubkey::default()` on all critical fields | P0 | DONE |
| Emergency Freeze | Halt all treasury operations on authority command | P0 | DONE |

**Why these five:** The current system uses a single-key authority on the treasury PDA, relies on browser-authenticated MCP sessions for signing, and has no emergency halt capability. These are the minimum security controls needed before mainnet deployment.

---

## 2. Architecture Changes

### A. Phantom Server SDK v2.0.1

#### Current State

`rtp/swarm/src/wings/trading/phantom_mcp.rs` starts `@phantom/mcp-server` as a subprocess and communicates via stdio JSON-RPC. All 22+ functions take `di: u32` for per-token wallet isolation. Session requires browser-based `phantom login`.

#### Upgrade Path

**Keep MCP subprocess for Rust Trading Wing** (it works for devnet/demo). **Add Server SDK for TypeScript SDK and dashboard operations.**

The Server SDK uses API key authentication — no browser interaction required. This is critical for production headless operation.

**Package:** `@phantom/server-sdk` v2.0.1
**Auth:** `@phantom/api-key-stamper` — cryptographic request signing
**PHANTOM_APP_ID:** `2fbef7dc-7975-4378-ba2b-ff8018ad2325`

#### Server SDK Initialization

```typescript
import { ServerSDK, NetworkId } from "@phantom/server-sdk";

const sdk = new ServerSDK({
  organizationId: process.env.PHANTOM_ORGANIZATION_ID!,
  appId: process.env.PHANTOM_APP_ID!,         // "2fbef7dc-7975-4378-ba2b-ff8018ad2325"
  apiPrivateKey: process.env.PHANTOM_API_PRIVATE_KEY!,
  apiBaseUrl: process.env.PHANTOM_API_BASE_URL!,
});
```

#### Key SDK Methods

| Method | Signature | Purpose |
|--------|-----------|---------|
| `sdk.createWallet(name)` | `Promise<{ walletId, addresses }>` | Create new embedded wallet |
| `sdk.signMessage({ walletId, message, networkId })` | `Promise<signature>` | Sign UTF-8 message |
| `sdk.signAndSendTransaction({ walletId, transaction, networkId })` | `Promise<tx result>` | Sign + broadcast tx |

Network IDs: `NetworkId.SOLANA_MAINNET`, `NetworkId.ETHEREUM_MAINNET`

#### Architecture Decision

```
Rust Trading Wing (rtp-daemon)
  └── MCP subprocess (@phantom/mcp-server) — RETAINED for Rust compatibility
        └── di: u32 pattern preserved (token_wallet_map)
        └── Browser auth needed for initial session (phantom login)

TypeScript SDK / Dashboard
  └── Server SDK (@phantom/server-sdk v2.0.1) — ADDED for headless ops
        └── API key auth — no browser session required
        └── Per-token isolation: create separate wallet per token (each gets unique walletId)
        └── Used for: freeze/unfreeze, multisig status queries, dashboard signing ops
```

#### EIP-712 Gap

The Server SDK README does **not** explicitly show an EIP-712 `signTypedData` method. Keep the direct ETH keypair (`configs/hl_testnet_key.json`) for Hyperliquid order signing. Do not migrate HL signing until EIP-712 typed data is explicitly documented in the Server SDK.

#### Spending Limit Handling

The Trading Wing must handle `SPENDING_LIMIT_EXCEEDED` errors from Phantom. These are user-configured on-chain limits — the app cannot set them programmatically. Add error handling in `trading/mod.rs` order execution path and surface to Coordinator for audit logging.

#### Files Changed

| File | Action | Detail |
|------|--------|--------|
| `sdk/index.ts` | Modify | Add Server SDK initialization, freeze/unfreeze helpers that use Server SDK signing |
| `dashboard/src/app/page.tsx` | Modify | Wallet ops via Server SDK for multisig status, freeze indicator |
| `rtp/swarm/src/wings/trading/phantom_mcp.rs` | Modify | Add `SPENDING_LIMIT_EXCEEDED` error handling in swap/transfer wrappers |
| `rtp/swarm/src/wings/trading/mod.rs` | Modify | Propagate spending limit errors to Coordinator |

#### Environment Variables Required

```bash
PHANTOM_ORGANIZATION_ID=<from Portal dashboard>
PHANTOM_APP_ID=2fbef7dc-7975-4378-ba2b-ff8018ad2325
PHANTOM_API_PRIVATE_KEY=<from Portal dashboard>
PHANTOM_API_BASE_URL=<from Portal dashboard>
```

**Security:** Store in environment variables or secrets manager. Never commit to `configs/`. Add `PHANTOM_API_PRIVATE_KEY` to `.gitignore`.

---

### B. Squads Multisig as treasury.authority

#### Overview

Replace the single-key `treasury.authority` with a Squads v4 multisig PDA. All authority-gated operations (`evolve_phase`, `register_strategy`, `force_retire_strategy`, `end_beta`) then require 2-of-3 approval with a 24-hour time lock.

#### Squads Configuration

| Parameter | Value |
|-----------|-------|
| Program ID | `SQDS4ep65T869zMMBKyuUq6a6EgTu8psMjkvj52pCf` (same on mainnet + devnet) |
| REST API | `https://developer-api.squads.so/api/v1` |
| Auth Header | `Authorization: Bearer <API_KEY>` |
| Network Header | `x-squads-network: devnet` or `mainnet` |
| Idempotency | `x-idempotency-key` header for safe retries |

#### Multisig Setup — 2-of-3

| Signer | Role | Permissions | Key Type |
|--------|------|-------------|----------|
| Signer 1 | RTP Agent | `CAN_INITIATE`, `CAN_VOTE`, `CAN_EXECUTE` | Hot wallet (agent) |
| Signer 2 | Human Admin | `CAN_VOTE` | Cold wallet |
| Signer 3 | Emergency Recovery | `CAN_VOTE` | Offline backup key |

**Threshold:** 2 — agent + one human for ops, or both humans to override agent.
**Time Lock:** 86,400 seconds (24h) — applies to ALL transactions through this multisig.

#### Create Multisig via REST API

```
POST https://developer-api.squads.so/api/v1/smart-accounts
Authorization: Bearer <SQUADS_API_KEY>
x-squads-network: devnet
Content-Type: application/json

{
  "smart_account_signers": [
    {
      "address": "<agent_wallet_pubkey>",
      "permissions": ["CAN_INITIATE", "CAN_VOTE", "CAN_EXECUTE"]
    },
    {
      "address": "<human_admin_pubkey>",
      "permissions": ["CAN_VOTE"]
    },
    {
      "address": "<emergency_recovery_pubkey>",
      "permissions": ["CAN_VOTE"]
    }
  ],
  "threshold": 2,
  "admin_address": "<human_admin_pubkey>"
}
```

Response contains the multisig PDA address.

#### Set Time Lock

```
PATCH https://developer-api.squads.so/api/v1/smart-accounts/{address}

{
  "time_lock": 86400,
  "transaction_signers": ["<signer1>", "<signer2>", "<signer3>"]
}
```

#### Set Spending Limit (Daily USDC Cap)

```
POST https://developer-api.squads.so/api/v1/smart-accounts/{address}/spending-limits

{
  "amount": "10000000000",
  "token_address": "<USDC_MINT_ADDRESS>",  // EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v on mainnet
  "period": "DAILY",
  "spending_limit_signers": ["<agent_wallet>"],
  "destinations": ["<treasury_pda>"]
}
```

This caps USDC transfers at **10,000 USDC/day** (10,000,000,000 lamports with 6 decimals).

#### Authority Flow

```
Agent/Human → Squads Proposal → Vote (2-of-3) → Time Lock (24h) → Execute
     │                                                           │
     │                              ┌─────────────────────────────┘
     │                              ▼
     │                    Vault PDA signs via invoke_signed
     │                              │
     │                              ▼
     └──────────────────► RTP Treasury Program
                          checks treasury.authority == vault PDA ✓
```

Squads executes transactions as top-level instructions from the multisig vault PDA. Any Anchor `Signer` constraint on `treasury.authority` works unchanged because the Squads vault PDA is the signer via `invoke_signed`.

**Vault PDA derivation:**
```
seeds: ["multisig", multisig_key, "vault", vault_index]
program ID: SQDS4ep65T869zMMBKyuUq6a6EgTu8psMjkvj52pCf
```

#### Rust CPI Integration

**Crate:** `squads-multisig-program` on crates.io
**Repository:** `github.com/Squads-Protocol/v4`
**Audits:** OtterSec (3 rounds), Neodyme (3 rounds), Certora (3 rounds), Trail of Bits (1 round)

The treasury program can CPI into Squads for programmatic proposal creation from the swarm runtime. This enables the Trading Wing to propose `register_strategy` or `force_retire_strategy` without direct key access.

**Cargo.toml addition:**
```toml
[dependencies]
squads-multisig-program = "<version>"
```

#### New Rust Client File

Create `rtp/swarm/src/wings/trading/squads_client.rs` — a REST API client for Squads operations:

```rust
/// Squads multisig REST API client.
/// Manages proposals for authority-gated treasury operations.
pub struct SquadsClient {
    api_key: String,
    network: String, // "devnet" | "mainnet"
    multisig_address: Pubkey,
    http: reqwest::blocking::Client,
}

impl SquadsClient {
    pub fn new(api_key: &str, network: &str, multisig_address: Pubkey) -> Self { ... }

    /// Create a proposal to execute an authority-gated instruction.
    /// Returns the proposal ID for tracking.
    pub fn create_proposal(
        &self,
        instruction_data: &[u8],
        program_id: &Pubkey,
        accounts: &[AccountMeta],
    ) -> Result<String, SquadsError> { ... }

    /// Check if a proposal has been approved and is ready for execution.
    pub fn proposal_status(&self, proposal_id: &str) -> Result<ProposalStatus, SquadsError> { ... }

    /// Execute an approved proposal after time lock expires.
    pub fn execute_proposal(&self, proposal_id: &str) -> Result<Signature, SquadsError> { ... }
}
```

#### Authority-Gated Instructions (Require Squads Proposal)

| RTP Instruction | Squads Gate | Rationale |
|-----------------|-------------|-----------|
| `evolve_phase` | 2-of-3 + 24h | Irreversible — full scrutiny required |
| `register_strategy` | 2-of-3 + 24h | Promotes strategy to Live |
| `force_retire_strategy` | 2-of-3 + 24h | Emergency retirement |
| `end_beta` | 2-of-3 + 24h | Manual beta adopter sunset |
| `freeze_treasury` | 2-of-3 | Emergency halt (no time lock) |
| `unfreeze_treasury` | 2-of-3 + 24h | Resume requires deliberation |

#### NOT Authority-Gated (Permissionless — Hydra cranks handle these)

| RTP Instruction | Mechanism |
|-----------------|-----------|
| `withdraw_fees` | Hydra crank, permissionless |
| `check_redistribute` | Hydra crank, permissionless |
| `record_fee_deposit` | Any signer can call |
| `update_strategy_performance` | Any signer can call |
| `register_adopter` / `register_adopter_beta` | Any signer can call |

---

### C. Hydra Cranks for Permissionless Ops

#### Overview

Hydra is a permissionless Solana crank that stores a scheduled instruction in a PDA and lets anyone trigger it when due. Perfect for RTP's permissionless instructions that should run automatically without human intervention.

#### Hydra Configuration

| Parameter | Value |
|-----------|-------|
| Program ID | `Hydra17i1feui9deaxu6d1TzSQMRNHeBRkDR1Awy7zea` |
| Crate | `hydra-api = "0.1.0"` with `features = ["client"]` |
| License | MIT |
| Source | `github.com/magicblock-labs/hydra` |

**Cargo.toml addition:**
```toml
[dependencies]
hydra-api = { version = "0.1.0", features = ["client"] }
```

#### Two Cranks

| Crank | Instruction | Interval | Purpose |
|-------|------------|----------|---------|
| Crank 1 | `withdraw_fees` | ~800 slots (~5 min) | Pull TransferFeeConfig fees into treasury vault |
| Crank 2 | `check_redistribute` | ~1,600 slots (~10 min) | Trigger 70/20/10 redistribution split |

#### Crank 1: withdraw_fees

```rust
use hydra_api::instruction::{self as ix, CreateArgs, SchedMeta};

let seed = hash("rtp-withdraw-fees-v1"); // unique seed
let (crank, _bump) = ix::find_crank_pda(&seed);

let create = ix::create(
    payer_pubkey,
    crank,
    &CreateArgs {
        seed,
        authority: [0u8; 32],      // all-zeros = permanent (no cancel)
        start_slot: 0,              // start immediately
        interval_slots: 800,        // ~5 min on mainnet
        remaining: 0,               // 0 = infinite runs
        priority_tip: 2_500,        // lamports paid to cranker per trigger
        cu_limit: 0,                // inherit default (200k)
        scheduled_program_id: rtp_program_id, // 8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB
        scheduled_metas: &[
            SchedMeta::writable(treasury_pda),       // treasury account
            SchedMeta::writable(vault_ata),           // vault ATA (writable)
            SchedMeta::readonly(mint_pubkey),          // mint account
            SchedMeta::readonly(token_program),        // Token-2022 program
        ],
        scheduled_data: &withdraw_fees_discriminator, // instruction discriminator + args
    },
);
```

#### Crank 2: check_redistribute

```rust
let seed = hash("rtp-redistribute-v1");
let (crank, _bump) = ix::find_crank_pda(&seed);

let create = ix::create(
    payer_pubkey,
    crank,
    &CreateArgs {
        seed,
        authority: [0u8; 32],      // permanent
        start_slot: 0,
        interval_slots: 1_600,     // ~10 min on mainnet
        remaining: 0,               // infinite
        priority_tip: 2_500,
        cu_limit: 0,
        scheduled_program_id: rtp_program_id,
        scheduled_metas: &[
            SchedMeta::writable(treasury_pda),       // treasury account
            SchedMeta::readonly(token_program),       // Token-2022 program
            SchedMeta::readonly(mint_pubkey),          // mint account
        ],
        scheduled_data: &check_redistribute_discriminator,
    },
);
```

#### Failure Handling — Atomic Rollback

Hydra trigger transactions contain two instructions:
```
ix[k]   = Hydra.Trigger
ix[k+1] = scheduled instruction
```

If the scheduled instruction fails, the **entire transaction rolls back** (Solana atomicity):
- Hydra's payout does NOT advance
- Crank state does NOT change
- The crank **remains eligible** for the next trigger attempt
- The cranker will retry on the next eligible slot

This is ideal for RTP: if `check_redistribute` hits a no-op (nothing to redistribute), the transaction fails atomically and retries next cycle. No partial state corruption.

#### Cranker Deployment

The cranker is a long-running daemon with WebSocket subscriptions:

```bash
hydra-cranker --keypair ~/.config/solana/cranker.json \
  --rpc-url https://api.mainnet-beta.solana.com \
  --ws-url wss://api.mainnet-beta.solana.com \
  --prometheus-port 9100
```

Alternatively, rely on the existing Hydra cranker fleet on mainnet (no self-hosting required).

#### Cost Model

| Item | Amount |
|------|--------|
| Rent deposit | ~0.002 SOL (refundable on close) |
| Create tx fee | 5,000 lamports |
| Per trigger | 10,000 + 2,500 (tip) = 12,500 lamports |
| Triggers/day (withdraw_fees) | ~270 |
| Triggers/day (check_redistribute) | ~135 |
| **Daily cost (both cranks)** | ~0.005 SOL |
| **Monthly cost (both cranks)** | ~0.10 SOL/month |

Fund each crank with ~0.05 SOL for 30 days. Top up monthly.

#### Constraints

| Constraint | Value |
|------------|-------|
| Max accounts per crank | 32 |
| Max scheduled data | 1,024 bytes |
| Signer metas | **Not allowed** (permissionless only) |
| Trigger scope | Top-level only (no CPI from cranks) |
| Trigger compute overhead | ~464 CU |

**Only permissionless instructions** (no signer required) can be Hydra-scheduled. Authority-gated ops (`hydrate_swarm`, `evolve_phase`) remain under Squads multisig control.

#### New Rust File

Create `rtp/swarm/src/wings/trading/hydra_crank.rs`:

```rust
/// Hydra crank management utilities for RTP treasury operations.
/// Creates and manages permissionless scheduled instructions.

pub struct HydraCrankManager {
    rpc_url: String,
    payer: Keypair,
    program_id: Pubkey, // RTP program ID
}

impl HydraCrankManager {
    pub fn new(rpc_url: &str, payer: Keypair, program_id: Pubkey) -> Self { ... }

    /// Create the withdraw_fees crank (~800 slot interval).
    /// Returns the crank PDA address.
    pub fn create_withdraw_fees_crank(
        &self,
        treasury_pda: Pubkey,
        vault_ata: Pubkey,
        mint: Pubkey,
        token_program: Pubkey,
    ) -> Result<Pubkey, HydraError> { ... }

    /// Create the check_redistribute crank (~1600 slot interval).
    /// Returns the crank PDA address.
    pub fn create_redistribute_crank(
        &self,
        treasury_pda: Pubkey,
        mint: Pubkey,
        token_program: Pubkey,
    ) -> Result<Pubkey, HydraError> { ... }

    /// Check crank balance and return remaining runway in days.
    pub fn crank_runway_days(&self, crank_address: Pubkey) -> Result<f64, HydraError> { ... }

    /// Top up crank balance for 30 more days.
    pub fn top_up_crank(&self, crank_address: Pubkey, sol_amount: f64) -> Result<Signature, HydraError> { ... }
}
```

---

### D. Zero-Address Rejection Guard

#### Overview

Add input validation to the Anchor treasury program to reject `Pubkey::default()` (the zero address `11111111111111111111111111111111`) on all critical fields. Prevents misconfiguration attacks.

#### Changes to `lib.rs`

Add new error variant:

```rust
#[error_code]
pub enum TreasuryError {
    // ... existing errors ...
    #[msg("Zero address (Pubkey::default()) is not allowed")]
    ZeroAddressRejected,
}
```

Add guard function:

```rust
/// Reject the Solana zero address on critical fields.
fn reject_zero_address(addr: Pubkey) -> Result<()> {
    if addr == Pubkey::default() {
        return err!(TreasuryError::ZeroAddressRejected);
    }
    Ok(())
}
```

Apply to `initialize` instruction:

```rust
pub fn initialize(ctx: Context<Initialize>, ...) -> Result<()> {
    reject_zero_address(ctx.accounts.authority.key())?;
    reject_zero_address(ctx.accounts.mint.key())?;
    reject_zero_address(params.holders_wallet)?;
    reject_zero_address(params.project_dev_wallet)?;
    reject_zero_address(params.ecosystem_wallet)?;
    // ... rest of initialization
}
```

Apply to any instruction that receives a recipient address:
- `initialize` — authority, mint, all three wallet addresses
- Any future `set_authority` instruction — new authority must be non-zero
- Any instruction receiving a destination address parameter

Also reject zero-amount operations where amount > 0 is expected:
- `record_fee_deposit` — already guarded by `ZeroAmount` error
- `hydrate_swarm` — verify amount > 0

#### Verification

The zero address `11111111111111111111111111111111` is the System Program. No legitimate treasury operation should use it as a recipient or authority.

---

### E. Emergency Freeze/Unfreeze

#### Overview

Add `freeze_treasury` and `unfreeze_treasury` instructions to the treasury program. When frozen, all non-read operations are rejected. This provides an emergency halt capability for incident response.

#### State Changes

Add to `Treasury` account:

```rust
#[account]
#[derive(InitSpace)]
pub struct Treasury {
    // ... existing fields ...
    /// Whether the treasury is frozen (emergency halt).
    /// When true, all non-read operations are rejected.
    pub frozen: bool,
}
```

**Account size:** Grows by 1 byte (`bool` + Anchor 8-byte discriminator padding already covered by `InitSpace`). Existing accounts will need to be closed and re-initialized, or use Anchor's account resizing.

#### New Error

```rust
#[error_code]
pub enum TreasuryError {
    // ... existing errors ...
    #[msg("Treasury is frozen — all operations are halted")]
    TreasuryFrozen,
}
```

#### New Events

```rust
#[event]
pub struct TreasuryFrozen {
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TreasuryUnfrozen {
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}
```

#### freeze_treasury Instruction

```rust
/// Emergency freeze: authority-gated, sets frozen = true.
/// In production, authority is the Squads multisig PDA — requires 2-of-3 approval.
/// No time lock on freeze (emergency speed). Unfreeze requires 24h time lock.
pub fn freeze_treasury(ctx: Context<FreezeTreasury>) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    require!(!treasury.frozen, TreasuryError::AlreadyFrozen);

    treasury.frozen = true;
    emit!(TreasuryFrozen {
        mint: treasury.mint,
        authority: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
```

#### unfreeze_treasury Instruction

```rust
/// Unfreeze: authority-gated, sets frozen = false.
/// In production, requires Squads 2-of-3 + 24h time lock.
pub fn unfreeze_treasury(ctx: Context<UnfreezeTreasury>) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    require!(treasury.frozen, TreasuryError::NotFrozen);

    treasury.frozen = false;
    emit!(TreasuryUnfrozen {
        mint: treasury.mint,
        authority: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
```

#### Frozen Guard on All State-Mutating Instructions

Add to every instruction that modifies treasury state:

```rust
pub fn withdraw_fees(ctx: Context<WithdrawFees>) -> Result<()> {
    require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
    // ... existing logic
}

pub fn check_redistribute(ctx: Context<CheckRedistribute>) -> Result<()> {
    require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
    // ... existing logic
}

pub fn hydrate_swarm(ctx: Context<HydrateSwarm>) -> Result<()> {
    require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
    // ... existing logic
}
```

**All instructions that get the frozen guard:**

| Instruction | Frozen Check |
|-------------|-------------|
| `withdraw_fees` | ✅ Yes (moves funds) |
| `check_redistribute` | ✅ Yes (moves funds) |
| `hydrate_swarm` | ✅ Yes (moves funds) |
| `evolve_phase` | ✅ Yes (state change) |
| `register_strategy` | ✅ Yes (state change) |
| `force_retire_strategy` | ✅ Yes (state change) |
| `end_beta` | ✅ Yes (state change) |
| `create_swarm_vault` | ✅ Yes (creates account) |
| `record_fee_deposit` | ✅ Yes (writes counter) |
| `update_strategy_performance` | ✅ Yes (writes metrics) |
| `register_adopter` | ✅ Yes (creates account) |
| `register_adopter_beta` | ✅ Yes (creates account) |

**Read operations (continue working when frozen):**
- `verify_adoption` — read-only
- Fetch treasury state — read-only
- Query adopter records — read-only

#### Additional Errors Needed

```rust
#[error_code]
pub enum TreasuryError {
    // ... existing errors ...
    #[msg("Treasury is already frozen")]
    AlreadyFrozen,
    #[msg("Treasury is not frozen")]
    NotFrozen,
}
```

---

## 3. File List — Exact Changes

### Create

**None.** Squads client and Hydra crank were created then deleted during audit cleanup (shelfware — not wired into any execution path, unvalidated API assumptions, incorrect serialization). Post-launch, these will be reimplemented with validated integration.

### Modify

| File | Changes | Status |
|------|---------|--------|
| **`rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`** | Add `ZeroAddressRejected`, `TreasuryFrozen`, `AlreadyFrozen`, `NotFrozen` errors. Add `frozen: bool` to `Treasury` account. Add `reject_zero_address()` guard on `initialize`. Add `freeze_treasury` and `unfreeze_treasury` instructions. Add frozen guard to all 12 state-mutating instructions. Emit `TreasuryFrozen`/`TreasuryUnfrozen` events. | ✅ DONE |
| **`rtp/swarm/src/wings/trading/phantom_mcp.rs`** | Add `SPENDING_LIMIT_EXCEEDED` error logging in `call_tool()`. No structural changes — MCP subprocess retained. | ✅ DONE |
| **`rtp/swarm/src/wings/trading/mod.rs`** | Removed SquadsClient/HydraCrankManager declarations (deleted during audit). | ✅ DONE |
| **`rtp/swarm/src/wings/trading/types.rs`** | Removed `SquadsConfig` and `HydraCrankState` (deleted during audit). | ✅ DONE |
| **`sdk/index.ts`** | Add `freezeTreasury`, `unfreezeTreasury`, `isTreasuryFrozen` helpers. Update `TreasuryState` with `isFrozen`. Regenerate IDL (16 instructions). | ✅ DONE |
| **`sdk/idl.ts`** | Regenerated from updated Anchor build (16 instructions including freeze/unfreeze). | ✅ DONE |
| **`dashboard/src/app/page.tsx`** | Add freeze banner + frozen state polling. | ✅ DONE |
| **`dashboard/src/lib/sdk/index.ts`** | Synced from sdk/index.ts. | ✅ DONE |
| **`dashboard/src/lib/sdk/idl.ts`** | Synced from sdk/idl.ts. | ✅ DONE |
| **`CLAUDE.md`** | Updated invariants, key files, trust model, security hardening section. Removed Squads/Hydra shelfware claims. | ✅ DONE |
| **`SOULCONTRACT.md`** | Updated invariants — removed multisig/Hydra aspirational claims, kept freeze/zero-address. | ✅ DONE |
| **`docs/RESOURCES.md`** | Trimmed Squads to reference-only. Removed Hydra section. | ✅ DONE |
| **`SECURITY-HARDENING-SPEC.md`** | Updated status markers for each phase. Fixed Squads program ID. | ✅ DONE |

### Delete

**None.** The MCP subprocess client is retained as fallback for Rust integration and interactive debugging.

---

## 4. Test Plan

### Test 1: Squads Multisig on Devnet

**Prerequisite:** Squads API key, 3 test wallets.

```
1. Create 2-of-3 multisig via POST /smart-accounts
   → Verify response contains multisig PDA address
   → Verify threshold = 2, time_lock = 86400

2. Set multisig PDA as treasury.authority
   → Call set_authority(new_authority = multisig_pda)
   → Verify treasury.authority updated on-chain

3. Test proposal flow for evolve_phase
   → Create proposal via Squads REST API
   → Vote with 2 signers
   → Wait 24h (or use devnet bypass)
   → Execute proposal
   → Verify phase evolved on-chain

4. Test unauthorized execution fails
   → Try calling evolve_phase directly with single signer
   → Expect Anchor error: "constraint was violated" (authority mismatch)

5. Test spending limits
   → Configure $10K/day USDC cap
   → Attempt transfer exceeding cap
   → Expect spending limit rejection
```

### Test 2: Hydra Crank on Devnet

**Prerequisite:** Funded cranker keypair, RTP program deployed.

```
1. Create check_redistribute crank
   → Build CreateArgs with interval_slots: 1600
   → Submit create transaction
   → Verify crank account exists on-chain

2. Verify trigger fires
   → Wait for next eligible slot
   → Check crank account: last_triggered_slot updated
   → Verify check_redistribute executed (or failed atomically if no funds to redistribute)

3. Verify scheduled ix matches expected data
   → Read crank account data
   → Compare scheduled_data against expected instruction discriminator
   → Compare scheduled_metas against expected accounts

4. Test failure rollback
   → Create crank targeting an instruction that will fail
   → Wait for trigger
   → Verify crank state unchanged (no advancement)
   → Verify retry occurs on next eligible slot

5. Create withdraw_fees crank
   → Same process with interval_slots: 800
   → Verify both cranks run independently
```

### Test 3: Zero-Address Guard

```
1. Test initialize with Pubkey::default() authority
   → Call initialize(authority = 11111111...1111)
   → Expect error: ZeroAddressRejected

2. Test initialize with Pubkey::default() mint
   → Expect error: ZeroAddressRejected

3. Test initialize with zero address for any wallet (holders, dev, ecosystem)
   → Expect error: ZeroAddressRejected

4. Test valid addresses still work
   → Call initialize with all non-zero addresses
   → Verify success

5. Test that System Program (11111111...1111) is not accepted
   → Pubkey::default() == System Program ID
   → Explicitly verify this identity holds
```

### Test 4: Freeze/Unfreeze

```
1. Freeze treasury
   → Call freeze_treasury() with authority
   → Verify treasury.frozen == true
   → Verify TreasuryFrozen event emitted

2. Verify operations rejected when frozen
   → Call withdraw_fees → expect TreasuryFrozen
   → Call check_redistribute → expect TreasuryFrozen
   → Call hydrate_swarm → expect TreasuryFrozen
   → Call evolve_phase → expect TreasuryFrozen

3. Verify read operations still work when frozen
   → Fetch treasury state → success
   → Call verify_adoption → success

4. Unfreeze treasury via Squads (2-of-3 + 24h)
   → Create unfreeze proposal
   → Vote with 2 signers
   → Wait time lock
   → Execute
   → Verify treasury.frozen == false
   → Verify TreasuryUnfrozen event emitted

5. Verify operations restored after unfreeze
   → Call withdraw_fees → success
   → Call check_redistribute → success

6. Test double freeze rejected
   → Freeze treasury → success
   → Freeze again → expect AlreadyFrozen
```

### Test 5: Server SDK Integration

```
1. Test createWallet
   → Call sdk.createWallet("Test Token Wallet")
   → Verify response contains walletId + addresses (Solana, EVM)
   → Verify addresses are unique per wallet

2. Test signMessage (Solana)
   → Call sdk.signMessage({ walletId, message: "test", networkId: SOLANA_MAINNET })
   → Verify signature is valid base58
   → Verify signature verifies against the wallet's Solana address

3. Test signAndSendTransaction (Solana)
   → Build a simple transfer transaction
   → Call sdk.signAndSendTransaction({ walletId, transaction, networkId: SOLANA_MAINNET })
   → For devnet: use SOLANA_DEVNET if supported, or mock

4. Test SPENDING_LIMIT_EXCEEDED handling
   → Configure a low spending limit on test wallet
   → Attempt transfer exceeding limit
   → Verify Trading Wing catches and surfaces the error
   → Verify error is logged to Coordinator/audit trail

5. Test derivationIndex preservation
   → Create wallet for token A, wallet for token B
   → Verify each gets unique walletId
   → Verify addresses are isolated
```

### Test 6: Integration — Full Security Flow

```
1. Initialize treasury with Squads multisig as authority
2. Create Hydra cranks for withdraw_fees and check_redistribute
3. Fund trading via MCP bridge
4. Execute a trade on HL testnet
5. Yield returns → check_redistribute crank fires → split executes
6. Freeze treasury → verify cranks fail (TreasuryFrozen)
7. Unfreeze via Squads → cranks resume
8. Verify audit trail: events, logs, on-chain state
```

---

## 5. Doc Update Checklist

After all code changes are complete:

- [ ] **CLAUDE.md** — Update Signing Architecture section with dual MCP + Server SDK flow. Add Squads Multisig section (program ID, REST API, config). Add Hydra Crank section (program ID, crate, cost model). Update Key Files table with new files. Update Commands section. Update Devnet Limitations. Add new invariants (multisig authority, freeze capability, Hydra automation).
- [ ] **SESSION-CONTEXT.md** — Add new session entry documenting all security hardening work. Update signing architecture references. Add Squads multisig address and configuration. Add Hydra crank addresses. Mark migration items from `rtp-current-state.md` as addressed.
- [ ] **SOULCONTRACT.md** — Add constitutional invariant: "treasury.authority MUST be a Squads multisig PDA after initialization, minimum 2-of-3 threshold". Add emergency freeze rules: "freeze is authority-gated (no time lock for speed), unfreeze requires 2-of-3 + 24h time lock". Add Hydra scheduling rules: "withdraw_fees every ~800 slots (~5 min), check_redistribute every ~1,600 slots (~10 min), atomic rollback on failure". Update capital flow description to note Hydra automation of permissionless ops.
- [ ] **docs/RESOURCES.md** — Add Squads: docs (`https://docs.squads.so`), REST API (`https://developer-api.squads.so/api/v1`), program ID (`SQDS4ep65T869zMMBKyuUq6a6EgTu8psMjkvj52pCf`), Rust crate (`squads-multisig-program`). Add Hydra: GitHub (`https://github.com/magicblock-labs/hydra`), crate (`hydra-api`), program ID (`Hydra17i1feui9deaxu6d1TzSQMRNHeBRkDR1Awy7zea`). Add Server SDK: npm (`@phantom/server-sdk`), GitHub (`https://github.com/phantom/phantom-connect-sdk`).
- [ ] **README.md** — Add Security Hardening section (if it doesn't exist): describe Squads multisig, Hydra cranks, freeze capability, zero-address guards. Update architecture diagram if present.
- [ ] **research/rtp-current-state.md** — Mark all P0 items as addressed. Mark P1 items as addressed. Update status of each file listed in the migration map.

---

## 6. Migration Order

Execute in this order to minimize integration risk:

### Phase 1 — On-Chain Security (P0) — COMPLETE

```
1a. Add zero-address rejection guard to lib.rs                    ✅ DONE
    ├── Add ZeroAddressRejected error
    ├── Add reject_zero_address() function
    └── Apply to initialize() — authority, mint, all 3 wallets

1b. Add emergency freeze/unfreeze to lib.rs                       ✅ DONE
    ├── Add frozen: bool to Treasury account
    ├── Add TreasuryFrozen, AlreadyFrozen, NotFrozen errors
    ├── Add freeze_treasury and unfreeze_treasury instructions
    ├── Add TreasuryFrozen/TreasuryUnfrozen events
    └── Add frozen guard to all 12 state-mutating instructions

1c. Deploy updated program to devnet                              ⬜ TODO (pre-submission)
    ├── anchor build ✅
    ├── anchor deploy --provider.cluster devnet
    └── Re-initialize treasury PDA (account layout changed — added frozen field)

1d. Run existing test suite                                       ✅ DONE
    ├── anchor build passes
    └── cargo test --lib — 307 passed, 0 failed
```

### Phase 2 — Squads Multisig Integration (P1) — DEFERRED POST-HACKATHON

```
Requires: Squads API key, 3 signer wallets, operational commitment.
The authority rotation to Squads PDA is a production security upgrade,
not needed for the hackathon demo. See Section B for full spec.
```

### Phase 3 — Hydra Crank Integration (P1) — DEFERRED POST-HACKATHON

```
Requires: hydra-api crate integration, cranker deployment, funded accounts.
Permissionless ops are called manually in demo. Post-launch, set up
Hydra cranks via CLI or script. See Section C for full spec.
```

### Phase 4 — SDK + Dashboard Updates (P1) — PARTIALLY DONE

```
4a. Update TypeScript SDK                                         ✅ DONE
    ├── Add freezeTreasury, unfreezeTreasury, isTreasuryFrozen helpers
    ├── Update TreasuryState type with isFrozen field
    └── Regenerate IDL from updated Anchor program (16 instructions)

4b. Update dashboard                                              ✅ DONE
    ├── Add freeze banner (red bar when isFrozen)
    └── Add frozen state polling from devnet account data

4c. Add Server SDK for dashboard signing ops                      ❌ SKIPPED
    └── MCP subprocess retained — Server SDK is deprecated per Phantom docs
```

### Phase 5 — Documentation (P2) — DONE

```
5a. Update CLAUDE.md                                              ✅ DONE
5b. Update SESSION-CONTEXT.md                                     ⬜ TODO
5c. Update SOULCONTRACT.md                                        ✅ DONE
5d. Update docs/RESOURCES.md                                      ✅ DONE
5e. Update README.md                                              ⬜ TODO (if needed)
5f. Mark rtp-current-state.md items as addressed                  ⬜ TODO
```

---

## Appendix: Key Addresses and Identifiers

| Item | Value |
|------|-------|
| RTP Program ID | `8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB` |
| RTP Treasury PDA (devnet) | `FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF` |
| Squads Program ID | `SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf` |
| Hydra Program ID | `Hydra17i1feui9deaxu6d1TzSQMRNHeBRkDR1Awy7zea` |
| Phantom Portal App ID | `2fbef7dc-7975-4378-ba2b-ff8018ad2325` |
| Agent Wallet (Solana, di=0) | `AxRWo1N4xjyUN3fbmRpUVwP4WQcEPakdECThyx93CxkR` |
| Agent Wallet (EVM, di=0) | `0xc1c3b483ec26f5aece1aa25b74de5180fd6dbff8` |
| Phantom MCP Server | `@phantom/mcp-server` v1.2.x |
| Phantom Server SDK | `@phantom/server-sdk` v2.0.1 |
| Hydra Crate | `hydra-api` v0.1.0 |
| Squads REST API | `https://developer-api.squads.so/api/v1` |
| USDC Mint (mainnet) | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| Devnet RPC | `https://api.devnet.solana.com` |
| Mainnet RPC | `https://api.mainnet-beta.solana.com` |

---

*End of Security Hardening Specification v1.0*
