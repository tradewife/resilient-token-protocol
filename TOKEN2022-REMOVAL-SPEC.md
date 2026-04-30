# RTP Treasury Program - Token-2022 Removal Spec

**File:** `TOKEN2022-REMOVAL-SPEC.md`  
**Status:** Execution-control spec for the Token-2022 migration only  
**Goal:** Remove Token-2022 `TransferFeeConfig` as RTP's fee-intake model and make treasury intake/accounting native SOL based, without breaking Flash Trade CPI or existing governance gates.

---

## Fresh Agent Handoff

You are taking over a repo that is mid-migration. Do not assume the working tree is coherent just because the Rust program builds.

Current known state as of this handoff:

- `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs` has already been heavily rewritten toward native SOL.
- `rtp/programs/rtp-treasury/programs/rtp-treasury/Cargo.toml` and `Cargo.lock` have already had `spl-token-2022-interface` removed.
- `sdk/index.ts` and `dashboard/src/lib/sdk/index.ts` were partially rewritten by another agent and are currently broken.
- `TOKEN2022-REMOVAL-SPEC.md` itself is untracked in git.
- Other current files still likely reference the old mint/vault/Token-2022 model.

Verification already run:

```bash
cd rtp/programs/rtp-treasury && anchor build
```

Result: passes after `deposit_sol` was corrected to use System Program CPI for payer -> treasury.

```bash
cd sdk && npx tsc --noEmit
```

Result: fails. Known errors include undefined `authorityAddress`, undefined `adopterId`, old `tokenMint` return fields, removed `deriveVaultPDA`, and stale `mint` references.

Immediate objective:

1. Stabilize the on-chain core with minimal tests.
2. Make the SDK compile against the new authority-seeded, SOL-native program API.
3. Only after SDK is green, update CLI/dashboard/scripts/tests.

Do not start with dashboard polish, docs copy, security-campaign planning, or large rewrites outside the current critical path.

Recommended first commands:

```bash
git status --short
cd rtp/programs/rtp-treasury && anchor build
cd ../../.. && cd sdk && npx tsc --noEmit
rg -n "TransferFeeConfig|withdraw_fees|withdrawFees|withdraw_withheld|verify_adoption|VerifyAdoption|treasuryVault|deriveVaultPDA|TOKEN_2022_PROGRAM_ID" .
```

Interpretation:

- `anchor build` passing means only the Rust layer compiles. It does not mean the migration is complete.
- SDK TypeScript passing is the next gate. Until then, CLI/dashboard work is premature.
- Remaining `TOKEN_2022_PROGRAM_ID` references are acceptable only if they are explicitly Flash Trade funding-token compatibility, not RTP fee intake.

---

## Audit Findings That Shape This Spec

The old spec was not safe to execute as written.

1. `system_program::transfer` cannot be used to send SOL out of the treasury state account once that account is owned by the RTP program. Inbound deposits from a wallet can use System Program transfer. Outbound transfers from the program-owned treasury account must debit/credit lamports directly.
2. Removing `Treasury.mint` breaks every account context currently seeded with `[TREASURY_SEED, treasury.mint]`, including strategy, freeze, Flash Trade, and emergency paths. If `mint` is removed, all contexts must be reseeded to `[TREASURY_SEED, treasury.authority]`.
3. Flash Trade is not "unaffected" if `treasury_vault` is removed. `open_flash_position` currently reads `treasury_vault.amount` for runway and position-size checks and signs with mint-derived treasury seeds.
4. `anchor-spl` cannot be blindly removed while Flash Trade support remains. The program currently uses `anchor_spl::token::ID` and `anchor_spl::token_2022::ID` to validate Flash funding token programs. Token-2022 fee-intake code should be removed; Flash token-program compatibility is a separate concern.
5. The affected surface is larger than `lib.rs` and `tests/treasury.ts`: SDK, dashboard SDK copy, CLI commands/types, dashboard launch/docs pages, treasury scripts, `strategy-lifecycle.ts`, `flash-trade-cpi.ts`, and generated IDLs all reference the old mint/vault model.

The implementation must be managed in phases. A partially updated SDK/CLI/dashboard is worse than no SDK update because it gives callers a broken API surface while the on-chain program has already changed.

---

## Execution Phases

### Phase 1 - On-Chain Core

Definition of done:

- Anchor program builds.
- No Token-2022 fee-intake code remains in `src/lib.rs`.
- Native SOL deposit, redistribution, hydration, freeze/unfreeze, strategy lifecycle, and Flash accounting compile against the same treasury seed model.
- A minimal Anchor test file proves the new core API.

