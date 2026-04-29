use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use spl_token_2022_interface::{
    extension::{transfer_fee::TransferFeeConfig, BaseStateWithExtensions, StateWithExtensions},
    state::Mint as SplMint,
};

declare_id!("8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB");

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Redistribution split basis points (10000 = 100%).
/// Ecosystem receives rounding remainder via saturating_sub, not direct BPS calc.
const HOLDERS_BPS: u16 = 7000;     // 70%
const PROJECT_DEV_BPS: u16 = 2000; // 20%
#[allow(dead_code)]
const ECOSYSTEM_BPS: u16 = 1000;   // 10% (auditability reference)

/// Phase thresholds in USDC (6 decimals).
/// Production: validated against on-chain oracle (Pyth/Switchboard).
/// Devnet: phase_authority signature is the guard.
/// Wired into evolve_phase via oracle TODO — see handler for details.
const SUSTENANCE_CAP: u64 = 50_000_000_000;   // $50k
const ECOSYSTEM_CAP: u64 = 1_000_000_000_000; // $1M

/// Default minimum redistribution amount (1 token, 6 decimals).
/// Prevents dust distributions that waste gas.
const DEFAULT_MIN_REDISTRIBUTE: u64 = 1_000_000;

/// Default minimum runway balance (10 tokens).
/// Production: set to USDC value covering 90 days of ops (~$18k USDC).
/// See BUILD_PLAN.md: "~$100-200/mo ops cost" → $18,000 for 90 days.
const DEFAULT_MIN_RUNWAY: u64 = 10_000_000;

/// PDA seeds
const TREASURY_SEED: &[u8] = b"treasury";
const SWARM_HYDRATION_SEED: &[u8] = b"swarm-hydration";
const STRATEGY_SEED: &[u8] = b"strategy";

/// Hard stop thresholds — mirrors Python RetirementGate in research/promotion_criteria.py
const HARD_DRAWDOWN_24H_BPS: u16 = 1000;       // 10% = 1000 bps — mirrors RetirementGate.HARD_DRAWDOWN_24H_PCT
const HARD_CONSECUTIVE_LOSSES: u8 = 5;          // mirrors RetirementGate.HARD_CONSECUTIVE_LOSSES
const HARD_ROLLING_SHARPE_MIN_X100: i32 = 50;   // 0.5 * 100 — mirrors RetirementGate.HARD_ROLLING_SHARPE_MIN

/// Soft decay retirement — mirrors Python RetirementGate.SOFT_STRIKE_THRESHOLD
const SOFT_STRIKE_THRESHOLD: u8 = 3;

/// Minimum consecutive positive-performance updates before soft decay strikes reset.
/// Prevents a single lucky trade from clearing the strike count.
const MIN_RECOVERY_TRADES: u8 = 3;

/// Flash Trade Perpetuals program ID (mainnet)
const FLASH_TRADE_PROGRAM_ID: &str = "FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn";

/// Flash Trade open_position discriminator (IDL v15.2.0)
const FLASH_OPEN_POSITION_DISC: [u8; 8] = [135, 128, 47, 77, 15, 152, 240, 49];

/// Flash Trade close_position discriminator (IDL v15.2.0)
/// Verified via mainnet close TX dFqkoP2... — must match flash-trade-demo.ts CLOSE_POS_DISC
const FLASH_CLOSE_POSITION_DISC: [u8; 8] = [191, 210, 137, 115, 145, 22, 230, 244];

/// Max concurrent Flash Trade positions per strategy
const MAX_CONCURRENT_POSITIONS: u8 = 3;

/// Max position size as fraction of vault (BPS, 2000 = 20%)
const MAX_POSITION_SIZE_BPS: u16 = 2000;

/// Flash Trade compute budget for open/close (agent layer reference)
#[allow(dead_code)]
const FLASH_CU_LIMIT: u32 = 600_000;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[error_code]
pub enum TreasuryError {
    #[msg("Treasury reserves below redistribution threshold")]
    BelowThreshold,
    #[msg("Post-hydration balance would fall below the 90-day runway minimum")]
    InsufficientRunway,
    #[msg("Hydration amount exceeds available balance")]
    HydrationExceedsBalance,
    #[msg("Treasury is already at maximum phase (Humanity)")]
    AlreadyMaxPhase,
    #[msg("Only the treasury authority can evolve phases")]
    UnauthorizedPhaseEvolution,
    #[msg("Mint's withdraw_withheld_authority does not match Treasury PDA")]
    WithdrawAuthorityMismatch,
    #[msg("Mint does not have TransferFeeConfig enabled — cannot adopt RTP")]
    MintNotConfigured,
    #[msg("Fee deposit amount must be greater than zero")]
    ZeroAmount,
    #[msg("Arithmetic overflow in fee accounting")]
    Overflow,
    #[msg("Strategy is not in Live status — cannot fund or trade")]
    StrategyNotLive,
    #[msg("Strategy has breached a hard stop threshold")]
    HardStopBreached,
    #[msg("Strategy has accumulated too many soft decay strikes")]
    SoftDecayRetirement,
    #[msg("Strategy ID must be 1–16 characters")]
    InvalidStrategyId,
    #[msg("Only the treasury authority can register or retire strategies")]
    UnauthorizedStrategyOp,
    #[msg("Beta period has expired — operations no longer permitted")]
    BetaExpired,
    #[msg("Only the treasury authority can end a beta")]
    UnauthorizedBetaOp,
    #[msg("Zero address (Pubkey::default()) is not allowed")]
    ZeroAddressRejected,
    #[msg("Treasury is frozen — all operations are halted")]
    TreasuryFrozen,
    #[msg("Treasury is already frozen")]
    AlreadyFrozen,
    #[msg("Treasury is not frozen")]
    NotFrozen,
    #[msg("Too many concurrent Flash Trade positions (max 3)")]
    TooManyOpenPositions,
    #[msg("Input SOL exceeds maximum position size (20% of vault)")]
    PositionSizeExceeded,
    #[msg("Position PDA does not match Treasury PDA as owner")]
    PositionNotOwnedByTreasury,
    #[msg("Flash Trade CPI call failed")]
    FlashCpiFailed,
    #[msg("Invalid Flash Trade program ID")]
    InvalidFlashProgramId,
    #[msg("Pool name must be 1-32 characters")]
    InvalidPoolName,
    #[msg("Decremented committed_sol_lamports exceeds tracked balance")]
    CommittedDeltaExceedsBalance,
    #[msg("FlashSide::None is not valid for open/close — position must have a direction")]
    InvalidFlashSide,
    #[msg("remaining_accounts[15] does not match the Flash Trade event authority PDA")]
    InvalidFlashEventAuthority,
    #[msg("remaining_accounts[13] is not the expected System Program")]
    InvalidFlashSystemProgram,
    #[msg("remaining_accounts[14] is not an expected token program (SPL token or token-2022)")]
    InvalidFlashTokenProgram,
    #[msg("Adopter record does not belong to this treasury")]
    AdopterTreasuryMismatch,
    #[msg("Only the treasury authority can record fee deposits")]
    UnauthorizedFeeAttribution,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[account]
#[derive(InitSpace)]
pub struct Treasury {
    /// The Token-2022 mint this treasury serves
    pub mint: Pubkey,
    /// The phase authority (set at initialization).
    pub authority: Pubkey,
    /// Current evolution phase (Sustenance → Ecosystem → Humanity)
    pub phase: Phase,
    /// Cumulative fees withdrawn from mint via TransferFeeConfig
    pub total_fees_withdrawn: u64,
    /// Cumulative tokens distributed to holders (70%)
    pub total_distributed_holders: u64,
    /// Cumulative tokens distributed to project dev (20%)
    pub total_distributed_dev: u64,
    /// Cumulative tokens distributed to ecosystem (10%)
    pub total_distributed_ecosystem: u64,
    /// Cumulative tokens sent to swarm hydration vault
    pub total_hydration: u64,
    /// Cumulative fee contributions recorded from all adopters via record_fee_deposit.
    /// Denominator for pro-rata yield attribution:
    ///   adopter_yield_share = fees_contributed / total_fees_received_lamports * yield_pool
    pub total_fees_received_lamports: u64,
    /// Holders wallet (receives 70% of redistribution)
    pub holders_wallet: Pubkey,
    /// Project dev wallet (receives 20% of redistribution)
    pub project_dev_wallet: Pubkey,
    /// Ecosystem wallet (receives 10% of redistribution)
    pub ecosystem_wallet: Pubkey,
    /// Minimum balance that must remain after hydration.
    /// Enforces the 90-day runway invariant (CLAUDE.md #9).
    /// Production: set to USDC-denominated 90-day ops cost via oracle.
    pub min_runway_balance: u64,
    /// Whether the treasury is frozen (emergency halt).
    /// When true, all non-read operations are rejected.
    pub frozen: bool,
    /// PDA bump
    pub bump: u8,
}

/// Treasury phase -- can only advance forward. Transitions are IRREVERSIBLE.
/// - Sustenance (<$50k): self-hydrate, reinvest all yield
/// - Ecosystem ($50k-$1M): auto-provide LP to top RTP-adopting tokens
/// - Humanity (>$1M): USDC grants to Solana public-goods projects
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum Phase {
    Sustenance,
    Ecosystem,
    Humanity,
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Sustenance
    }
}

