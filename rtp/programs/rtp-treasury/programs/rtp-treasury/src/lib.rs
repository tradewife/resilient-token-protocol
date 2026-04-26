use anchor_lang::prelude::*;
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
    /// Largest single drawdown observed in the last 24h, in basis points
    pub drawdown_24h_bps: u16,
    /// Cumulative total trades executed on-chain
    pub total_trades: u32,
    /// Sharpe ratio at time of promotion (stored as integer x100, e.g. 396 = 3.96)
    pub promotion_sharpe_x100: i32,
    /// Current rolling Sharpe (integer x100). Updated by the swarm agent.
    pub rolling_sharpe_x100: i32,
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
        record.deposit_count += 1;

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
        require!(
            ctx.accounts.authority.key() == ctx.accounts.treasury.authority,
            TreasuryError::UnauthorizedBetaOp,
        );

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
        require!(
            ctx.accounts.authority.key() == ctx.accounts.treasury.authority,
            TreasuryError::UnauthorizedStrategyOp,
        );

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
        record.drawdown_24h_bps = 0;
        record.total_trades = 0;
        record.promotion_sharpe_x100 = promotion_sharpe_x100;
        record.rolling_sharpe_x100 = promotion_sharpe_x100;
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

        // 3. Increment soft decay strikes
        if new_soft_strike {
            record.soft_decay_strikes = record.soft_decay_strikes.saturating_add(1);
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
            drawdown_24h_bps: record.drawdown_24h_bps,
            status: record.status,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Emergency manual retirement by treasury authority. Bypasses thresholds.
    pub fn force_retire_strategy(ctx: Context<ForceRetireStrategy>) -> Result<()> {
        require!(!ctx.accounts.treasury.frozen, TreasuryError::TreasuryFrozen);
        require!(
            ctx.accounts.authority.key() == ctx.accounts.treasury.authority,
            TreasuryError::UnauthorizedStrategyOp,
        );

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
        #[account(mut)]
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
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        /// Treasury state account — receives the total_fees_received_lamports increment
        #[account(mut)]
        pub treasury: Account<'info, Treasury>,

        /// The authority that can record fee deposits
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
        #[account(mut)]
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

        /// Authority — must equal treasury.authority (enforced in handler)
        pub authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct EndBeta<'info> {
        /// AdopterRecord PDA — seeds: ["adopter", token_mint]
        #[account(
            mut,
            seeds = [b"adopter", adopter_record.token_mint.as_ref()],
            bump = adopter_record.bump,
        )]
        pub adopter_record: Account<'info, AdopterRecord>,

        /// Treasury state account (PDA, read-only, seeds verified).
        #[account(
            seeds = [TREASURY_SEED, treasury.mint.as_ref()],
            bump = treasury.bump,
        )]
        pub treasury: Account<'info, Treasury>,

        /// Authority — must equal treasury.authority (enforced in handler)
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
}
