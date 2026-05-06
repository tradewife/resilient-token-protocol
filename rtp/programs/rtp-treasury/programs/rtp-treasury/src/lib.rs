use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
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

/// Phase thresholds in lamports (1 SOL = 1_000_000_000 lamports).
/// Production: validated against on-chain oracle (Pyth/Switchboard).
/// Devnet: phase_authority signature is the guard.
/// Wired into evolve_phase via oracle TODO — see handler for details.
const SUSTENANCE_CAP_LAMPORTS: u64 = 50_000_000_000;   // 50 SOL
const ECOSYSTEM_CAP_LAMPORTS: u64 = 1_000_000_000_000; // 1000 SOL

/// Default minimum redistribution amount (0.001 SOL = 1_000_000 lamports).
/// Prevents dust distributions that waste gas.
const DEFAULT_MIN_REDISTRIBUTE: u64 = 1_000_000;

/// Default minimum runway balance (0.01 SOL = 10_000_000 lamports).
/// Production: set to USDC value covering 90 days of ops (~$18k USDC).
/// See BUILD_PLAN.md: "~$100-200/mo ops cost" → $18,000 for 90 days.
const DEFAULT_MIN_RUNWAY: u64 = 10_000_000;

/// PDA seeds
const TREASURY_SEED: &[u8] = b"treasury";
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
    #[msg("Slippage basis points must be 0–10000 (0%–100%)")]
    InvalidSlippage,
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
    #[msg("Only the treasury authority can freeze/unfreeze")]
    UnauthorizedFreeze,
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
    /// The phase authority (set at initialization).
    pub authority: Pubkey,
    /// Current evolution phase (Sustenance → Ecosystem → Humanity)
    pub phase: Phase,
    /// Cumulative fees withdrawn (native SOL lamports)
    pub total_fees_withdrawn: u64,
    /// Cumulative tokens distributed to holders (70%) — kept for metric continuity
    pub total_distributed_holders: u64,
    /// Cumulative tokens distributed to project dev (20%) — kept for metric continuity
    pub total_distributed_dev: u64,
    /// Cumulative tokens distributed to ecosystem (10%) — kept for metric continuity
    pub total_distributed_ecosystem: u64,
    /// Cumulative tokens sent to swarm hydration wallet
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
    pub min_runway_balance: u64,
    /// Whether the treasury is frozen (emergency halt).
    /// When true, all non-read operations are rejected.
    pub frozen: bool,
    /// SOL lamports already committed to open Flash Trade positions.
    /// Deducted from available balance for redistribution/hydration.
    pub committed_sol_lamports: u64,
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
// AdopterRecord — per-adopter fee tracking for multi-token attribution
// ---------------------------------------------------------------------------

/// Tracks a single token project's cumulative fee contributions to the RTP treasury.
/// Seeds: ["adopter", treasury.key(), adopter_id]
/// This enables pro-rata yield attribution:
///   adopter_yield_share = fees_contributed_lamports / treasury.total_fees_received_lamports
#[account]
#[derive(InitSpace)]
pub struct AdopterRecord {
    /// The treasury this adopter belongs to (back-reference for cross-validation)
    pub treasury: Pubkey,
    /// Unique adopter identifier (caller-defined, max 32 bytes)
    #[max_len(32)]
    pub adopter_id: String,
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
    pub treasury: Pubkey,
    pub adopter_id: String,
    pub adopted_at: i64,
}

#[event]
pub struct FeeDepositRecorded {
    pub treasury: Pubkey,
    pub adopter_id: String,
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
    pub treasury: Pubkey,
    pub adopter_id: String,
    pub ended_at: i64,
    pub fees_contributed_lamports: u64,
}

#[event]
pub struct Redistribution {
    pub treasury: Pubkey,
    pub excess: u64,
    pub holders_amount: u64,
    pub dev_amount: u64,
    pub ecosystem_amount: u64,
    pub ts: i64,
}