// ---------------------------------------------------------------------------
// AdopterRecord — per-token fee tracking for multi-token attribution
// ---------------------------------------------------------------------------

/// Tracks a single token project's cumulative fee contributions to the RTP treasury.
/// One AdopterRecord PDA per adopting token mint.
/// Seeds: ["adopter", token_mint.key()]
/// This enables pro-rata yield attribution:
///   adopter_yield_share = fees_contributed_lamports / treasury.total_fees_received_lamports
#[account]
#[derive(InitSpace)]
pub struct AdopterRecord {
    /// The SPL token mint of the adopting project
    pub token_mint: Pubkey,
    /// The treasury this adopter belongs to (back-reference for cross-validation)
    pub treasury: Pubkey,
    /// Cumulative fee contributions (in lamports) since adoption
    pub fees_contributed_lamports: u64,
    /// Unix timestamp of first fee deposit (adoption date)
    pub adopted_at: i64,
    /// Unix timestamp of most recent fee deposit
    pub last_deposit_ts: i64,
    /// Number of discrete fee deposits recorded
    pub deposit_count: u64,
    /// Beta expiry: Unix timestamp after which the swarm stops managing this adopter.
    /// 0 = permanent adopter (no expiry). Non-zero = beta adopter with sunset date.
    pub beta_expires_at: i64,
    /// Whether this beta has been manually ended by the authority
    pub beta_ended: bool,
    /// PDA bump
    pub bump: u8,
}

// ---------------------------------------------------------------------------
// StrategyRecord — on-chain strategy lifecycle ledger
// ---------------------------------------------------------------------------