Do not claim system completion at the end of Phase 1. At this point, the program may compile while SDK, CLI, dashboard, and old tests are still broken.

### Phase 2 - Client Surface

Definition of done:

- `sdk/index.ts` compiles with `npx tsc --noEmit`.
- `dashboard/src/lib/sdk/index.ts` is generated or manually synced from the same working SDK.
- CLI TypeScript no longer exposes mint-derived treasury commands for current paths.
- Dashboard current pages no longer import or call removed Token-2022 functions.

### Phase 3 - Test and Demo Rewrite

Definition of done:

- `anchor test` passes or obsolete tests are explicitly archived.
- Current scripts demonstrate native SOL flow only.
- Flash Trade tests prove the funding/accounting path still works or are marked blocked with the exact missing funding decision.

Do not add security-campaign planning to this spec. That becomes a separate readiness document after the migration is tested end to end.

---

## Target Architecture

RTP becomes a native-SOL treasury:

- `initialize` creates one treasury PDA per authority: `[b"treasury", authority]`.
- The treasury account itself holds SOL lamports.
- `deposit_sol` moves lamports from a payer into the treasury.
- `check_redistribute` sends excess native SOL to the configured recipient wallets.
- `hydrate_swarm` sends native SOL to a swarm/operator wallet, still gated by Live strategy and beta status.
- `evolve_phase` checks native treasury available balance.
- Strategy, freeze, emergency, and Flash account constraints are reseeded away from `treasury.mint`.
- Token-2022 TransferFeeConfig adoption, mint verification, withheld-fee withdrawal, token vaults, and token recipient accounts are removed.

Important: this removes Token-2022 as the fee-intake mechanism. It does not require removing every use of SPL token concepts from Flash Trade integration, because Flash Trade itself may require token-program accounts for WSOL/funding paths.

---

## Native SOL Accounting Rules

Add helpers near existing shared helpers:

```rust
fn treasury_rent_exempt_minimum() -> Result<u64> {
    Ok(Rent::get()?.minimum_balance(8 + Treasury::INIT_SPACE))
}

fn treasury_lamports(treasury: &Account<Treasury>) -> u64 {
    treasury.to_account_info().lamports()
}

fn available_treasury_lamports(treasury: &Account<Treasury>) -> Result<u64> {
    Ok(treasury_lamports(treasury)
        .saturating_sub(treasury_rent_exempt_minimum()?)
        .saturating_sub(treasury.committed_sol_lamports))
}

fn transfer_from_treasury<'info>(
    treasury: &Account<'info, Treasury>,
    recipient: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, TreasuryError::ZeroAmount);

    let available = available_treasury_lamports(treasury)?;
    require!(available >= amount, TreasuryError::HydrationExceedsBalance);

    **treasury.to_account_info().try_borrow_mut_lamports()? = treasury
        .to_account_info()
        .lamports()
        .checked_sub(amount)
        .ok_or(TreasuryError::Overflow)?;

    **recipient.try_borrow_mut_lamports()? = recipient
        .lamports()
        .checked_add(amount)
        .ok_or(TreasuryError::Overflow)?;

    Ok(())
}
```

Add `committed_sol_lamports: u64` to `Treasury` if Flash-open commitments are tracked globally. This avoids redistributing or hydrating SOL that is already committed to open positions. Keep the per-strategy `committed_sol_lamports` too.

---

## Program Changes

### Cargo.toml

Remove:

```toml
spl-token-2022-interface = "..."
```

Remove any Token-2022 transfer-fee feature usage.

Do not remove `anchor-spl` unless Flash Trade code is also rewritten to avoid `anchor_spl::token::ID`, `anchor_spl::token_2022::ID`, token-account validation, and any future WSOL sync path. A surgical implementation should keep `anchor-spl` for Flash-only compatibility.

Update `idl-build` features accordingly; do not leave `anchor-spl/idl-build` if no `anchor-spl` types remain in the IDL.

### Imports

Remove:

```rust
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use spl_token_2022_interface::{
    extension::{transfer_fee::TransferFeeConfig, BaseStateWithExtensions, StateWithExtensions},
    state::Mint as SplMint,
};
```

Keep or add:

```rust
use anchor_lang::system_program;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};
```

If Flash still validates SPL token-program IDs, import only the exact `anchor_spl` modules needed for Flash.

### Delete Token-2022 Fee-Intake Code

Delete:

