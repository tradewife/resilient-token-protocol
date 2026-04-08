use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use spl_token_2022_interface::{
    extension::{transfer_fee::TransferFeeConfig, BaseStateWithExtensions, StateWithExtensions},
    state::Mint as SplMint,
};

declare_id!("4LvsHbe9LLwgogcDbH7ieTsGcWZctjYFZkzZwaHDM8Ad");

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
    /// PDA bump
    pub bump: u8,
}

/// Treasury phase — can only advance forward. Transitions are IRREVERSIBLE.
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
        let treasury = &mut ctx.accounts.treasury;
        treasury.mint = ctx.accounts.mint.key();
        treasury.authority = ctx.accounts.authority.key();
        treasury.phase = Phase::default();
        treasury.total_fees_withdrawn = 0;
        treasury.total_distributed_holders = 0;
        treasury.total_distributed_dev = 0;
        treasury.total_distributed_ecosystem = 0;
        treasury.total_hydration = 0;
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
        Ok(())
    }

    /// Fund swarm operations from the treasury vault.
    ///
    /// Enforces the 90-day runway invariant (CLAUDE.md #9):
    /// post-hydration balance MUST remain >= `min_runway_balance`.
    /// Transfers tokens to the swarm hydration PDA for swap to USDC.
    pub fn hydrate_swarm(ctx: Context<HydrateSwarm>, amount: u64) -> Result<()> {
        let treasury = &mut ctx.accounts.treasury;
        let vault = &ctx.accounts.treasury_vault;

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
    pub fn create_swarm_vault(_ctx: Context<CreateSwarmVault>) -> Result<()> {
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
}