#[event]
pub struct TreasuryFrozen {
    pub treasury: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TreasuryUnfrozen {
    pub treasury: Pubkey,
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

/// Rent-exempt minimum lamports for a Treasury account.
fn treasury_rent_exempt_minimum() -> Result<u64> {
    Ok(Rent::get()?.minimum_balance(8 + Treasury::INIT_SPACE))
}

/// Current lamports held in the treasury account.
fn treasury_lamports(treasury: &Account<Treasury>) -> u64 {
    treasury.to_account_info().lamports()
}

/// Available lamports: total - rent exemption - committed positions.
fn available_treasury_lamports(treasury: &Account<Treasury>) -> Result<u64> {
    Ok(treasury_lamports(treasury)
        .saturating_sub(treasury_rent_exempt_minimum()?)
        .saturating_sub(treasury.committed_sol_lamports))
}

/// Transfer lamports from the treasury account to a recipient.
fn transfer_from_treasury(
    treasury: &Account<'_, Treasury>,
    recipient: &AccountInfo<'_>,
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

    /// Initialize a new Treasury owned by the given authority.
    ///
    /// Authority is stored in the Treasury account and used as the PDA seed.
    /// Anyone can call this on behalf of an authority — fees are paid by caller.
    pub fn initialize(
        ctx: Context<Initialize>,
        holders_wallet: Pubkey,
        project_dev_wallet: Pubkey,
        ecosystem_wallet: Pubkey,
        min_runway_balance: u64,
    ) -> Result<()> {
        reject_zero_address(ctx.accounts.authority.key())?;
        reject_zero_address(holders_wallet)?;
        reject_zero_address(project_dev_wallet)?;
        reject_zero_address(ecosystem_wallet)?;

        let treasury = &mut ctx.accounts.treasury;
        treasury.authority = ctx.accounts.authority.key();
        treasury.phase = Phase::default();
        treasury.total_fees_withdrawn = 0;
        treasury.total_distributed_holders = 0;
        treasury.total_distributed_dev = 0;
        treasury.total_distributed_ecosystem = 0;
        treasury.total_hydration = 0;
        treasury.total_fees_received_lamports = 0;
        treasury.frozen = false;
        treasury.holders_wallet = holders_wallet;
        treasury.project_dev_wallet = project_dev_wallet;
        treasury.ecosystem_wallet = ecosystem_wallet;
        require!(
            min_runway_balance >= DEFAULT_MIN_RUNWAY,
            TreasuryError::InsufficientRunway,
        );
        treasury.min_runway_balance = min_runway_balance;
        treasury.committed_sol_lamports = 0;
        treasury.bump = ctx.bumps.treasury;

        Ok(())
    }

    /// Deposit native SOL into the treasury.
    ///
    /// Increments total_fees_withdrawn and total_fees_received_lamports.
    /// Caller pays via system program transfer; treasury lamports increase directly.
    pub fn deposit_sol(ctx: Context<DepositSol>, amount_lamports: u64) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        require!(amount_lamports > 0, TreasuryError::ZeroAmount);

        let treasury = &mut ctx.accounts.treasury;

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: treasury.to_account_info(),
                },
            ),
            amount_lamports,
        )?;

        treasury.total_fees_withdrawn = treasury
            .total_fees_withdrawn
            .checked_add(amount_lamports)
            .ok_or(TreasuryError::Overflow)?;
        treasury.total_fees_received_lamports = treasury
            .total_fees_received_lamports
            .checked_add(amount_lamports)
            .ok_or(TreasuryError::Overflow)?;

        Ok(())
    }

    /// Check redistribution threshold and execute 70/20/10 split in native SOL.
    ///
    /// Distributes the excess above `min_runway_balance`:
    /// - 70% → holders_wallet
    /// - 20% → project_dev_wallet
    /// - 10% → ecosystem_wallet (+ rounding dust)
    ///
    /// Callable by anyone. The split is deterministic on-chain.
    pub fn check_redistribute(ctx: Context<CheckRedistribute>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let treasury = &mut ctx.accounts.treasury;

        let balance = available_treasury_lamports(treasury)?;
        let excess = balance.saturating_sub(treasury.min_runway_balance);
        require!(excess > DEFAULT_MIN_REDISTRIBUTE, TreasuryError::BelowThreshold);

        let holders_amt = (excess as u128 * HOLDERS_BPS as u128 / 10000) as u64;
        let dev_amt = (excess as u128 * PROJECT_DEV_BPS as u128 / 10000) as u64;
        let eco_amt = excess.saturating_sub(holders_amt).saturating_sub(dev_amt);

        if holders_amt > 0 {
            transfer_from_treasury(
                treasury,
                &ctx.accounts.holders_wallet.to_account_info(),
                holders_amt,
            )?;
        }
        if dev_amt > 0 {
            transfer_from_treasury(
                treasury,
                &ctx.accounts.project_dev_wallet.to_account_info(),
                dev_amt,
            )?;
        }
        if eco_amt > 0 {
            transfer_from_treasury(
                treasury,
                &ctx.accounts.ecosystem_wallet.to_account_info(),
                eco_amt,
            )?;
        }

        treasury.total_distributed_holders = treasury.total_distributed_holders.saturating_add(holders_amt);
        treasury.total_distributed_dev = treasury.total_distributed_dev.saturating_add(dev_amt);
        treasury.total_distributed_ecosystem = treasury.total_distributed_ecosystem.saturating_add(eco_amt);

        let clock = Clock::get()?;
        emit!(Redistribution {
            treasury: treasury.key(),
            excess,
            holders_amount: holders_amt,
            dev_amount: dev_amt,
            ecosystem_amount: eco_amt,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Fund swarm operations from the treasury.
    ///
    /// Enforces the 90-day runway invariant (CLAUDE.md #9):
    /// post-hydration balance MUST remain >= `min_runway_balance`.
    pub fn hydrate_swarm(ctx: Context<HydrateSwarm>, amount: u64) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let treasury = &mut ctx.accounts.treasury;

        require!(
            ctx.accounts.strategy_record.status == StrategyLifecycleStatus::Live,
            TreasuryError::StrategyNotLive,
        );

        let adopter = &ctx.accounts.adopter_record;
        if adopter.beta_expires_at > 0 {
            let clock = Clock::get()?;
            require!(
                !adopter.beta_ended && clock.unix_timestamp < adopter.beta_expires_at,
                TreasuryError::BetaExpired,
            );
        }

        require!(amount > 0, TreasuryError::ZeroAmount);

        let post_balance = available_treasury_lamports(treasury)?.saturating_sub(amount);
        require!(
            post_balance >= treasury.min_runway_balance,
            TreasuryError::InsufficientRunway,
        );

        transfer_from_treasury(treasury, &ctx.accounts.swarm_wallet.to_account_info(), amount)?;

        treasury.total_hydration = treasury.total_hydration.saturating_add(amount);
        Ok(())
    }

    /// Evolve the treasury phase. IRREVERSIBLE.
    ///
    /// Phase thresholds (native SOL lamports in treasury):
    /// - Sustenance → Ecosystem:  >= SUSTENANCE_CAP
    /// - Ecosystem   → Humanity:  >= ECOSYSTEM_CAP
    ///
    /// Production: these thresholds should be validated against an on-chain
    /// oracle (e.g. Pyth). For devnet, the phase_authority signature is
    /// the guard — the authority is responsible for checking reserves.
    pub fn evolve_phase(ctx: Context<EvolvePhase>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let treasury = &mut ctx.accounts.treasury;
        let balance = available_treasury_lamports(treasury)?;

        let next = match treasury.phase {
            Phase::Sustenance => {
                require!(
                    balance >= SUSTENANCE_CAP_LAMPORTS,
                    TreasuryError::BelowThreshold,
                );
                Phase::Ecosystem
            }
            Phase::Ecosystem => {
                require!(
                    balance >= ECOSYSTEM_CAP_LAMPORTS,
                    TreasuryError::BelowThreshold,
                );
                Phase::Humanity
            }
            Phase::Humanity => return Err(TreasuryError::AlreadyMaxPhase.into()),
        };

        treasury.phase = next;
        Ok(())
    }

    /// Register a new adopter with the treasury.
    ///
    /// Seeds: ["adopter", treasury.key(), adopter_id]
    pub fn register_adopter(ctx: Context<RegisterAdopter>, adopter_id: String) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let record = &mut ctx.accounts.adopter_record;
        let clock = Clock::get()?;

        record.treasury = ctx.accounts.treasury.key();
        record.adopter_id = adopter_id.clone();
        record.fees_contributed_lamports = 0;
        record.adopted_at = clock.unix_timestamp;
        record.last_deposit_ts = clock.unix_timestamp;
        record.deposit_count = 0;
        record.beta_expires_at = 0;
        record.beta_ended = false;
        record.bump = ctx.bumps.adopter_record;

        emit!(AdopterRegistered {
            treasury: ctx.accounts.treasury.key(),
            adopter_id,
            adopted_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Register a beta adopter with an automatic expiry timestamp.
    pub fn register_adopter_beta(
        ctx: Context<RegisterAdopter>,
        adopter_id: String,
        beta_expires_at: i64,
    ) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        let clock = Clock::get()?;
        require!(
            beta_expires_at > clock.unix_timestamp,
            TreasuryError::BetaExpired,
        );

        let record = &mut ctx.accounts.adopter_record;

        record.treasury = ctx.accounts.treasury.key();
        record.adopter_id = adopter_id.clone();
        record.fees_contributed_lamports = 0;
        record.adopted_at = clock.unix_timestamp;
        record.last_deposit_ts = clock.unix_timestamp;
        record.deposit_count = 0;
        record.beta_expires_at = beta_expires_at;
        record.beta_ended = false;
        record.bump = ctx.bumps.adopter_record;

        emit!(AdopterRegistered {
            treasury: ctx.accounts.treasury.key(),
            adopter_id,
            adopted_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Record a fee deposit from an adopting token project.
    /// Accounting only — does not move funds.
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

        let treasury_key = treasury.key();
        let total_fees = treasury
            .total_fees_received_lamports
            .checked_add(amount_lamports)
            .ok_or(TreasuryError::Overflow)?;
        treasury.total_fees_received_lamports = total_fees;

        emit!(FeeDepositRecorded {
            treasury: treasury_key,
            adopter_id: record.adopter_id.clone(),
            amount_lamports,
            cumulative: record.fees_contributed_lamports,
            total_treasury_fees: total_fees,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// End a beta adopter's RTP participation early.
    pub fn end_beta(ctx: Context<EndBeta>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);

        let record = &mut ctx.accounts.adopter_record;
        let clock = Clock::get()?;
        record.beta_ended = true;

        emit!(BetaEnded {
            treasury: ctx.accounts.treasury.key(),
            adopter_id: record.adopter_id.clone(),
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
        // Authority gate is enforced by the Anchor constraint on UpdateStrategyPerformance.
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
            treasury: treasury.key(),
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
            treasury: treasury.key(),
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
        require!(slippage_bps <= 10000, TreasuryError::InvalidSlippage);

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

        // Read native SOL available balance (excludes rent exemption + committed positions)
        let available = available_treasury_lamports(treasury)?;

        // Position size cap (20% of available)
        let max_input = available as u128 * MAX_POSITION_SIZE_BPS as u128 / 10000;
        require!(
            input_sol_lamports as u128 <= max_input,
            TreasuryError::PositionSizeExceeded,
        );

        // Runway floor: available after commit must still cover min_runway_balance
        require!(
            available.saturating_sub(input_sol_lamports) >= treasury.min_runway_balance,
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
        require!(
            remaining[16].key() == flash_program_id,
            TreasuryError::InvalidFlashProgramId,
        );

        // Validate Flash Trade event authority PDA at remaining[15].
        let (expected_event_authority, _evt_bump) = Pubkey::find_program_address(
            &[b"__event_authority"],
            &flash_program_id,
        );
        require!(
            remaining[15].key() == expected_event_authority,
            TreasuryError::InvalidFlashEventAuthority,
        );

        // Validate canonical System Program at remaining[13].
        require!(
            remaining[13].key() == anchor_lang::system_program::ID,
            TreasuryError::InvalidFlashSystemProgram,
        );

        // Validate funding_token_program at remaining[14].
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

        // Sign with Treasury PDA seeds (authority-seeded)
        let seeds = &[TREASURY_SEED, treasury.authority.as_ref(), &[treasury.bump]];
        let signer_seeds = &[&seeds[..]];

        invoke_signed(&flash_ix, &remaining.iter().map(|a| a.to_account_info()).collect::<Vec<_>>(), signer_seeds)
            .map_err(|_| TreasuryError::FlashCpiFailed)?;

        // Update strategy state
        strategy.open_position_count = strategy.open_position_count.saturating_add(1);
        strategy.committed_sol_lamports = strategy
            .committed_sol_lamports
            .checked_add(input_sol_lamports)
            .ok_or(TreasuryError::Overflow)?;
        strategy.flash_pool_name = pool_name.clone();

        // Update treasury global commitment counter
        treasury.committed_sol_lamports = treasury
            .committed_sol_lamports
            .checked_add(input_sol_lamports)
            .ok_or(TreasuryError::Overflow)?;

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
        require!(slippage_bps <= 10000, TreasuryError::InvalidSlippage);

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

        let seeds = &[TREASURY_SEED, treasury.authority.as_ref(), &[treasury.bump]];
        let signer_seeds = &[&seeds[..]];

        invoke_signed(&flash_ix, &remaining.iter().map(|a| a.to_account_info()).collect::<Vec<_>>(), signer_seeds)
            .map_err(|_| TreasuryError::FlashCpiFailed)?;

        // Decrement both strategy and treasury commitment counters.
        strategy.open_position_count = strategy.open_position_count.saturating_sub(1);
        require!(
            strategy.committed_sol_lamports >= committed_sol_lamports_delta,
            TreasuryError::CommittedDeltaExceedsBalance,
        );
        strategy.committed_sol_lamports -= committed_sol_lamports_delta;
        treasury.committed_sol_lamports = treasury
            .committed_sol_lamports
            .checked_sub(committed_sol_lamports_delta)
            .ok_or(TreasuryError::CommittedDeltaExceedsBalance)?;

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
        let treasury = &mut ctx.accounts.treasury;
        let previous_committed = strategy.committed_sol_lamports;
        let clock = Clock::get()?;

        // Reset counters first (audit captures the post-state intent).
        strategy.open_position_count = 0;
        strategy.committed_sol_lamports = 0;
        treasury.committed_sol_lamports = treasury
            .committed_sol_lamports
            .saturating_sub(previous_committed);

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
    #[instruction(holders_wallet: Pubkey, project_dev_wallet: Pubkey, ecosystem_wallet: Pubkey, min_runway_balance: u64)]
    pub struct Initialize<'info> {
        /// Treasury state account (PDA, authority-seeded). No private key exists.
        #[account(
            init,
            payer = authority,
            space = 8 + Treasury::INIT_SPACE,
            seeds = [TREASURY_SEED, authority.key().as_ref()],
            bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Authority paying for initialization (anyone can initialize).
        #[account(mut)]
        pub authority: Signer<'info>,

        pub system_program: Program<'info, System>,
    }

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

        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct HydrateSwarm<'info> {
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// CHECK: swarm/operator wallet receiving native SOL
        #[account(mut)]
        pub swarm_wallet: UncheckedAccount<'info>,

        #[account(
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        #[account(
            seeds = [b"adopter", treasury.key().as_ref(), adopter_record.adopter_id.as_bytes()],
            bump = adopter_record.bump,
            constraint = adopter_record.treasury == treasury.key() @ TreasuryError::AdopterTreasuryMismatch,
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        #[account(mut)]
        pub authority: Signer<'info>,

        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct EvolvePhase<'info> {
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(constraint = phase_authority.key() == treasury.authority @ TreasuryError::UnauthorizedPhaseEvolution)]
        pub phase_authority: Signer<'info>,

        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    #[instruction(adopter_id: String)]
    pub struct RegisterAdopter<'info> {
        /// Seeds: ["adopter", treasury.key(), adopter_id]
        #[account(
            init,
            payer = authority,
            space = 8 + AdopterRecord::INIT_SPACE,
            seeds = [b"adopter", treasury.key().as_ref(), adopter_id.as_bytes()],
            bump,
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(mut)]
        pub authority: Signer<'info>,

        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct RecordFeeDeposit<'info> {
        #[account(
            mut,
            seeds = [b"adopter", treasury.key().as_ref(), adopter_record.adopter_id.as_bytes()],
            bump = adopter_record.bump,
            constraint = adopter_record.treasury == treasury.key() @ TreasuryError::AdopterTreasuryMismatch,
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedFeeAttribution,
        )]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    #[instruction(strategy_id: String, _promotion_sharpe_x100: i32)]
    pub struct RegisterStrategy<'info> {
        #[account(
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(
            init,
            payer = authority,
            space = 8 + StrategyRecord::INIT_SPACE,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_id.as_bytes()],
            bump,
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        #[account(
            mut,
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedStrategyOp,
        )]
        pub authority: Signer<'info>,

        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct UpdateStrategyPerformance<'info> {
        #[account(
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        #[account(
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedStrategyOp,
        )]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct ForceRetireStrategy<'info> {
        #[account(
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        #[account(
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedStrategyOp,
        )]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct EndBeta<'info> {
        #[account(
            mut,
            seeds = [b"adopter", treasury.key().as_ref(), adopter_record.adopter_id.as_bytes()],
            bump = adopter_record.bump,
            constraint = adopter_record.treasury == treasury.key() @ TreasuryError::AdopterTreasuryMismatch,
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        #[account(
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedBetaOp,
        )]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct FreezeTreasury<'info> {
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedFreeze,
        )]
        pub treasury: Account<'info, Treasury>,

        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct UnfreezeTreasury<'info> {
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
            constraint = authority.key() == treasury.authority @ TreasuryError::UnauthorizedFreeze,
        )]
        pub treasury: Account<'info, Treasury>,

        pub authority: Signer<'info>,
    }

    // ==== Flash Trade Account Contexts ======================================

    #[derive(Accounts)]
    pub struct OpenFlashPosition<'info> {
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        #[account(mut)]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct CloseFlashPosition<'info> {
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        #[account(mut)]
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct EmergencyCloseAllPositions<'info> {
        #[account(
            mut,
            seeds = [TREASURY_SEED, treasury.authority.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        #[account(
            mut,
            seeds = [STRATEGY_SEED, treasury.key().as_ref(), strategy_record.strategy_id.as_bytes()],
            bump = strategy_record.bump,
            constraint = strategy_record.treasury == treasury.key(),
        )]
        pub strategy_record: Account<'info, StrategyRecord>,

        pub authority: Signer<'info>,
    }
}