- `verify_transfer_fee_config`
- `verify_adoption`
- `withdraw_fees`
- `WithdrawFees`
- `VerifyAdoption`
- all imports and comments specific to `TransferFeeConfig`
- errors `WithdrawAuthorityMismatch` and `MintNotConfigured`

### Treasury State

Remove:

```rust
pub mint: Pubkey,
```

Add:

```rust
pub committed_sol_lamports: u64,
```

Keep:

- `authority`
- phase
- distribution totals
- hydration totals
- `total_fees_withdrawn` for backwards metric continuity, but update comments to native SOL deposits
- `total_fees_received_lamports`
- recipient wallets
- `min_runway_balance`
- `frozen`
- `bump`

All code and comments must say lamports/SOL, not token/vault/TransferFeeConfig.

### Initialize

Replace the current mint/vault initializer with:

```rust
pub fn initialize(
    ctx: Context<Initialize>,
    holders_wallet: Pubkey,
    project_dev_wallet: Pubkey,
    ecosystem_wallet: Pubkey,
    min_runway_balance: u64,
) -> Result<()>
```

Accounts:

```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Treasury::INIT_SPACE,
        seeds = [TREASURY_SEED, authority.key().as_ref()],
        bump,
    )]
    pub treasury: Account<'info, Treasury>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}
```

Handler requirements:

- reject zero authority and recipient wallets
- require `min_runway_balance >= DEFAULT_MIN_RUNWAY`
- initialize all counters to zero, including `committed_sol_lamports`
- store `ctx.bumps.treasury`

### Deposit SOL

Add:

```rust
pub fn deposit_sol(ctx: Context<DepositSol>, amount_lamports: u64) -> Result<()>
```

Accounts:

```rust
#[derive(Accounts)]
pub struct DepositSol<'info> {
    #[account(
        mut,
        seeds = [TREASURY_SEED, treasury.authority.as_ref()],
        bump = treasury.bump,
    )]
    pub treasury: Account<'info, Treasury>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
```

Use `system_program::transfer` for payer -> treasury. Increment:

- `treasury.total_fees_withdrawn`
- `treasury.total_fees_received_lamports`

Do not combine optional adopter attribution into this context. Anchor optional accounts with self-referential seeds are easy to get wrong and make the hot path less legible. Keep attribution explicit through `record_fee_deposit`, or add a separate `deposit_sol_for_adopter` instruction only if a single atomic deposit-plus-attribution path is required later.

Inbound deposit must use System Program CPI:

```rust
anchor_lang::system_program::transfer(
    CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: ctx.accounts.payer.to_account_info(),
            to: ctx.accounts.treasury.to_account_info(),
        },
    ),
    amount_lamports,
)?;
```

Do not directly subtract lamports from the payer account. The RTP program does not own the payer account.

### Adopter Records

Replace `token_mint` with:

```rust
pub adopter_id: Pubkey,
```

Use seeds:

```rust
[b"adopter", treasury.key().as_ref(), adopter_id.as_ref()]
```

Update:

- `register_adopter`
- `register_adopter_beta`
- `record_fee_deposit`
- `end_beta`
- all events
- all tests and SDK PDA derivations

`register_adopter` must initialize `fees_contributed_lamports`, `last_deposit_ts`, and `deposit_count`; the old spec forgot these fields.

`record_fee_deposit` remains authority-gated. It is accounting only. It must not move funds.

### Check Redistribute

Remove mint, token vault, token recipient, and token program accounts.

Accounts:

```rust
#[derive(Accounts)]
pub struct CheckRedistribute<'info> {
    #[account(
        mut,
        seeds = [TREASURY_SEED, treasury.authority.as_ref()],
        bump = treasury.bump,
    )]
    pub treasury: Account<'info, Treasury>,

    /// CHECK: must equal treasury.holders_wallet
    #[account(mut, address = treasury.holders_wallet)]
    pub holders_wallet: UncheckedAccount<'info>,

    /// CHECK: must equal treasury.project_dev_wallet
    #[account(mut, address = treasury.project_dev_wallet)]
    pub project_dev_wallet: UncheckedAccount<'info>,

    /// CHECK: must equal treasury.ecosystem_wallet
    #[account(mut, address = treasury.ecosystem_wallet)]
    pub ecosystem_wallet: UncheckedAccount<'info>,
}
```

Handler:

- `balance = available_treasury_lamports(&treasury)?`
- `excess = balance.saturating_sub(treasury.min_runway_balance)`
- require `excess > DEFAULT_MIN_REDISTRIBUTE`
- calculate 70/20/10 exactly as today
- use `transfer_from_treasury`, not `system_program::transfer`
- increment distribution totals with checked or saturating math consistently
- emit `Redistribution { treasury: treasury.key(), ... }`