/// On-chain lifecycle ledger for a single trading strategy.
/// Seeds: [STRATEGY_SEED, treasury.key(), strategy_id.as_bytes()]
#[account]
#[derive(InitSpace)]
pub struct StrategyRecord {
    /// The treasury this strategy belongs to
    pub treasury: Pubkey,
    /// Unique strategy identifier (max 16 bytes, e.g. "S03", "SOL_CARRY_v1")
    #[max_len(16)]
    pub strategy_id: String,
    /// Current lifecycle status
    pub status: StrategyLifecycleStatus,
    /// Unix timestamp when strategy was promoted to LIVE
    pub promoted_at: i64,
    /// Unix timestamp of last performance update
    pub last_update_ts: i64,
    /// Rolling 30-day PnL in basis points (signed, scaled x100)
    /// e.g. +350 = +3.50%, -120 = -1.20%
    pub rolling_pnl_bps: i32,
    /// Number of consecutive losing trades (reset on any win)
    pub consecutive_losses: u8,
    /// Number of soft decay strikes accumulated
    pub soft_decay_strikes: u8,
    /// Consecutive positive-performance updates since last strike (recovery gate).
    /// Strikes only reset after MIN_RECOVERY_TRADES consecutive positive updates.
    pub recovery_counter: u8,
    /// Largest single drawdown observed in the last 24h, in basis points
    pub drawdown_24h_bps: u16,
    /// Cumulative total trades executed on-chain
    pub total_trades: u32,
    /// Sharpe ratio at time of promotion (stored as integer x100, e.g. 396 = 3.96)
    pub promotion_sharpe_x100: i32,
    /// Current rolling Sharpe (integer x100). Updated by the swarm agent.
    pub rolling_sharpe_x100: i32,
    /// Number of currently open Flash Trade positions (max 3)
    pub open_position_count: u8,
    /// Cumulative SOL (lamports) committed across all open positions
    pub committed_sol_lamports: u64,
    /// Flash Trade pool identifier for this strategy (e.g., "Crypto.1")
    #[max_len(32)]
    pub flash_pool_name: String,
    /// PDA bump
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum StrategyLifecycleStatus {
    /// Promoted, actively trading
    Live,
    /// Hard stop triggered — no new trades, existing positions closing
    Suspended,
    /// Retired — strategy is dead, no further operations permitted
    Retired,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum RetirementReason {
    HardDrawdown,
    ConsecutiveLosses,
    RollingSharpeLow,
    SoftDecayStrikes,
    AuthorityForced,
}

// ---------------------------------------------------------------------------
// Flash Trade CPI types — match deployed IDL v15.2.0
// ---------------------------------------------------------------------------

/// Position side — matches Flash Trade on-chain repr (None=0, Long=1, Short=2)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum FlashSide {
    None,
    Long,
    Short,
}

/// Oracle price — matches Flash Trade on-chain struct (i64 price, i32 exponent)
/// Pyth uses exponent -8 (not -6 as originally assumed)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, Debug)]
pub struct FlashOraclePrice {
    pub price: i64,
    pub exponent: i32,
}

/// Privilege level — matches Flash Trade on-chain enum (None=0, Stake=1, Referral=2)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum FlashPrivilege {
    None,
    Stake,
    Referral,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[event]
pub struct AdopterRegistered {
    pub token_mint: Pubkey,
    pub adopted_at: i64,
}

#[event]
pub struct FeeDepositRecorded {
    pub token_mint: Pubkey,
    pub amount_lamports: u64,
    pub cumulative: u64,
    pub total_treasury_fees: u64,
    pub ts: i64,
}

#[event]
pub struct StrategyPromoted {
    pub treasury: Pubkey,
    pub strategy_id: String,
    pub promotion_sharpe_x100: i32,
    pub promoted_at: i64,
}

#[event]
pub struct StrategyPerformanceUpdated {
    pub treasury: Pubkey,
    pub strategy_id: String,
    pub rolling_pnl_bps: i32,
    pub rolling_sharpe_x100: i32,
    pub consecutive_losses: u8,
    pub soft_decay_strikes: u8,
    pub recovery_counter: u8,
    pub drawdown_24h_bps: u16,
    pub status: StrategyLifecycleStatus,
    pub ts: i64,
}

#[event]
pub struct StrategyRetired {
    pub treasury: Pubkey,
    pub strategy_id: String,
    pub reason: RetirementReason,
    pub final_rolling_sharpe_x100: i32,
    pub ts: i64,
}

#[event]
pub struct BetaEnded {
    pub token_mint: Pubkey,
    pub ended_at: i64,
    pub fees_contributed_lamports: u64,
}

#[event]
pub struct Redistribution {
    pub mint: Pubkey,
    pub excess: u64,
    pub holders_amount: u64,
    pub dev_amount: u64,
    pub ecosystem_amount: u64,
    pub ts: i64,
}

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

#[event]
pub struct FlashPositionOpened {
    pub treasury: Pubkey,
    pub strategy_id: String,
    pub side: FlashSide,
    pub input_sol_lamports: u64,
    pub leverage_bps: u32,
    pub pool_name: String,
    pub position_pda: Pubkey,
    pub ts: i64,
}

#[event]
pub struct FlashPositionClosed {
    pub treasury: Pubkey,
    pub strategy_id: String,
    pub position_pda: Pubkey,
    pub realised_pnl_sol_lamports: i64,
    pub returned_sol_lamports: u64,
    pub ts: i64,
}

/// Emitted by `emergency_close_all_positions`. Distinct from `FlashPositionClosed`
/// because the on-chain instruction does NOT itself fire Flash Trade CPI close
/// calls — it resets the position counters and records the operator's intent.
/// Operators MUST follow up with explicit `close_flash_position` calls (or rely
/// on Flash Trade liquidation) to actually close the on-chain positions.
#[event]
pub struct EmergencyPositionsReset {
    pub treasury: Pubkey,
    pub strategy_id: String,
    pub authority: Pubkey,
    pub position_pubkeys: Vec<Pubkey>,
    pub previous_committed_sol_lamports: u64,
    pub ts: i64,
}

// ---------------------------------------------------------------------------
// Shared Helpers (outside #[program] so Anchor doesn't treat as instructions)
// ---------------------------------------------------------------------------

/// Verify that the given mint account has TransferFeeConfig enabled and
/// that `treasury_key` is the `withdraw_withheld_authority`.
///
/// Shared by `initialize` (M-1 fix) and `verify_adoption` — no code
/// duplication.
fn verify_transfer_fee_config(
    mint_info: &AccountInfo,
    treasury_key: &Pubkey,
) -> Result<()> {
    let data = mint_info.try_borrow_data()?;

    // Unpack the full mint account: 82 bytes base Mint + TLV extensions.
    let mint_with_extensions =
        StateWithExtensions::<SplMint>::unpack(data.as_ref())
            .map_err(|_| TreasuryError::MintNotConfigured)?;

    // Extract the TransferFeeConfig extension. If the extension is
    // missing (mint doesn't have TransferFeeConfig), this fails.
    let fee_config = mint_with_extensions
        .get_extension::<TransferFeeConfig>()
        .map_err(|_| TreasuryError::MintNotConfigured)?;

    // Verify the withdraw_withheld_authority matches the Treasury PDA.
    let auth: Option<Pubkey> = fee_config.withdraw_withheld_authority.into();
    match auth {
        Some(key) => require!(
            key == *treasury_key,
            TreasuryError::WithdrawAuthorityMismatch
        ),
        None => return Err(TreasuryError::WithdrawAuthorityMismatch.into()),
    }

    Ok(())
}

/// Reject the Solana zero address on critical fields.
fn reject_zero_address(addr: Pubkey) -> Result<()> {
    if addr == Pubkey::default() {
        return err!(TreasuryError::ZeroAddressRejected);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Program + Account Contexts
// ---------------------------------------------------------------------------

#[program]
pub mod rtp_treasury {
    use super::*;
    use anchor_spl::token_2022_extensions::transfer_fee::withdraw_withheld_tokens_from_mint;

    /// Initialize a new Treasury for a given Token-2022 mint.
    ///
    /// Prerequisites:
    /// - The mint MUST have TransferFeeConfig enabled with the Treasury PDA
    ///   set as `withdraw_withheld_authority`. This is immutable once set.
    /// - The mint MUST be a Token-2022 mint.
    ///
    /// Called once per adopting token. Sets the PDA authority and vault.
    pub fn initialize(
        ctx: Context<Initialize>,
        min_runway_balance: u64,
    ) -> Result<()> {
        // Zero-address guard: reject Pubkey::default() on all critical fields.
        reject_zero_address(ctx.accounts.authority.key())?;
        reject_zero_address(ctx.accounts.mint.key())?;
        reject_zero_address(ctx.accounts.holders_wallet.key())?;
        reject_zero_address(ctx.accounts.project_dev_wallet.key())?;
        reject_zero_address(ctx.accounts.ecosystem_wallet.key())?;

        let treasury = &mut ctx.accounts.treasury;
        treasury.mint = ctx.accounts.mint.key();
        treasury.authority = ctx.accounts.authority.key();
        treasury.phase = Phase::default();
        treasury.total_fees_withdrawn = 0;
        treasury.total_distributed_holders = 0;
        treasury.total_distributed_dev = 0;
        treasury.total_distributed_ecosystem = 0;
        treasury.total_hydration = 0;
        treasury.total_fees_received_lamports = 0;
        treasury.frozen = false;
        treasury.holders_wallet = ctx.accounts.holders_wallet.key();
        treasury.project_dev_wallet = ctx.accounts.project_dev_wallet.key();
        treasury.ecosystem_wallet = ctx.accounts.ecosystem_wallet.key();
        // Enforce explicit non-zero runway floor.
        // H-3 fix: reject 0 explicitly — caller MUST provide a runway value.
        require!(
            min_runway_balance >= DEFAULT_MIN_RUNWAY,
            TreasuryError::InsufficientRunway,
        );
        treasury.min_runway_balance = min_runway_balance;
        treasury.bump = ctx.bumps.treasury;

        // M-1 fix: verify mint has TransferFeeConfig with Treasury PDA as
        // withdraw_withheld_authority BEFORE storing state. Fails early
        // with a clear error if the mint is not properly configured.
        verify_transfer_fee_config(
            &ctx.accounts.mint.to_account_info(),
            &treasury.key(),
        )?;

        Ok(())
    }

    /// Withdraw accumulated TransferFeeConfig fees from mint into treasury vault.
    ///
    /// Uses CPI: `spl_token_2022::withdraw_withheld_tokens_from_mint`
    /// The Treasury PDA (set as `withdraw_withheld_authority` on the mint at
    /// adoption time) signs for the withdrawal. Anyone can call this — fees
    /// are permissionlessly pulled into the PDA.
    pub fn withdraw_fees(ctx: Context<WithdrawFees>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let treasury = &mut ctx.accounts.treasury;
        let mint = &ctx.accounts.mint;
        let mint_key = mint.key();

        // Snapshot balance BEFORE withdrawal to compute delta (F-002 fix)
        let balance_before = ctx.accounts.treasury_vault.amount;

        let seeds = &[
            TREASURY_SEED,
            mint_key.as_ref(),
            &[treasury.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            anchor_spl::token_2022_extensions::transfer_fee::WithdrawWithheldTokensFromMint {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: mint.to_account_info(),
                destination: ctx.accounts.treasury_vault.to_account_info(),
                authority: treasury.to_account_info(),
            },
            signer_seeds,
        );
        withdraw_withheld_tokens_from_mint(cpi_ctx)?;

        // H-2 fix: reload vault to get post-CPI balance.
        ctx.accounts.treasury_vault.reload()?;

        // Track actual delta withdrawn (not just vault balance)
        let withdrawn = ctx.accounts.treasury_vault.amount.saturating_sub(balance_before);
        if withdrawn > 0 {
            treasury.total_fees_withdrawn = treasury.total_fees_withdrawn.saturating_add(withdrawn);
        }
        Ok(())
    }

    /// Check redistribution threshold and execute 70/20/10 split.
    ///
    /// Distributes the vault's excess above `min_runway_balance`:
    /// - 70% → holders
    /// - 20% → project dev wallet
    /// - 10% → ecosystem wallet (+ rounding dust)
    ///
    /// Callable by anyone. The split is deterministic on-chain.
    pub fn check_redistribute(ctx: Context<CheckRedistribute>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let treasury = &mut ctx.accounts.treasury;
        let vault = &ctx.accounts.treasury_vault;
        let balance = vault.amount;
        let decimals = ctx.accounts.mint.decimals;

        // Only distribute the EXCESS above the runway floor.
        // The floor always remains in the vault for ops funding.
        let excess = balance.saturating_sub(treasury.min_runway_balance);
        require!(excess > DEFAULT_MIN_REDISTRIBUTE, TreasuryError::BelowThreshold);

        // Calculate 70/20/10 split on excess (ecosystem gets rounding remainder)
        let holders_amt = (excess as u128 * HOLDERS_BPS as u128 / 10000) as u64;
        let dev_amt = (excess as u128 * PROJECT_DEV_BPS as u128 / 10000) as u64;
        let eco_amt = excess.saturating_sub(holders_amt).saturating_sub(dev_amt);

        let mint_key = ctx.accounts.mint.key();
        let seeds = &[TREASURY_SEED, mint_key.as_ref(), &[treasury.bump]];
        let signer = &[&seeds[..]];
        let token_program = &ctx.accounts.token_program;
        let mint_info = ctx.accounts.mint.to_account_info();

        // 70% → holders
        if holders_amt > 0 {
            token_interface::transfer_checked(
                CpiContext::new_with_signer(
                    token_program.key(),
                    TransferChecked {
                        from: vault.to_account_info(),
                        to: ctx.accounts.holders_recipient.to_account_info(),
                        mint: mint_info.clone(),
                        authority: treasury.to_account_info(),
                    },
                    signer,
                ),
                holders_amt,
                decimals,
            )?;
        }

        // 20% → project dev
        if dev_amt > 0 {
            token_interface::transfer_checked(
                CpiContext::new_with_signer(
                    token_program.key(),
                    TransferChecked {
                        from: vault.to_account_info(),
                        to: ctx.accounts.dev_recipient.to_account_info(),
                        mint: mint_info.clone(),
                        authority: treasury.to_account_info(),
                    },
                    signer,
                ),
                dev_amt,
                decimals,
            )?;
        }

        // 10% → ecosystem (+ rounding dust)
        if eco_amt > 0 {
            token_interface::transfer_checked(
                CpiContext::new_with_signer(
                    token_program.key(),
                    TransferChecked {
                        from: vault.to_account_info(),
                        to: ctx.accounts.ecosystem_recipient.to_account_info(),
                        mint: mint_info,
                        authority: treasury.to_account_info(),
                    },
                    signer,
                ),
                eco_amt,
                decimals,
            )?;
        }

        // Update cumulative tracking
        treasury.total_distributed_holders = treasury.total_distributed_holders.saturating_add(holders_amt);
        treasury.total_distributed_dev = treasury.total_distributed_dev.saturating_add(dev_amt);
        treasury.total_distributed_ecosystem = treasury.total_distributed_ecosystem.saturating_add(eco_amt);

        // Audit event — every redistribution is on-chain verifiable
        let clock = Clock::get()?;
        emit!(Redistribution {
            mint: mint_key,
            excess,
            holders_amount: holders_amt,
            dev_amount: dev_amt,
            ecosystem_amount: eco_amt,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Fund swarm operations from the treasury vault.
    ///
    /// Enforces the 90-day runway invariant (CLAUDE.md #9):
    /// post-hydration balance MUST remain >= `min_runway_balance`.
    /// Transfers tokens to the swarm hydration PDA for swap to USDC.
    pub fn hydrate_swarm(ctx: Context<HydrateSwarm>, amount: u64) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let treasury = &mut ctx.accounts.treasury;
        let vault = &ctx.accounts.treasury_vault;

        // Strategy lifecycle gate: only Live strategies can receive funding
        require!(
            ctx.accounts.strategy_record.status == StrategyLifecycleStatus::Live,
            TreasuryError::StrategyNotLive,
        );

        // Beta expiry gate: if this treasury has an adopter record with a
        // beta expiry set, refuse funding after expiry or manual end.
        // Permanent adopters (beta_expires_at == 0) are not affected.
        let adopter = &ctx.accounts.adopter_record;
        if adopter.beta_expires_at > 0 {
            let clock = Clock::get()?;
            require!(
                !adopter.beta_ended && clock.unix_timestamp < adopter.beta_expires_at,
                TreasuryError::BetaExpired,
            );
        }

        require!(amount > 0, TreasuryError::HydrationExceedsBalance);
        require!(vault.amount >= amount, TreasuryError::HydrationExceedsBalance);

        // F-001 fix: enforce the stored runway floor, not a percentage heuristic
        let post_balance = vault.amount.saturating_sub(amount);
        require!(
            post_balance >= treasury.min_runway_balance,
            TreasuryError::InsufficientRunway,
        );

        let mint_key = ctx.accounts.mint.key();
        let seeds = &[TREASURY_SEED, mint_key.as_ref(), &[treasury.bump]];

        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: vault.to_account_info(),
                    to: ctx.accounts.swarm_vault.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    authority: treasury.to_account_info(),
                },
                &[seeds],
            ),
            amount,
            ctx.accounts.mint.decimals,
        )?;

        treasury.total_hydration = treasury.total_hydration.saturating_add(amount);
        Ok(())
    }

    /// Evolve the treasury phase. IRREVERSIBLE.
    ///
    /// Phase thresholds (USDC value of treasury reserves):
    /// - Sustenance → Ecosystem:  >= $50k   (SUSTENANCE_CAP)
    /// - Ecosystem   → Humanity:  >= $1M    (ECOSYSTEM_CAP)
    ///
    /// Production: these thresholds should be validated against an on-chain
    /// oracle (e.g. Pyth). For devnet, the phase_authority signature is
    /// the guard — the authority is responsible for checking reserves.
    ///
    /// Only the treasury authority can trigger (Squads Multisig compatible).
    pub fn evolve_phase(ctx: Context<EvolvePhase>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let treasury = &mut ctx.accounts.treasury;
        let vault_balance = ctx.accounts.treasury_vault.amount;

        // Authority is verified by Anchor constraint on the account struct.
        // S-002 fix: removed duplicate manual check — single guard is
        // the canonical source of truth (spec-lock principle).

        let next = match treasury.phase {
            Phase::Sustenance => {
                // C-1 fix: enforce vault balance against SUSTENANCE_CAP.
                // Production: replace with oracle-denominated USDC value.
                require!(
                    vault_balance >= SUSTENANCE_CAP,
                    TreasuryError::BelowThreshold,
                );
                Phase::Ecosystem
            }
            Phase::Ecosystem => {
                // C-1 fix: enforce vault balance against ECOSYSTEM_CAP.
                // Production: replace with oracle-denominated USDC value.
                require!(
                    vault_balance >= ECOSYSTEM_CAP,
                    TreasuryError::BelowThreshold,
                );
                Phase::Humanity
            }
            Phase::Humanity => return Err(TreasuryError::AlreadyMaxPhase.into()),
        };

        // F-005: production path — validate USDC reserves against phase cap
        // via on-chain oracle (Pyth/Switchboard). For devnet, vault balance
        // denominated in the mint's native token serves as the guard.
        // Thresholds are documented here for auditability.

        treasury.phase = next;
        Ok(())
    }

    /// Verify that the mint has TransferFeeConfig enabled and that the
    /// Treasury PDA is the `withdraw_withheld_authority`.
    ///
    /// READ-ONLY instruction — no state mutation. Deserializes the mint
    /// account data (base Mint + TLV extensions) and confirms the withdraw
    /// authority matches the Treasury PDA.
    ///
    /// SL-001/SL-002 fix: on-chain adoption verification instead of
    /// relying on off-chain "did you configure the mint?" trust.
    pub fn verify_adoption(ctx: Context<VerifyAdoption>) -> Result<()> {
        let mint_info = ctx.accounts.mint.to_account_info();
        let treasury_key = ctx.accounts.treasury.key();
        verify_transfer_fee_config(&mint_info, &treasury_key)
    }

    /// Create the swarm hydration PDA vault.
    ///
    /// Must be called once after `initialize` and before `hydrate_swarm`.
    /// S-001 fix: replaces `init_if_needed` with explicit initialization
    /// to prevent re-initialization attacks on the swarm vault.
    pub fn create_swarm_vault(ctx: Context<CreateSwarmVault>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        Ok(())
    }

    /// Register a new token project as an RTP adopter (permanent — no expiry).
    ///
    /// Creates an AdopterRecord PDA for the given token mint. Called once
    /// per adopting token project at adoption time. The AdopterRecord tracks
    /// cumulative fee contributions for pro-rata yield attribution.
    pub fn register_adopter(ctx: Context<RegisterAdopter>, token_mint: Pubkey) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let record = &mut ctx.accounts.adopter_record;
        let clock = Clock::get()?;

        record.token_mint = token_mint;
        record.treasury = ctx.accounts.treasury.key();
        record.fees_contributed_lamports = 0;
        record.adopted_at = clock.unix_timestamp;
        record.last_deposit_ts = clock.unix_timestamp;
        record.deposit_count = 0;
        record.beta_expires_at = 0; // permanent — no expiry
        record.beta_ended = false;
        record.bump = ctx.bumps.adopter_record;

        emit!(AdopterRegistered {
            token_mint,
            adopted_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Register a beta adopter with an automatic expiry timestamp.
    ///
    /// Same as register_adopter but sets `beta_expires_at`. After this
    /// timestamp, hydrate_swarm will refuse to fund strategies for this
    /// adopter. The beta can also be ended early via `end_beta`.
    ///
    /// Typical use: Colosseum hackathon beta — expires 1 week after the
    /// hackathon deadline.
    pub fn register_adopter_beta(
        ctx: Context<RegisterAdopter>,
        token_mint: Pubkey,
        beta_expires_at: i64,
    ) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let clock = Clock::get()?;
        require!(
            beta_expires_at > clock.unix_timestamp,
            TreasuryError::BetaExpired,
        );

        let record = &mut ctx.accounts.adopter_record;

        record.token_mint = token_mint;
        record.treasury = ctx.accounts.treasury.key();
        record.fees_contributed_lamports = 0;
        record.adopted_at = clock.unix_timestamp;
        record.last_deposit_ts = clock.unix_timestamp;
        record.deposit_count = 0;
        record.beta_expires_at = beta_expires_at;
        record.beta_ended = false;
        record.bump = ctx.bumps.adopter_record;

        emit!(AdopterRegistered {
            token_mint,
            adopted_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Record a fee deposit from an adopting token project.
    ///
    /// Increments the AdopterRecord's cumulative fees and the treasury's
    /// total_fees_received_lamports. This is the accounting hook called
    /// alongside (or composed into) any fee deposit. It does not move
    /// funds — it only updates accounting state for pro-rata attribution.
    pub fn record_fee_deposit(ctx: Context<RecordFeeDeposit>, amount_lamports: u64) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        require!(amount_lamports > 0, TreasuryError::ZeroAmount);

        let record = &mut ctx.accounts.adopter_record;
        let treasury = &mut ctx.accounts.treasury;
        let clock = Clock::get()?;

        record.fees_contributed_lamports = record
            .fees_contributed_lamports
            .checked_add(amount_lamports)
            .ok_or(TreasuryError::Overflow)?;
        record.last_deposit_ts = clock.unix_timestamp;
        record.deposit_count = record.deposit_count.saturating_add(1);

        treasury.total_fees_received_lamports = treasury
            .total_fees_received_lamports
            .checked_add(amount_lamports)
            .ok_or(TreasuryError::Overflow)?;

        emit!(FeeDepositRecorded {
            token_mint: record.token_mint,
            amount_lamports,
            cumulative: record.fees_contributed_lamports,
            total_treasury_fees: treasury.total_fees_received_lamports,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// End a beta adopter's RTP participation early.
    ///
    /// Only callable by `treasury.authority`. Sets `beta_ended = true`,
    /// which prevents further hydrate_swarm funding for this adopter.
    /// The adopter's fee contributions remain on record for attribution.
    /// Yield already generated stays with the project.
    pub fn end_beta(ctx: Context<EndBeta>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        // Authority validated by Anchor constraint on EndBeta struct.

        let record = &mut ctx.accounts.adopter_record;
        let clock = Clock::get()?;
        record.beta_ended = true;

        emit!(BetaEnded {
            token_mint: record.token_mint,
            ended_at: clock.unix_timestamp,
            fees_contributed_lamports: record.fees_contributed_lamports,
        });

        Ok(())
    }

    /// Register (promote) a strategy from the Python research layer into
    /// on-chain LIVE status. Only callable by `treasury.authority`.
    pub fn register_strategy(
        ctx: Context<RegisterStrategy>,
        strategy_id: String,
        promotion_sharpe_x100: i32,
    ) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        require!(
            strategy_id.len() >= 1 && strategy_id.len() <= 16,
            TreasuryError::InvalidStrategyId,
        );
        // Authority validated by Anchor constraint on RegisterStrategy struct.

        let clock = Clock::get()?;
        let record = &mut ctx.accounts.strategy_record;
        record.treasury = ctx.accounts.treasury.key();
        record.strategy_id = strategy_id.clone();
        record.status = StrategyLifecycleStatus::Live;
        record.promoted_at = clock.unix_timestamp;
        record.last_update_ts = clock.unix_timestamp;
        record.rolling_pnl_bps = 0;
        record.consecutive_losses = 0;
        record.soft_decay_strikes = 0;
        record.recovery_counter = 0;
        record.drawdown_24h_bps = 0;
        record.total_trades = 0;
        record.promotion_sharpe_x100 = promotion_sharpe_x100;
        record.rolling_sharpe_x100 = promotion_sharpe_x100;
        record.open_position_count = 0;
        record.committed_sol_lamports = 0;
        record.flash_pool_name = String::default();
        record.bump = ctx.bumps.strategy_record;

        emit!(StrategyPromoted {
            treasury: ctx.accounts.treasury.key(),
            strategy_id,
            promotion_sharpe_x100,
            promoted_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Update strategy performance metrics after each completed trade batch.
    /// Enforces hard stop and soft decay thresholds automatically.
    pub fn update_strategy_performance(
        ctx: Context<UpdateStrategyPerformance>,
        rolling_pnl_bps: i32,
        rolling_sharpe_x100: i32,
        consecutive_losses: u8,
        drawdown_24h_bps: u16,
        new_soft_strike: bool,
    ) -> Result<()> {
        // Authority gate — only treasury.authority can update strategy metrics.
        // Without this, any signer could write arbitrary PnL/Sharpe/strikes
        // and keep a bad strategy Live forever.
        require!(
            ctx.accounts.authority.key() == ctx.accounts.treasury.authority,
            TreasuryError::UnauthorizedStrategyOp,
        );
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let record = &mut ctx.accounts.strategy_record;
        require!(
            record.status == StrategyLifecycleStatus::Live,
            TreasuryError::StrategyNotLive,
        );

        // 2. Update all metric fields
        record.rolling_pnl_bps = rolling_pnl_bps;
        record.rolling_sharpe_x100 = rolling_sharpe_x100;
        record.consecutive_losses = consecutive_losses;
        record.drawdown_24h_bps = drawdown_24h_bps;

        // 3. Increment soft decay strikes, or track recovery toward a reset.
        // Strikes only reset after MIN_RECOVERY_TRADES consecutive positive
        // updates — a single lucky trade cannot clear the strike count.
        if new_soft_strike {
            record.soft_decay_strikes = record.soft_decay_strikes.saturating_add(1);
            record.recovery_counter = 0; // New strike resets recovery progress
        } else if rolling_pnl_bps > 0 && rolling_sharpe_x100 > 0 {
            record.recovery_counter = record.recovery_counter.saturating_add(1);
            if record.recovery_counter >= MIN_RECOVERY_TRADES {
                // Sustained recovery: reset strikes
                record.soft_decay_strikes = 0;
                record.recovery_counter = 0;
            }
        } else {
            // Neither strike nor recovery — reset recovery counter
            record.recovery_counter = 0;
        }

        // 4. Increment total trades
        record.total_trades = record.total_trades.saturating_add(1);

        // 5. Set last_update_ts
        let clock = Clock::get()?;
        record.last_update_ts = clock.unix_timestamp;

        // Hard stop checks (order matters — first match wins)
        let mut retirement_reason: Option<RetirementReason> = None;

        if drawdown_24h_bps >= HARD_DRAWDOWN_24H_BPS {
            record.status = StrategyLifecycleStatus::Suspended;
            retirement_reason = Some(RetirementReason::HardDrawdown);
        } else if consecutive_losses >= HARD_CONSECUTIVE_LOSSES {
            record.status = StrategyLifecycleStatus::Suspended;
            retirement_reason = Some(RetirementReason::ConsecutiveLosses);
        } else if rolling_sharpe_x100 < HARD_ROLLING_SHARPE_MIN_X100 {
            record.status = StrategyLifecycleStatus::Suspended;
            retirement_reason = Some(RetirementReason::RollingSharpeLow);
        }

        // Soft decay retirement check
        if record.soft_decay_strikes >= SOFT_STRIKE_THRESHOLD {
            record.status = StrategyLifecycleStatus::Retired;
            retirement_reason = Some(RetirementReason::SoftDecayStrikes);
        }

        // Emit retirement event if triggered
        if let Some(reason) = retirement_reason {
            emit!(StrategyRetired {
                treasury: ctx.accounts.treasury.key(),
                strategy_id: record.strategy_id.clone(),
                reason,
                final_rolling_sharpe_x100: record.rolling_sharpe_x100,
                ts: clock.unix_timestamp,
            });
        }

        // 6. Always emit performance update (audit trail)
        emit!(StrategyPerformanceUpdated {
            treasury: ctx.accounts.treasury.key(),
            strategy_id: record.strategy_id.clone(),
            rolling_pnl_bps: record.rolling_pnl_bps,
            rolling_sharpe_x100: record.rolling_sharpe_x100,
            consecutive_losses: record.consecutive_losses,
            soft_decay_strikes: record.soft_decay_strikes,
            recovery_counter: record.recovery_counter,
            drawdown_24h_bps: record.drawdown_24h_bps,
            status: record.status,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Emergency manual retirement by treasury authority. Bypasses thresholds.
    pub fn force_retire_strategy(ctx: Context<ForceRetireStrategy>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        // Authority validated by Anchor constraint on ForceRetireStrategy struct.

        let record = &mut ctx.accounts.strategy_record;
        let clock = Clock::get()?;
        record.status = StrategyLifecycleStatus::Retired;

        emit!(StrategyRetired {
            treasury: ctx.accounts.treasury.key(),
            strategy_id: record.strategy_id.clone(),
            reason: RetirementReason::AuthorityForced,
            final_rolling_sharpe_x100: record.rolling_sharpe_x100,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Emergency freeze: authority-gated, sets frozen = true.
    /// In production, authority is the Squads multisig PDA — requires 2-of-3 approval.
    /// No time lock on freeze (emergency speed). Unfreeze requires 24h time lock.
    pub fn freeze_treasury(ctx: Context<FreezeTreasury>) -> Result<()> {
        let treasury = &mut ctx.accounts.treasury;
        require!(!treasury.frozen, TreasuryError::AlreadyFrozen);

        treasury.frozen = true;
        let clock = Clock::get()?;
        emit!(TreasuryFrozen {
            mint: treasury.mint,
            authority: ctx.accounts.authority.key(),
            timestamp: clock.unix_timestamp,
        });
        Ok(())
    }

    /// Unfreeze: authority-gated, sets frozen = false.
    /// In production, requires Squads 2-of-3 + 24h time lock.
    pub fn unfreeze_treasury(ctx: Context<UnfreezeTreasury>) -> Result<()> {
        let treasury = &mut ctx.accounts.treasury;
        require!(treasury.frozen, TreasuryError::NotFrozen);

        treasury.frozen = false;
        let clock = Clock::get()?;
        emit!(TreasuryUnfrozen {
            mint: treasury.mint,
            authority: ctx.accounts.authority.key(),
            timestamp: clock.unix_timestamp,
        });
        Ok(())
    }

    // ==== Flash Trade CPI Instructions ======================================

    /// Open a Flash Trade perpetual position via CPI, signed by Treasury PDA.
    ///
    /// Constraints enforced before CPI:
    /// 1. Treasury not frozen
    /// 2. Strategy must be Live
    /// 3. open_position_count < MAX_CONCURRENT_POSITIONS (3)
    /// 4. Vault balance after commit >= min_runway_balance
    /// 5. input_sol_lamports <= vault * MAX_POSITION_SIZE_BPS / 10000
    ///
    /// Flash Trade accounts are passed via remaining_accounts in IDL v15.2.0 order:
    /// 0: owner (treasury PDA, signer via invoke_signed)
    /// 1: fee_payer (authority, pays rent)
    /// 2: funding_account (WSOL temp account)
    /// 3: transfer_authority (Flash Trade PDA)
    /// 4: perpetuals (Flash Trade PDA)
    /// 5: pool (writable)
    /// 6: position (writable, PDA)
    /// 7: market (writable)
    /// 8: target_custody
    /// 9: target_oracle_account
    /// 10: collateral_custody (writable)
    /// 11: collateral_oracle_account
    /// 12: collateral_custody_token_account (writable)
    /// 13: system_program
    /// 14: funding_token_program
    /// 15: event_authority (Flash Trade PDA)
    /// 16: program (Flash Trade program ID)
    /// 17: ix_sysvar
    /// 18: funding_mint
    pub fn open_flash_position(
        ctx: Context<OpenFlashPosition>,
        side: FlashSide,
        input_sol_lamports: u64,
        leverage_bps: u32,
        slippage_bps: u16,
        oracle_price: FlashOraclePrice,
        pool_name: String,
    ) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);

        // Validate pool name: 1..=32 chars (matches StrategyRecord.flash_pool_name max_len).
        require!(
            !pool_name.is_empty() && pool_name.len() <= 32,
            TreasuryError::InvalidPoolName,
        );

        // Validate slippage: must be <= 10000 bps (100%) to prevent negative slippage prices
        require!(slippage_bps <= 10000, TreasuryError::PositionSizeExceeded);

        // Validate leverage: must be <= 1_000_000 bps (100x) to prevent overflow
        require!(leverage_bps <= 1_000_000, TreasuryError::PositionSizeExceeded);

        let treasury = &mut ctx.accounts.treasury;
        let strategy = &mut ctx.accounts.strategy_record;

        // Strategy lifecycle gate
        require!(
            strategy.status == StrategyLifecycleStatus::Live,
            TreasuryError::StrategyNotLive,
        );

        // Max concurrent positions gate
        require!(
            strategy.open_position_count < MAX_CONCURRENT_POSITIONS,
            TreasuryError::TooManyOpenPositions,
        );

        // Read the *token* balance of the treasury vault.
        // Units MUST match `min_runway_balance` and `input_sol_lamports`
        // (callers commit a wSOL/SOL-denominated token whose decimals match
        // SOL lamports — see Initialize). Previously this read .lamports()
        // off an UncheckedAccount which only returned rent dust and made the
        // runway check vacuous. P0 fix: use the typed TokenAccount amount.
        let vault_balance = ctx.accounts.treasury_vault.amount;

        // Position size cap (20% of vault)
        let max_input = vault_balance as u128 * MAX_POSITION_SIZE_BPS as u128 / 10000;
        require!(
            input_sol_lamports as u128 <= max_input,
            TreasuryError::PositionSizeExceeded,
        );

        // Runway floor: vault after commit must still cover min_runway_balance
        let post_commit = vault_balance.saturating_sub(input_sol_lamports);
        require!(
            post_commit >= treasury.min_runway_balance,
            TreasuryError::InsufficientRunway,
        );

        // Flash Trade program ID validation
        let flash_program_id = Pubkey::try_from(FLASH_TRADE_PROGRAM_ID)
            .map_err(|_| TreasuryError::InvalidFlashProgramId)?;

        // Build Flash Trade open_position instruction data
        // Discriminator (8 bytes) + OraclePrice (12 bytes) + collateralAmount (8) + sizeAmount (8) + privilege (1) = 37 bytes
        // Reject FlashSide::None for open — a position must have a direction
        require!(side != FlashSide::None, TreasuryError::InvalidFlashSide);

        let size_amount = input_sol_lamports as u128 * leverage_bps as u128 / 10000;
        // Bounds check before truncation to u64
        require!(size_amount <= u64::MAX as u128, TreasuryError::Overflow);
        let slippage_mult = 10000u32 + slippage_bps as u32;
        let slippage_price = if side == FlashSide::Long {
            oracle_price.price as i128 * slippage_mult as i128 / 10000
        } else {
            oracle_price.price as i128 * (20000 - slippage_mult as i128) / 10000
        };

        let mut ix_data = Vec::with_capacity(37);
        ix_data.extend_from_slice(&FLASH_OPEN_POSITION_DISC);
        // OraclePrice: price (i64 LE) + exponent (i32 LE)
        ix_data.extend_from_slice(&(slippage_price as i64).to_le_bytes());
        ix_data.extend_from_slice(&oracle_price.exponent.to_le_bytes());
        // collateralAmount (u64 LE)
        ix_data.extend_from_slice(&input_sol_lamports.to_le_bytes());
        // sizeAmount (u64 LE) — safe after bounds check above
        ix_data.extend_from_slice(&(size_amount as u64).to_le_bytes());
        // privilege (u8) — None = 0
        ix_data.push(0u8);

        // Build account metas from remaining_accounts
        let remaining = ctx.remaining_accounts;
        require!(
            remaining.len() >= 19,
            TreasuryError::FlashCpiFailed,
        );

        // Validate Flash Trade program ID at remaining[16] (per IDL v15.2.0 layout).
        // Close handler validates position PDA; open handler validates program ID.
        // This ensures the CPI targets the expected program, not a malicious substitute.
        require!(
            remaining[16].key() == flash_program_id,
            TreasuryError::InvalidFlashProgramId,
        );

        // Validate Flash Trade event authority PDA at remaining[15].
        // Anchor #[event_cpi] derives this PDA from ["__event_authority"] under
        // the Flash Trade program. Substituting another account here would let
        // a caller redirect Flash Trade's emitted CPI events to a fake authority.
        let (expected_event_authority, _evt_bump) = Pubkey::find_program_address(
            &[b"__event_authority"],
            &flash_program_id,
        );
        require!(
            remaining[15].key() == expected_event_authority,
            TreasuryError::InvalidFlashEventAuthority,
        );

        // Validate canonical System Program at remaining[13]. A malicious
        // substitute could front-run lamport math during CPI account creation.
        require!(
            remaining[13].key() == anchor_lang::system_program::ID,
            TreasuryError::InvalidFlashSystemProgram,
        );

        // Validate funding_token_program at remaining[14]. Flash Trade's
        // funding flow uses either the legacy SPL Token program or Token-2022.
        // Reject anything else so the CPI cannot be tricked into using a
        // counterfeit token program.
        let token_program_key = remaining[14].key();
        require!(
            token_program_key == anchor_spl::token::ID
                || token_program_key == anchor_spl::token_2022::ID,
            TreasuryError::InvalidFlashTokenProgram,
        );

        // Validate treasury PDA at remaining[0] matches our treasury signer.
        require!(
            remaining[0].key() == treasury.key(),
            TreasuryError::FlashCpiFailed,
        );

        // Account layout (matches Flash Trade IDL v15.2.0):
        //   0: owner (treasury PDA, signs via invoke_signed — readonly signer)
        //   1: fee_payer (authority, writable signer)
        //   2,5,6,7,10,12: writable accounts (funding/pool/position/market/custodies)
        //   all others: readonly
        let account_metas: Vec<AccountMeta> = remaining.iter().enumerate().map(|(i, acc)| {
            match i {
                0 => AccountMeta::new_readonly(acc.key(), true), // PDA signer
                1 => AccountMeta::new(acc.key(), true),           // fee_payer
                2 | 5 | 6 | 7 | 10 | 12 => AccountMeta::new(acc.key(), false),
                _ => AccountMeta::new_readonly(acc.key(), false),
            }
        }).collect();

        let flash_ix = Instruction {
            program_id: flash_program_id,
            accounts: account_metas,
            data: ix_data,
        };

        // Sign with Treasury PDA seeds
        let mint_key = treasury.mint;
        let seeds = &[TREASURY_SEED, mint_key.as_ref(), &[treasury.bump]];
        let signer_seeds = &[&seeds[..]];

        invoke_signed(&flash_ix, &remaining.iter().map(|a| a.to_account_info()).collect::<Vec<_>>(), signer_seeds)
            .map_err(|_| TreasuryError::FlashCpiFailed)?;

        // Update strategy state
        strategy.open_position_count = strategy.open_position_count.saturating_add(1);
        strategy.committed_sol_lamports = strategy.committed_sol_lamports.saturating_add(input_sol_lamports);
        strategy.flash_pool_name = pool_name.clone();

        let clock = Clock::get()?;
        emit!(FlashPositionOpened {
            treasury: treasury.key(),
            strategy_id: strategy.strategy_id.clone(),
            side,
            input_sol_lamports,
            leverage_bps,
            pool_name,
            position_pda: remaining.get(6).map(|a| a.key()).unwrap_or_default(),
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Close a Flash Trade perpetual position via CPI.
    ///
    /// Closing is permitted even if strategy is Suspended (exiting is always safe).
    /// Treasury frozen check still applies.
    ///
    /// Flash Trade close_position accounts (18 accounts from IDL v15.2.0):
    /// 0: owner (treasury PDA, signer)
    /// 1: fee_payer (authority)
    /// 2: receiving_account (writable)
    /// 3: transfer_authority
    /// 4: perpetuals
    /// 5: pool (writable)
    /// 6: position (writable)
    /// 7: market (writable)
    /// 8: target_custody
    /// 9: target_oracle_account
    /// 10: collateral_custody (writable)
    /// 11: collateral_oracle_account
    /// 12: collateral_custody_token_account (writable)
    /// 13: token_program
    /// 14: event_authority
    /// 15: program
    /// 16: ix_sysvar
    /// 17: collateral_mint
    pub fn close_flash_position(
        ctx: Context<CloseFlashPosition>,
        side: FlashSide,
        oracle_price: FlashOraclePrice,
        slippage_bps: u16,
        committed_sol_lamports_delta: u64,
    ) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        // Reject FlashSide::None for close — must specify direction for slippage
        require!(side != FlashSide::None, TreasuryError::InvalidFlashSide);
        let treasury = &mut ctx.accounts.treasury;
        let strategy = &mut ctx.accounts.strategy_record;

        // Validate slippage: must be <= 10000 bps (100%) to prevent negative slippage prices
        require!(slippage_bps <= 10000, TreasuryError::PositionSizeExceeded);

        let flash_program_id = Pubkey::try_from(FLASH_TRADE_PROGRAM_ID)
            .map_err(|_| TreasuryError::InvalidFlashProgramId)?;

        let remaining = ctx.remaining_accounts;
        require!(
            remaining.len() >= 18,
            TreasuryError::FlashCpiFailed,
        );

        // §2.6 fix: verify the position PDA at remaining[6] is derived from
        // ["position", treasury_pda, market], proving the treasury PDA owns
        // the position before signing the close. We check both bumps via
        // find_program_address — if the supplied position is owned by any
        // other authority, the derivation will not match.
        let position_account = &remaining[6];
        let market_account = &remaining[7];
        let (expected_position, _bump) = Pubkey::find_program_address(
            &[b"position", treasury.key().as_ref(), market_account.key.as_ref()],
            &flash_program_id,
        );
        require!(
            position_account.key() == expected_position,
            TreasuryError::PositionNotOwnedByTreasury,
        );

        // §2.4 fix: slippage direction depends on side.
        // Closing a long sells the target → accept exit price >= oracle * (1 - slip)
        // Closing a short buys back the target → accept exit price <= oracle * (1 + slip)
        // We always pass the worst-acceptable price; Flash Trade rejects worse fills.
        let slip = slippage_bps as i128;
        let slippage_price: i128 = match side {
            FlashSide::Long => oracle_price.price as i128 * (10_000 - slip) / 10_000,
            FlashSide::Short => oracle_price.price as i128 * (10_000 + slip) / 10_000,
            FlashSide::None => oracle_price.price as i128,
        };

        // Build close_position instruction data: disc (8) + OraclePrice (12)
        // + sizeUsd (8) + privilege (1) = 29 bytes.
        // Matches Flash Trade IDL v15.2.0 close_position (verified on mainnet).
        // u64::MAX = full close.
        let mut ix_data = Vec::with_capacity(29);
        ix_data.extend_from_slice(&FLASH_CLOSE_POSITION_DISC);
        ix_data.extend_from_slice(&(slippage_price as i64).to_le_bytes());
        ix_data.extend_from_slice(&oracle_price.exponent.to_le_bytes());
        ix_data.extend_from_slice(&u64::MAX.to_le_bytes()); // sizeUsd: full close
        ix_data.push(0u8); // privilege: None

        let account_metas: Vec<AccountMeta> = remaining.iter().enumerate().map(|(i, acc)| {
            match i {
                0 => AccountMeta::new_readonly(acc.key(), true), // PDA signer
                2 | 5 | 6 | 7 | 10 | 12 => AccountMeta::new(acc.key(), false),
                _ => AccountMeta::new_readonly(acc.key(), false),
            }
        }).collect();

        let flash_ix = Instruction {
            program_id: flash_program_id,
            accounts: account_metas,
            data: ix_data,
        };

        let mint_key = treasury.mint;
        let seeds = &[TREASURY_SEED, mint_key.as_ref(), &[treasury.bump]];
        let signer_seeds = &[&seeds[..]];

        invoke_signed(&flash_ix, &remaining.iter().map(|a| a.to_account_info()).collect::<Vec<_>>(), signer_seeds)
            .map_err(|_| TreasuryError::FlashCpiFailed)?;

        // §2.9 fix: decrement both counters. The caller passes the input
        // amount that was committed at open time (typically the same value
        // they passed to open_flash_position). For a partial close, the
        // caller can pass the proportional delta. Reject under-flow rather
        // than silently saturating to keep accounting honest.
        strategy.open_position_count = strategy.open_position_count.saturating_sub(1);
        require!(
            strategy.committed_sol_lamports >= committed_sol_lamports_delta,
            TreasuryError::CommittedDeltaExceedsBalance,
        );
        strategy.committed_sol_lamports -= committed_sol_lamports_delta;

        let clock = Clock::get()?;
        emit!(FlashPositionClosed {
            treasury: treasury.key(),
            strategy_id: strategy.strategy_id.clone(),
            position_pda: position_account.key(),
            realised_pnl_sol_lamports: 0, // Filled in by agent via update_strategy_performance
            returned_sol_lamports: 0,     // Filled in by agent via update_strategy_performance
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Emergency reset of Flash Trade position counters.
    /// Authority-gated. Designed to be called *together with* `freeze_treasury`
    /// (in either order) so it is intentionally NOT blocked by `treasury.frozen`.
    ///
    /// What it does:
    ///   1. Resets `open_position_count` and `committed_sol_lamports` to 0
    ///   2. Emits an `EmergencyPositionsReset` event for the audit trail
    ///
    /// What it does NOT do:
    ///   - It does **not** invoke Flash Trade CPI close. Operators must follow
    ///     up with explicit `close_flash_position` calls per position (or rely
    ///     on Flash Trade keeper liquidation) to actually unwind exposure.
    ///   - The event is deliberately distinct from `FlashPositionClosed` so
    ///     observers cannot mistake a counter reset for a real position close.
    pub fn emergency_close_all_positions(
        ctx: Context<EmergencyCloseAllPositions>,
        position_pubkeys: Vec<Pubkey>,
    ) -> Result<()> {
        require!(
            position_pubkeys.len() <= MAX_CONCURRENT_POSITIONS as usize,
            TreasuryError::TooManyOpenPositions,
        );
        require!(
            ctx.accounts.authority.key() == ctx.accounts.treasury.authority,
            TreasuryError::UnauthorizedStrategyOp,
        );

        let strategy = &mut ctx.accounts.strategy_record;
        let previous_committed = strategy.committed_sol_lamports;
        let clock = Clock::get()?;

        // Reset counters first (audit captures the post-state intent).
        strategy.open_position_count = 0;
        strategy.committed_sol_lamports = 0;

        emit!(EmergencyPositionsReset {
            treasury: ctx.accounts.treasury.key(),
            strategy_id: strategy.strategy_id.clone(),
            authority: ctx.accounts.authority.key(),
            position_pubkeys,
            previous_committed_sol_lamports: previous_committed,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    // ==== Account Contexts ==================================================

    #[derive(Accounts)]
    pub struct Initialize<'info> {
        /// The Token-2022 mint adopting RTP.
        /// MUST have TransferFeeConfig enabled with the Treasury PDA as
        /// `withdraw_withheld_authority` (immutable once set).
        #[account(mint::token_program = token_program)]
        pub mint: InterfaceAccount<'info, Mint>,

        /// Treasury state account (PDA). No private key exists.
        #[account(
            init,
            payer = authority,
            space = 8 + Treasury::INIT_SPACE,
            seeds = [TREASURY_SEED, mint.key().as_ref()],
            bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// PDA-owned vault that receives withdrawn fees.
        /// Authority = treasury PDA (no human can sign for this).
        #[account(
            init,
            payer = authority,
            token::mint = mint,
            token::authority = treasury,
            seeds = [TREASURY_SEED, mint.key().as_ref(), b"vault"],
            bump,
        )]
        pub treasury_vault: InterfaceAccount<'info, TokenAccount>,

        /// Holders wallet — receives 70% of redistribution.
        /// Stored as pubkey in treasury state for on-chain verification.
        /// CHECK: plain pubkey, no data read
        pub holders_wallet: UncheckedAccount<'info>,

        /// Project dev wallet — receives 20% of redistribution.
        /// Stored as pubkey in treasury state for on-chain verification.
        /// CHECK: plain pubkey, no data read
        pub project_dev_wallet: UncheckedAccount<'info>,

        /// Ecosystem wallet — receives 10% of redistribution.
        /// Stored as pubkey in treasury state for on-chain verification.
        /// CHECK: plain pubkey, no data read
        pub ecosystem_wallet: UncheckedAccount<'info>,

        /// Authority paying for initialization (anyone can initialize).
        #[account(mut)]
        pub authority: Signer<'info>,

        pub token_program: Interface<'info, TokenInterface>,
        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct WithdrawFees<'info> {
        /// The Token-2022 mint with TransferFeeConfig enabled.
        /// `mut` required: CPI `withdraw_withheld_tokens_from_mint` marks
        /// mint as writable in its account metas.
        #[account(mut, mint::token_program = token_program)]
        pub mint: InterfaceAccount<'info, Mint>,

        /// Treasury state account (PDA).
        #[account(
            mut,
            seeds = [TREASURY_SEED, mint.key().as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Treasury vault where withdrawn fees land.
        /// Authority = treasury PDA.
        #[account(
            mut,
            token::mint = mint,
            token::authority = treasury,
            seeds = [TREASURY_SEED, mint.key().as_ref(), b"vault"],
            bump,
        )]
        pub treasury_vault: InterfaceAccount<'info, TokenAccount>,

        pub token_program: Interface<'info, TokenInterface>,
    }

    #[derive(Accounts)]
    pub struct CheckRedistribute<'info> {
        /// The Token-2022 mint.
        #[account(mint::token_program = token_program)]
        pub mint: InterfaceAccount<'info, Mint>,

        /// Treasury state account (PDA).
        #[account(
            mut,
            seeds = [TREASURY_SEED, mint.key().as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Treasury vault (source of redistribution).
        /// Authority = treasury PDA.
        #[account(
            mut,
            token::mint = mint,
            token::authority = treasury,
            seeds = [TREASURY_SEED, mint.key().as_ref(), b"vault"],
            bump,
        )]
        pub treasury_vault: InterfaceAccount<'info, TokenAccount>,

        /// Holder distribution recipient token account.
        /// Authority verified against `treasury.holders_wallet`.
        /// Boxed to reduce stack frame size (BPF 4KB limit).
        #[account(
            mut,
            token::mint = mint,
            token::authority = treasury.holders_wallet,
        )]
        pub holders_recipient: Box<InterfaceAccount<'info, TokenAccount>>,

        /// Project dev wallet token account. Authority verified against
        /// `treasury.project_dev_wallet` stored at initialization.
        /// Boxed to reduce stack frame size (BPF 4KB limit).
        #[account(
            mut,
            token::mint = mint,
            token::authority = treasury.project_dev_wallet,
        )]
        pub dev_recipient: Box<InterfaceAccount<'info, TokenAccount>>,

        /// Ecosystem wallet token account. Authority verified against
        /// `treasury.ecosystem_wallet` stored at initialization.
        /// Boxed to reduce stack frame size (BPF 4KB limit).
        #[account(
            mut,
            token::mint = mint,
            token::authority = treasury.ecosystem_wallet,
        )]
        pub ecosystem_recipient: Box<InterfaceAccount<'info, TokenAccount>>,

        pub token_program: Interface<'info, TokenInterface>,
    }

    #[derive(Accounts)]
    pub struct HydrateSwarm<'info> {
        /// The Token-2022 mint.
        #[account(mint::token_program = token_program)]
        pub mint: InterfaceAccount<'info, Mint>,

        /// Treasury state account (PDA).
        #[account(
            mut,
            seeds = [TREASURY_SEED, mint.key().as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Treasury vault (hydration source).
        /// Authority = treasury PDA.
        #[account(
            mut,
            token::mint = mint,
            token::authority = treasury,
            seeds = [TREASURY_SEED, mint.key().as_ref(), b"vault"],
            bump,
        )]
        pub treasury_vault: InterfaceAccount<'info, TokenAccount>,

        /// Swarm hydration PDA vault. Receives tokens for swap to USDC.
        /// Authority = treasury PDA. Must be explicitly initialized via
        /// `create_swarm_vault` before first hydration (S-001 fix:
        /// removed init_if_needed to prevent re-initialization attack).
        #[account(
            mut,
            token::mint = mint,
            token::authority = treasury,
            seeds = [SWARM_HYDRATION_SEED, mint.key().as_ref()],
            bump,
        )]
        pub swarm_vault: InterfaceAccount<'info, TokenAccount>,

        /// Strategy record — MUST be Live to receive funding.
        /// Seeds: [STRATEGY_SEED, treasury.key(), strategy_id]
        #[account(
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        /// Adopter record for beta expiry check. Seeds: ["adopter", token_mint]
        /// If beta_expires_at > 0 and the beta has expired or been ended,
        /// hydrate_swarm is refused.
        #[account(
            seeds = [b"adopter", adopter_record.token_mint.as_ref()],
            bump = adopter_record.bump,
            constraint = adopter_record.treasury == treasury.key() @ TreasuryError::AdopterTreasuryMismatch,
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        /// Authority initiating hydration (anyone can trigger).
        #[account(mut)]
        pub authority: Signer<'info>,

        pub token_program: Interface<'info, TokenInterface>,
        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct EvolvePhase<'info> {
        /// The Token-2022 mint.
        #[account(mint::token_program = token_program)]
        pub mint: InterfaceAccount<'info, Mint>,

        /// Treasury state account (PDA).
        #[account(
            mut,
            seeds = [TREASURY_SEED, mint.key().as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Treasury vault — balance checked against phase caps (C-1 fix).
        /// Authority = treasury PDA.
        #[account(
            token::mint = mint,
            token::authority = treasury,
            seeds = [TREASURY_SEED, mint.key().as_ref(), b"vault"],
            bump,
        )]
        pub treasury_vault: InterfaceAccount<'info, TokenAccount>,

        /// Phase authority — MUST be `treasury.authority`.
        /// Can be a Squads Multisig PDA for governance.
        /// S-002 fix: moved check here as Anchor constraint (single guard,
        /// spec-lock principle) — previously duplicated in handler body.
        #[account(constraint = phase_authority.key() == treasury.authority @ TreasuryError::UnauthorizedPhaseEvolution)]
        pub phase_authority: Signer<'info>,

        pub token_program: Interface<'info, TokenInterface>,
    }

    #[derive(Accounts)]
    pub struct VerifyAdoption<'info> {
        /// The Token-2022 mint — MUST have TransferFeeConfig enabled.
        #[account(mint::token_program = token_program)]
        pub mint: InterfaceAccount<'info, Mint>,

        /// Treasury state account (PDA).
        #[account(
            seeds = [TREASURY_SEED, mint.key().as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        pub token_program: Interface<'info, TokenInterface>,
    }

    #[derive(Accounts)]
    pub struct CreateSwarmVault<'info> {
        /// The Token-2022 mint.
        #[account(mint::token_program = token_program)]
        pub mint: InterfaceAccount<'info, Mint>,

        /// Treasury state account (PDA).
        #[account(
            seeds = [TREASURY_SEED, mint.key().as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Swarm hydration PDA vault. Created exactly once.
        /// Authority = treasury PDA.
        #[account(
            init,
            payer = authority,
            token::mint = mint,
            token::authority = treasury,
            seeds = [SWARM_HYDRATION_SEED, mint.key().as_ref()],
            bump,
        )]
        pub swarm_vault: InterfaceAccount<'info, TokenAccount>,

        /// Authority paying for vault creation (anyone can create).
        #[account(mut)]
        pub authority: Signer<'info>,

        pub token_program: Interface<'info, TokenInterface>,
        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    #[instruction(token_mint: Pubkey)]
    pub struct RegisterAdopter<'info> {
        /// AdopterRecord PDA — one per token mint. Seeds: ["adopter", token_mint]
        #[account(
            init,
            payer = authority,
            space = 8 + AdopterRecord::INIT_SPACE,
            seeds = [b"adopter", token_mint.as_ref()],
            bump,
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        /// The treasury state account (must already be initialised)
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// The authority signing this registration
        #[account(mut)]
        pub authority: Signer<'info>,

        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct RecordFeeDeposit<'info> {
        /// AdopterRecord PDA — seeds: ["adopter", token_mint]
        #[account(
            mut,
            seeds = [b"adopter", adopter_record.token_mint.as_ref()],
            bump = adopter_record.bump,
            constraint = adopter_record.treasury == treasury.key() @ TreasuryError::AdopterTreasuryMismatch,
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        /// Treasury state account — receives the total_fees_received_lamports increment
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// The authority — must equal treasury.authority to prevent arbitrary fee inflation.
        #[account(
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedFeeAttribution,
        )]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    #[instruction(strategy_id: String, promotion_sharpe_x100: i32)]
    pub struct RegisterStrategy<'info> {
        /// Treasury state account (PDA, read-only).
        #[account(
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Strategy record PDA — init, seeds: [STRATEGY_SEED, treasury, strategy_id]
        #[account(
            init,
            payer = authority,
            space = 8 + StrategyRecord::INIT_SPACE,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_id.as_bytes()],
            bump,
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        /// Authority — must equal treasury.authority
        #[account(
            mut,
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedStrategyOp,
        )]
        pub authority: Signer<'info>,

        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct UpdateStrategyPerformance<'info> {
        /// Treasury state account (PDA, read-only).
        #[account(
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Strategy record PDA — mutable, seeds verified.
        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        /// Authority — must equal treasury.authority
        #[account(
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedStrategyOp,
        )]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct ForceRetireStrategy<'info> {
        /// Treasury state account (PDA, read-only, seeds verified).
        #[account(
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Strategy record PDA — mutable, seeds verified.
        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        /// Authority — must equal treasury.authority
        #[account(
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedStrategyOp,
        )]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct EndBeta<'info> {
        /// AdopterRecord PDA — seeds: ["adopter", token_mint]
        #[account(
            mut,
            seeds = [b"adopter", adopter_record.token_mint.as_ref()],
            bump = adopter_record.bump,
            constraint = adopter_record.treasury == treasury.key() @ TreasuryError::AdopterTreasuryMismatch,
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        /// Treasury state account (PDA, read-only, seeds verified).
        #[account(
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Authority — must equal treasury.authority
        #[account(
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedBetaOp,
        )]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct FreezeTreasury<'info> {
        /// Treasury state account (PDA).
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedPhaseEvolution,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Authority — must equal treasury.authority.
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct UnfreezeTreasury<'info> {
        /// Treasury state account (PDA).
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedPhaseEvolution,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Authority — must equal treasury.authority.
        pub authority: Signer<'info>,
    }

    // ==== Flash Trade Account Contexts ======================================

    #[derive(Accounts)]
    pub struct OpenFlashPosition<'info> {
        /// Treasury state account (PDA, mutable for event emission).
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Strategy record — must be Live to open positions.
        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        /// Treasury vault — Token-2022 token account whose token amount denominates
        /// `min_runway_balance` and `input_sol_lamports`. Authority = treasury PDA.
        /// Seeds verified; runway/position-size checks read `.amount`, not `.lamports()`.
        #[account(
            mut,
            token::authority = treasury,
            seeds = [TREASURY_SEED, treasury.mint.as_ref(), b"vault"],
            bump,
        )]
        pub treasury_vault: InterfaceAccount<'info, TokenAccount>,

        /// Fee payer — pays for transaction gas and Flash Trade account rent.
        /// Has NO authority over treasury funds (only pays gas).
        #[account(mut)]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct CloseFlashPosition<'info> {
        /// Treasury state account (PDA, mutable for event emission).
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Strategy record — mutable for position count update.
        /// Close is permitted even if Suspended (exiting is always safe).
        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        /// Fee payer.
        #[account(mut)]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct EmergencyCloseAllPositions<'info> {
        /// Treasury state account (PDA).
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Strategy record — counters reset to zero.
        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        /// Authority — must equal treasury.authority.
        pub authority: Signer<'info>,
    }
}