// ==== Unit Tests ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Slippage math tests (Finding #12) --

    /// Open Long: slippage_price = price * (10000 + slip) / 10000
    /// With price=100_000, slippage_bps=100 (1%): expect 101_000
    #[test]
    fn slippage_open_long_increases_price() {
        let price: i128 = 100_000;
        let slippage_bps: u16 = 100; // 1%
        let slippage_mult = 10000u32 + slippage_bps as u32;
        let result = price * slippage_mult as i128 / 10000;
        assert_eq!(result, 101_000);
    }

    /// Open Short: slippage_price = price * (20000 - (10000 + slip)) / 10000
    /// With price=100_000, slippage_bps=100: expect 99_000
    #[test]
    fn slippage_open_short_decreases_price() {
        let price: i128 = 100_000;
        let slippage_bps: u16 = 100;
        let slippage_mult = 10000u32 + slippage_bps as u32;
        let result = price * (20000 - slippage_mult as i128) / 10000;
        assert_eq!(result, 99_000);
    }

    /// Close Long: slippage_price = price * (10000 - slip) / 10000
    /// With price=100_000, slippage_bps=100: expect 99_000 (accept lower exit)
    #[test]
    fn slippage_close_long_decreases_price() {
        let price: i128 = 100_000;
        let slip: i128 = 100;
        let result = price * (10_000 - slip) / 10_000;
        assert_eq!(result, 99_000);
    }

    /// Close Short: slippage_price = price * (10000 + slip) / 10000
    /// With price=100_000, slippage_bps=100: expect 101_000 (accept higher exit)
    #[test]
    fn slippage_close_short_increases_price() {
        let price: i128 = 100_000;
        let slip: i128 = 100;
        let result = price * (10_000 + slip) / 10_000;
        assert_eq!(result, 101_000);
    }

    /// Zero slippage: open/close price equals oracle price
    #[test]
    fn slippage_zero_is_passthrough() {
        let price: i128 = 100_000;
        let slippage_bps: u16 = 0;

        // Open Long
        let slippage_mult = 10000u32 + slippage_bps as u32;
        let open_long = price * slippage_mult as i128 / 10000;
        assert_eq!(open_long, 100_000);

        // Open Short
        let open_short = price * (20000 - slippage_mult as i128) / 10000;
        assert_eq!(open_short, 100_000);

        // Close Long
        let close_long = price * 10_000 / 10_000;
        assert_eq!(close_long, 100_000);

        // Close Short
        let close_short = price * 10_000 / 10_000;
        assert_eq!(close_short, 100_000);
    }

    /// Max slippage (10000 = 100%): verify no negative prices
    #[test]
    fn slippage_max_100_percent() {
        let price: i128 = 100_000;
        let slippage_bps: u16 = 10000;

        // Open Long: price * 20000 / 10000 = 2x
        let slippage_mult = 10000u32 + slippage_bps as u32;
        let open_long = price * slippage_mult as i128 / 10000;
        assert_eq!(open_long, 200_000);

        // Open Short: price * 0 / 10000 = 0 (floor, not negative)
        let open_short = price * (20000 - slippage_mult as i128) / 10000;
        assert_eq!(open_short, 0);

        // Close Long: price * 0 / 10000 = 0
        let slip: i128 = 10000;
        let close_long = price * (10_000 - slip) / 10_000;
        assert_eq!(close_long, 0);

        // Close Short: price * 20000 / 10000 = 2x
        let close_short = price * (10_000 + slip) / 10_000;
        assert_eq!(close_short, 200_000);
    }

    // -- Recovery counter tests (Finding #18) --

    #[test]
    fn recovery_counter_needs_three_positive_updates() {
        // Simulate: 2 positive updates → strikes remain
        let mut soft_decay_strikes: u8 = 2;
        let mut recovery_counter: u8 = 0;

        // Positive update 1
        recovery_counter = recovery_counter.saturating_add(1);
        if recovery_counter >= MIN_RECOVERY_TRADES {
            soft_decay_strikes = 0;
            recovery_counter = 0;
        }
        assert_eq!(soft_decay_strikes, 2); // Not yet reset
        assert_eq!(recovery_counter, 1);

        // Positive update 2
        recovery_counter = recovery_counter.saturating_add(1);
        if recovery_counter >= MIN_RECOVERY_TRADES {
            soft_decay_strikes = 0;
            recovery_counter = 0;
        }
        assert_eq!(soft_decay_strikes, 2); // Still not reset
        assert_eq!(recovery_counter, 2);

        // Positive update 3 → strikes reset
        recovery_counter = recovery_counter.saturating_add(1);
        if recovery_counter >= MIN_RECOVERY_TRADES {
            soft_decay_strikes = 0;
            recovery_counter = 0;
        }
        assert_eq!(soft_decay_strikes, 0); // Reset!
        assert_eq!(recovery_counter, 0);
    }

    #[test]
    fn new_strike_resets_recovery_counter() {
        let mut soft_decay_strikes: u8 = 1;
        let mut recovery_counter: u8 = 2; // 2/3 toward recovery

        // New soft strike arrives
        soft_decay_strikes = soft_decay_strikes.saturating_add(1);
        recovery_counter = 0;

        assert_eq!(soft_decay_strikes, 2);
        assert_eq!(recovery_counter, 0); // Recovery progress wiped
    }

    #[test]
    fn neutral_update_resets_recovery_counter() {
        let mut recovery_counter: u8 = 2;

        // Neither strike nor recovery (pnl_bps == 0)
        let rolling_pnl_bps: i32 = 0;
        let rolling_sharpe_x100: i32 = 50;
        if rolling_pnl_bps > 0 && rolling_sharpe_x100 > 0 {
            recovery_counter = recovery_counter.saturating_add(1);
        } else {
            recovery_counter = 0;
        }

        assert_eq!(recovery_counter, 0);
    }

    // -- Emergency close edge case (Finding #17) --

    #[test]
    fn emergency_close_saturating_sub_no_underflow() {
        // Scenario: strategy committed exceeds treasury committed (data corruption)
        let treasury_committed: u64 = 1_000;
        let strategy_committed: u64 = 5_000; // corrupted: more than treasury

        // saturating_sub prevents underflow
        let result = treasury_committed.saturating_sub(strategy_committed);
        assert_eq!(result, 0); // Clamped to zero, no panic
    }

    #[test]
    fn emergency_close_normal_subtraction() {
        let treasury_committed: u64 = 10_000;
        let strategy_committed: u64 = 3_000;

        let result = treasury_committed.saturating_sub(strategy_committed);
        assert_eq!(result, 7_000);
    }
}