### Hydrate Swarm

Remove mint, treasury token vault, swarm token vault, token program accounts.

Recipient is a plain wallet:

```rust
/// CHECK: swarm/operator wallet receiving native SOL
#[account(mut)]
pub swarm_wallet: UncheckedAccount<'info>,
```

Keep:

- frozen gate
- Live strategy gate
- beta expiry gate
- amount > 0
- runway floor check

Use:

```rust
let post_balance = available_treasury_lamports(treasury)?.saturating_sub(amount);
require!(post_balance >= treasury.min_runway_balance, TreasuryError::InsufficientRunway);
transfer_from_treasury(treasury, &ctx.accounts.swarm_wallet.to_account_info(), amount)?;
```

Increment `treasury.total_hydration`.

Delete `create_swarm_vault` and `CreateSwarmVault`; there is no token vault to initialize.

### Evolve Phase

Remove mint/vault/token-program accounts. Check native available lamports:

```rust
let balance = available_treasury_lamports(&ctx.accounts.treasury)?;
```

Keep authority constraint and irreversible phase transitions.

### Freeze/Unfreeze, Strategy, Emergency Contexts

Every context using:

```rust
seeds = [TREASURY_SEED, treasury.mint.as_ref()]
```

must become:

```rust
seeds = [TREASURY_SEED, treasury.authority.as_ref()]
```

This includes:

- `RegisterStrategy`
- `UpdateStrategyPerformance`
- `ForceRetireStrategy`
- `EndBeta`
- `FreezeTreasury`
- `UnfreezeTreasury`
- `OpenFlashPosition`
- `CloseFlashPosition`
- `EmergencyCloseAllPositions`
- any remaining treasury account context

Update `TreasuryFrozen` and `TreasuryUnfrozen` events from `mint` to `treasury`.

### Flash Trade CPI

This is the most important correction to the old spec: Flash Trade is affected by removing the token vault.

Minimum required changes:

- remove `treasury_vault: InterfaceAccount<TokenAccount>` from `OpenFlashPosition`
- compute runway/position-size checks from `available_treasury_lamports(&treasury)?`
- use signer seeds `[TREASURY_SEED, treasury.authority.as_ref(), &[treasury.bump]]`
- update both strategy and treasury `committed_sol_lamports` on open/close/emergency reset
- keep Flash program ID, discriminators, remaining-account order, event-authority validation, system-program validation, and token-program validation unless intentionally revalidated against a newer Flash IDL

Open handler:

```rust
let available = available_treasury_lamports(treasury)?;
let max_input = available as u128 * MAX_POSITION_SIZE_BPS as u128 / 10_000;
require!(input_sol_lamports as u128 <= max_input, TreasuryError::PositionSizeExceeded);
require!(
    available.saturating_sub(input_sol_lamports) >= treasury.min_runway_balance,
    TreasuryError::InsufficientRunway,
);
```

Then increment:

```rust
strategy.committed_sol_lamports = strategy.committed_sol_lamports.checked_add(input_sol_lamports).ok_or(TreasuryError::Overflow)?;
treasury.committed_sol_lamports = treasury.committed_sol_lamports.checked_add(input_sol_lamports).ok_or(TreasuryError::Overflow)?;
```

Close handler decrements both, with checked underflow rejection.

If Flash requires a WSOL funding token account to be pre-funded, document and test the exact funding flow in the implementation. Do not pretend native SOL deposits alone prove Flash funding works. Either:

- keep the Flash funding account funded by the agent before CPI and treat treasury native lamports as the risk/accounting source, or
- add a dedicated on-chain wrap-to-WSOL step before Flash CPI.

Do not remove `anchor-spl` until this question is resolved and covered by tests.

---

## SDK, CLI, Dashboard, Scripts

Update all public APIs away from mint-derived treasury identity.

SDK files:

- `sdk/index.ts`
- `dashboard/src/lib/sdk/index.ts`
- generated/bundled IDLs in both locations

Required SDK changes:

- `deriveTreasuryPDA(authority: PublicKey)`
- remove `deriveVaultPDA`
- `deriveAdopterPDA(treasury: PublicKey, adopterId: PublicKey)`
- replace `registerWithRTP(mint, ...)` with `initializeTreasury(...)` or `registerWithRTP({ authority/adopterId/... })`
- add `depositSol(connection, payer, treasuryOrAuthority, amountLamports)`
- remove `withdrawAndRedistribute`; replace with explicit `depositSol` and `checkRedistribute`
- `fetchTreasuryState` takes treasury or authority, not mint
- return native lamport/SOL balances from `connection.getBalance(treasuryPDA)` and decoded state
- remove SPL Token imports unless still needed for Flash account tooling

CLI/dashboard surfaces to update:

- `cli/src/extern.d.ts`
- `cli/src/commands/accounts.ts`
- `cli/src/commands/crank.ts`
- `cli/src/commands/deploy.ts`
- `cli/src/commands/freeze.ts`
- `cli/src/commands/register.ts`
- `cli/src/commands/status.ts`
- `cli/src/commands/strategy.ts`
- `dashboard/src/app/launch/page.tsx`
- `dashboard/src/app/page.tsx`
- `dashboard/src/app/docs/page.tsx`
- dashboard copy that says TransferFeeConfig, per-mint vault, or withdraw fees

Treasury scripts/tests to update or archive:

- `rtp/programs/rtp-treasury/tests/treasury.ts`
- `rtp/programs/rtp-treasury/tests/strategy-lifecycle.ts`
- `rtp/programs/rtp-treasury/tests/flash-trade-cpi.ts`
- `rtp/programs/rtp-treasury/scripts/devnet-demo.ts`
- `rtp/programs/rtp-treasury/scripts/devnet-beta-test.ts`
- `scripts/fee-crank.ts`
- any smoke test that still creates Token-2022 mints

Archive obsolete Token-2022 demos instead of quietly leaving them as current operational examples.

---

## Test Plan

On-chain tests must cover:

- initialize rejects zero recipient wallets
- initialize rejects runway below `DEFAULT_MIN_RUNWAY`
- treasury PDA is derived from authority
- deposit_sol rejects zero amount
- deposit_sol rejects frozen treasury
- deposit_sol increases treasury lamports and fee counters
- record_fee_deposit remains authority-gated and accounting-only
- check_redistribute rejects below threshold
- check_redistribute preserves rent exemption and runway
- check_redistribute pays 70/20/10 native SOL recipients
- hydrate_swarm rejects non-Live strategy
- hydrate_swarm rejects expired/ended beta
- hydrate_swarm preserves rent exemption and runway
- evolve_phase checks native available balance
- freeze/unfreeze contexts work after reseeding
- strategy lifecycle tests work after treasury reseeding
- Flash open position-size/runway gates use native available balance
- Flash close/emergency reset decrements treasury and strategy commitments

Remove tests that only prove Token-2022 mint configuration or withheld-fee withdrawal:

- mint without `TransferFeeConfig` rejection
- wrong withdraw authority rejection
- `withdraw_fees` CPI success/failure
- token recipient ATA redistribution assertions
- `create_swarm_vault` tests

---

## Verification Checklist

Run from `rtp/programs/rtp-treasury` unless noted:

```bash
anchor build
anchor test
```

Repository-wide stale-reference checks:

```bash
rg -n "TransferFeeConfig|withdraw_fees|withdrawFees|withdraw_withheld|verify_adoption|VerifyAdoption|treasuryVault|deriveVaultPDA|TOKEN_2022_PROGRAM_ID" .
```

Expected after implementation:

- no `TransferFeeConfig`, `withdraw_fees`, `withdrawFees`, `withdraw_withheld`, `verify_adoption`, `VerifyAdoption`, `treasuryVault`, or `deriveVaultPDA` references in current code
- `TOKEN_2022_PROGRAM_ID` may remain only if Flash Trade funding explicitly still supports Token-2022 token programs; each remaining occurrence must be Flash-related, not RTP fee intake

Then:

```bash
cd ../../..
cd sdk && npx tsc --noEmit
cd ../dashboard && npx tsc --noEmit
```

Run or create a smoke test that does:

1. initialize treasury
2. deposit_sol
3. register_adopter
4. record_fee_deposit
5. register_strategy
6. check_redistribute
7. hydrate_swarm
8. freeze/unfreeze

Flash Trade smoke testing must separately prove that removing the token treasury vault did not break the funding account path.

---

## Explicit Non-Goals

Do not change:

- Flash Trade program ID
- Flash Trade instruction discriminators
- Flash Trade remaining account order, unless verified against a newer Flash IDL
- hard stop constants
- strategy performance semantics
- phase transition semantics, other than balance source
- Python research/Night Shift
- Railway deployment behavior

Do update docs/copy after code changes so the public story no longer claims Token-2022 fee intake.
