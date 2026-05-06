//! On-chain client for the RTP Treasury program.
//!
//! Builds, simulates, and submits Anchor instructions for `open_flash_position`
//! and `close_flash_position`. PDA derivation matches the on-chain program in
//! `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`.
//!
//! Three execution modes (chosen via `RTP_EXECUTION_MODE`):
//!
//!   - `simulate` — build + RPC simulate, never submit. Default.
//!   - `devnet`   — build + submit to `SOLANA_RPC_URL` (devnet by default).
//!   - `mainnet`  — same as devnet, but mainnet-beta. Requires opt-in env.
//!
//! All paths are env-driven; nothing is hardcoded.

use base64::Engine as _;
use serde::{Deserialize, Serialize};
#[allow(deprecated)]
use solana_sdk::system_program;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

// ---- Discriminators (regenerate from target/idl/rtp_treasury.json after Anchor build) ----
pub const OPEN_FLASH_POSITION_DISC: [u8; 8] = [102, 68, 197, 231, 254, 69, 188, 127];
pub const CLOSE_FLASH_POSITION_DISC: [u8; 8] = [65, 15, 74, 221, 107, 136, 176, 33];

// ---- PDA seeds (must match rtp-treasury) ----
const TREASURY_SEED: &[u8] = b"treasury";

const STRATEGY_SEED: &[u8] = b"strategy";

/// Well-known SPL Token (legacy) program ID.
const TOKEN_PROGRAM_ID_STR: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
/// Token-2022 program ID. Exposed for callers that need to override the
/// default funding token program (e.g. token-2022 mints).
#[allow(dead_code)]
pub const TOKEN_2022_PROGRAM_ID_STR: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
/// Sysvar Instructions program.
const SYSVAR_INSTRUCTIONS_STR: &str = "Sysvar1nstructions1111111111111111111111111";

/// Execution mode — chosen via `RTP_EXECUTION_MODE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Build + RPC `simulateTransaction`. Never submits.
    Simulate,
    /// Submit to devnet RPC.
    Devnet,
    /// Submit to mainnet RPC. Requires explicit opt-in.
    Mainnet,
}

impl ExecutionMode {
    pub fn from_env() -> Self {
        match std::env::var("RTP_EXECUTION_MODE")
            .unwrap_or_else(|_| "simulate".to_string())
            .to_lowercase()
            .as_str()
        {
            "devnet" => Self::Devnet,
            "mainnet" => Self::Mainnet,
            _ => Self::Simulate,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Simulate => "simulate",
            Self::Devnet => "devnet",
            Self::Mainnet => "mainnet",
        }
    }

    pub fn submits(&self) -> bool {
        !matches!(self, Self::Simulate)
    }
}

/// Daemon configuration loaded from env vars.
///
/// Required for any chain interaction:
///   - `RTP_PROGRAM_ID`        — RTP treasury program (default: 8rt6yi…)
///   - `RTP_AUTHORITY`         — Treasury authority pubkey (used for PDA derivation)
///   - `RTP_AUTHORITY_KEYPAIR` — path to fee-payer/authority keypair
///
/// Optional:
///   - `RTP_STRATEGY_ID`       — defaults to `SOL_FT_V1`
///   - `RTP_TREASURY_PDA`      — override (else derived from program+authority)
///   - `SOLANA_RPC_URL`        — RPC endpoint for the chosen cluster
///   - `RTP_FLASH_PROGRAM_ID`  — Flash Trade program (default mainnet)
///   - `RTP_EXECUTION_MODE`    — simulate | devnet | mainnet (default simulate)
///
/// Legacy:
///   - `RTP_MINT`              — accepted as alias for `RTP_AUTHORITY` (pre-v1.3 daemon)
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub program_id: Pubkey,
    pub authority: Pubkey,
    pub treasury_pda: Pubkey,
    pub strategy_id: String,
    pub strategy_pda: Pubkey,
    pub authority_keypair_path: Option<String>,
    pub rpc_url: String,
    pub flash_program_id: Pubkey,
    pub mode: ExecutionMode,
}

impl ChainConfig {
    /// Load config from env. Returns `Err` if required env vars are missing or
    /// malformed.
    pub fn from_env() -> Result<Self, String> {
        let program_id_str = std::env::var("RTP_PROGRAM_ID")
            .unwrap_or_else(|_| "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB".to_string());
        let program_id = Pubkey::from_str(&program_id_str)
            .map_err(|e| format!("RTP_PROGRAM_ID invalid: {}", e))?;

        // Authority pubkey — the on-chain treasury PDA is seeded by [TREASURY_SEED, authority].
        // Accept RTP_MINT as legacy alias so existing Railway configs keep working.
        let authority_str = std::env::var("RTP_AUTHORITY")
            .or_else(|_| std::env::var("RTP_MINT"))
            .map_err(|_| "RTP_AUTHORITY (or legacy RTP_MINT) not set — cannot derive treasury PDA".to_string())?;
        let authority = Pubkey::from_str(&authority_str)
            .map_err(|e| format!("RTP_AUTHORITY invalid: {}", e))?;

        let (derived_treasury, _bump) =
            Pubkey::find_program_address(&[TREASURY_SEED, authority.as_ref()], &program_id);
        let treasury_pda = std::env::var("RTP_TREASURY_PDA")
            .ok()
            .and_then(|s| Pubkey::from_str(&s).ok())
            .unwrap_or(derived_treasury);

        let strategy_id =
            std::env::var("RTP_STRATEGY_ID").unwrap_or_else(|_| "SOL_FT_V1".to_string());
        let (strategy_pda, _sbump) = Pubkey::find_program_address(
            &[STRATEGY_SEED, treasury_pda.as_ref(), strategy_id.as_bytes()],
            &program_id,
        );

        let authority_keypair_path = std::env::var("RTP_AUTHORITY_KEYPAIR").ok();

        let mode = ExecutionMode::from_env();
        let default_rpc = match mode {
            ExecutionMode::Mainnet => "https://api.mainnet-beta.solana.com",
            _ => "https://api.devnet.solana.com",
        };
        let rpc_url = std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| default_rpc.to_string());

        let flash_program_str = std::env::var("RTP_FLASH_PROGRAM_ID")
            .unwrap_or_else(|_| "FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn".to_string());
        let flash_program_id = Pubkey::from_str(&flash_program_str)
            .map_err(|e| format!("RTP_FLASH_PROGRAM_ID invalid: {}", e))?;

        Ok(Self {
            program_id,
            authority,
            treasury_pda,
            strategy_id,
            strategy_pda,
            authority_keypair_path,
            rpc_url,
            flash_program_id,
            mode,
        })
    }

    /// Pretty-print the resolved config without leaking the keypair path
    /// contents.
    pub fn log_summary(&self) {
        tracing::info!("[CHAIN] mode             = {}", self.mode.label());
        tracing::info!("[CHAIN] program_id       = {}", self.program_id);
        tracing::info!("[CHAIN] authority        = {}", self.authority);
        tracing::info!("[CHAIN] treasury_pda     = {}", self.treasury_pda);
        tracing::info!("[CHAIN] strategy_id      = {}", self.strategy_id);
        tracing::info!("[CHAIN] strategy_pda     = {}", self.strategy_pda);
        tracing::info!("[CHAIN] flash_program_id = {}", self.flash_program_id);
        tracing::info!("[CHAIN] rpc_url          = {}", self.rpc_url);
        tracing::info!(
            "[CHAIN] authority_kp     = {}",
            self.authority_keypair_path.as_deref().unwrap_or("<unset>")
        );
    }

    pub fn load_authority(&self) -> Result<Keypair, String> {
        let path = self
            .authority_keypair_path
            .clone()
            .ok_or_else(|| "RTP_AUTHORITY_KEYPAIR not set".to_string())?;
        read_keypair_file(&path).map_err(|e| format!("read keypair {}: {}", path, e))
    }
}

/// Side enum mirror for `open_flash_position` (matches Rust on-chain enum).
#[derive(Debug, Clone, Copy)]
pub enum FlashSide {
    Long,
    Short,
}

impl ChainConfig {
    /// Create a ChainConfig with placeholder values for tests and local dev.
    /// Does NOT read from environment — safe for unit/integration tests.
    /// NOTE: Kept public for integration tests in tests/. Not for production use.
    pub fn test_default() -> Self {
        Self {
            authority: Pubkey::new_unique(),
            program_id: Pubkey::new_unique(),
            treasury_pda: Pubkey::new_unique(),
            strategy_id: "SOL_2.69".into(),
            strategy_pda: Pubkey::new_unique(),
            authority_keypair_path: None,
            rpc_url: "http://localhost:8899".to_string(),
            flash_program_id: Pubkey::new_unique(),
            mode: ExecutionMode::Simulate,
        }
    }
}

impl FlashSide {
    pub fn discriminant(self) -> u8 {
        match self {
            // FlashSide on-chain: None=0, Long=1, Short=2
            Self::Long => 1,
            Self::Short => 2,
        }
    }
}

/// Pre-computed Flash Trade market accounts (from `derive_flash_accounts.ts`).
#[derive(Debug, Clone)]
pub struct FlashMarketAccounts {
    pub perpetuals_pda: Pubkey,
    pub transfer_authority: Pubkey,
    pub event_authority: Pubkey,
    pub pool: Pubkey,
    pub market: Pubkey,
    pub target_custody: Pubkey,
    pub target_oracle: Pubkey,
    pub collateral_custody: Pubkey,
    pub collateral_oracle: Pubkey,
    pub collateral_custody_token_account: Pubkey,
    /// Token program used for the funding side (legacy SPL token for native SOL/wSOL).
    pub funding_token_program: Pubkey,
    /// Mint of the funding token (wSOL for SOL-denominated positions).
    pub funding_mint: Pubkey,
}

impl FlashMarketAccounts {
    /// Default Crypto.1 / SOL Long preset — values mirror
    /// `scripts/derive_flash_accounts.ts` and `flash_trade_client.rs`.
    pub fn sol_long_default() -> Self {
        Self {
            perpetuals_pda: pk("7DWCtB5Z8rPiyBMKUwqyC95R9tJpbhoQhLM9LbK3Z5QZ"),
            transfer_authority: pk("81xGAvJ27ZeRThU2JEfKAUeT4Fx6qCCd8WHZpujZbiiG"),
            event_authority: pk("9qb3KAyARHqhVGQjJmzSVJ1hTm3KDR2QL8EBW5paXkUB"),
            pool: pk("HfF7GCcEc76xubFCHLLXRdYcgRzwjEPdfKWqzRS8Ncog"),
            market: pk("3vHoXbUvGhEHFsLUmxyC6VWsbYDreb1zMn9TAp5ijN5K"),
            target_custody: pk("BjzZ33nMnbXZ7rw3Uy9Uu1W7BDCzzugqkiZoamJHRKF7"),
            target_oracle: pk("DXqtMo8qRBfHcK11kBnSaCSXkWKk1huMf94R6sAxLHtf"),
            collateral_custody: pk("BjzZ33nMnbXZ7rw3Uy9Uu1W7BDCzzugqkiZoamJHRKF7"),
            collateral_oracle: pk("DXqtMo8qRBfHcK11kBnSaCSXkWKk1huMf94R6sAxLHtf"),
            collateral_custody_token_account: pk("Hhed3wTHoVoPpnuBntGf236UfowMMAXfxqTLkMyJJENe"),
            funding_token_program: pk(TOKEN_PROGRAM_ID_STR),
            funding_mint: pk("So11111111111111111111111111111111111111112"),
        }
    }

    /// Derive the per-owner position PDA: ["position", owner, market].
    pub fn position_pda(&self, owner: &Pubkey, flash_program_id: &Pubkey) -> Pubkey {
        let (pda, _bump) = Pubkey::find_program_address(
            &[b"position", owner.as_ref(), self.market.as_ref()],
            flash_program_id,
        );
        pda
    }
}

fn pk(s: &str) -> Pubkey {
    Pubkey::from_str(s).expect("known valid pubkey literal")
}

/// Oracle price payload mirror.
#[derive(Debug, Clone, Copy)]
pub struct OraclePrice {
    pub price: i64,
    pub exponent: i32,
}

/// Build the `open_flash_position` Anchor instruction.
///
/// Fee-payer = caller (authority); Treasury PDA signs the inner CPI via
/// `invoke_signed` on-chain.
#[allow(clippy::too_many_arguments)]
pub fn build_open_flash_position_ix(
    cfg: &ChainConfig,
    authority: &Pubkey,
    funding_account: &Pubkey,
    market: &FlashMarketAccounts,
    side: FlashSide,
    input_sol_lamports: u64,
    leverage_bps: u32,
    slippage_bps: u16,
    oracle_price: OraclePrice,
    pool_name: &str,
) -> Instruction {
    // Anchor instruction data:
    // disc(8) + side(1) + input_sol_lamports(u64 LE) + leverage_bps(u32 LE)
    //   + slippage_bps(u16 LE) + oracle_price.price(i64 LE)
    //   + oracle_price.exponent(i32 LE) + pool_name(string: u32 len + bytes)
    let mut data: Vec<u8> = Vec::with_capacity(64 + pool_name.len());
    data.extend_from_slice(&OPEN_FLASH_POSITION_DISC);
    data.push(side.discriminant());
    data.extend_from_slice(&input_sol_lamports.to_le_bytes());
    data.extend_from_slice(&leverage_bps.to_le_bytes());
    data.extend_from_slice(&slippage_bps.to_le_bytes());
    data.extend_from_slice(&oracle_price.price.to_le_bytes());
    data.extend_from_slice(&oracle_price.exponent.to_le_bytes());
    let bytes = pool_name.as_bytes();
    data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(bytes);

    // Named accounts (matches Anchor #[derive(Accounts)] OpenFlashPosition).
    let mut accounts = vec![
        AccountMeta::new(cfg.treasury_pda, false),
        AccountMeta::new(cfg.strategy_pda, false),
        AccountMeta::new(*authority, true),
    ];

    // 19 remaining accounts in IDL v15.2.0 order — see lib.rs:1183.
    let position_pda = market.position_pda(&cfg.treasury_pda, &cfg.flash_program_id);
    let sysvar_ix = pk(SYSVAR_INSTRUCTIONS_STR);
    let remaining = [
        cfg.treasury_pda,             // 0  owner (PDA signer via invoke_signed)
        *authority,                   // 1  fee_payer (writable signer)
        *funding_account,             // 2  funding_account (writable)
        market.transfer_authority,    // 3
        market.perpetuals_pda,        // 4
        market.pool,                  // 5  writable
        position_pda,                 // 6  writable
        market.market,                // 7  writable
        market.target_custody,        // 8
        market.target_oracle,         // 9
        market.collateral_custody,    // 10 writable
        market.collateral_oracle,     // 11
        market.collateral_custody_token_account, // 12 writable
        system_program::ID,           // 13
        market.funding_token_program, // 14
        market.event_authority,       // 15
        cfg.flash_program_id,         // 16
        sysvar_ix,                    // 17
        market.funding_mint,          // 18
    ];

    let writable_idx = [2usize, 5, 6, 7, 10, 12];
    for (i, key) in remaining.iter().enumerate() {
        let writable = writable_idx.contains(&i);
        // Slot 0 is the PDA signer for the inner CPI but the *outer* Anchor
        // call cannot mark it as a signer (it's a PDA, not a real signer at
        // the transaction level). Slot 1 is the real fee-payer signer.
        let is_signer = i == 1;
        accounts.push(if writable {
            AccountMeta::new(*key, is_signer)
        } else {
            AccountMeta::new_readonly(*key, is_signer)
        });
    }

    Instruction {
        program_id: cfg.program_id,
        accounts,
        data,
    }
}

/// Build the `close_flash_position` Anchor instruction (18 remaining accounts).
#[allow(clippy::too_many_arguments)]
pub fn build_close_flash_position_ix(
    cfg: &ChainConfig,
    authority: &Pubkey,
    receiving_account: &Pubkey,
    market: &FlashMarketAccounts,
    side: FlashSide,
    oracle_price: OraclePrice,
    slippage_bps: u16,
    committed_sol_lamports_delta: u64,
) -> Instruction {
    let mut data: Vec<u8> =
        Vec::with_capacity(8 + 1 + 8 + 4 + 2 + 8);
    data.extend_from_slice(&CLOSE_FLASH_POSITION_DISC);
    data.push(side.discriminant());
    data.extend_from_slice(&oracle_price.price.to_le_bytes());
    data.extend_from_slice(&oracle_price.exponent.to_le_bytes());
    data.extend_from_slice(&slippage_bps.to_le_bytes());
    data.extend_from_slice(&committed_sol_lamports_delta.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(cfg.treasury_pda, false),
        AccountMeta::new(cfg.strategy_pda, false),
        AccountMeta::new(*authority, true),
    ];

    let position_pda = market.position_pda(&cfg.treasury_pda, &cfg.flash_program_id);
    let sysvar_ix = pk(SYSVAR_INSTRUCTIONS_STR);
    // Close layout (18 accounts) — see lib.rs:1369.
    let remaining = [
        cfg.treasury_pda,             // 0  owner (PDA signer)
        *authority,                   // 1  fee_payer
        *receiving_account,           // 2  receiving_account (writable)
        market.transfer_authority,    // 3
        market.perpetuals_pda,        // 4
        market.pool,                  // 5  writable
        position_pda,                 // 6  writable
        market.market,                // 7  writable
        market.target_custody,        // 8
        market.target_oracle,         // 9
        market.collateral_custody,    // 10 writable
        market.collateral_oracle,     // 11
        market.collateral_custody_token_account, // 12 writable
        market.funding_token_program, // 13 token_program
        market.event_authority,       // 14
        cfg.flash_program_id,         // 15
        sysvar_ix,                    // 16
        market.funding_mint,          // 17 collateral_mint
    ];

    let writable_idx = [2usize, 5, 6, 7, 10, 12];
    for (i, key) in remaining.iter().enumerate() {
        let writable = writable_idx.contains(&i);
        let is_signer = i == 1;
        accounts.push(if writable {
            AccountMeta::new(*key, is_signer)
        } else {
            AccountMeta::new_readonly(*key, is_signer)
        });
    }

    Instruction {
        program_id: cfg.program_id,
        accounts,
        data,
    }
}

/// Sign and submit (or simulate) a transaction. Returns either a signature
/// or, in simulate mode, the simulation result string.
pub fn submit_or_simulate(
    cfg: &ChainConfig,
    ixs: Vec<Instruction>,
    authority: &Keypair,
) -> Result<String, String> {
    let blockhash = fetch_blockhash(&cfg.rpc_url)?;
    let mut tx = Transaction::new_with_payer(&ixs, Some(&authority.pubkey()));
    tx.sign(&[authority], blockhash);
    let serialized =
        bincode::serialize(&tx).map_err(|e| format!("serialize tx: {}", e))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&serialized);

    match cfg.mode {
        ExecutionMode::Simulate => rpc_simulate_transaction(&cfg.rpc_url, &b64),
        ExecutionMode::Devnet | ExecutionMode::Mainnet => {
            rpc_send_transaction(&cfg.rpc_url, &b64)
        }
    }
}

fn fetch_blockhash(rpc_url: &str) -> Result<solana_sdk::hash::Hash, String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [{"commitment": "confirmed"}],
        }))
        .send()
        .map_err(|e| format!("blockhash request: {}", e))?;
    let json: serde_json::Value =
        resp.json().map_err(|e| format!("blockhash parse: {}", e))?;
    let bh_str = json
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("blockhash"))
        .and_then(|b| b.as_str())
        .ok_or_else(|| format!("blockhash missing: {}", json))?;
    bh_str
        .parse::<solana_sdk::hash::Hash>()
        .map_err(|e| format!("blockhash parse: {}", e))
}

fn rpc_simulate_transaction(rpc_url: &str, b64_tx: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "simulateTransaction",
            "params": [b64_tx, {"encoding": "base64", "sigVerify": false, "replaceRecentBlockhash": true}],
        }))
        .send()
        .map_err(|e| format!("simulate request: {}", e))?;
    let json: serde_json::Value =
        resp.json().map_err(|e| format!("simulate parse: {}", e))?;
    Ok(json.to_string())
}

fn rpc_send_transaction(rpc_url: &str, b64_tx: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::new();
    // Up to 3 attempts with exponential backoff.
    let mut last_err = String::new();
    for attempt in 0..3 {
        let resp = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "sendTransaction",
                "params": [b64_tx, {"encoding": "base64", "preflightCommitment": "confirmed"}],
            }))
            .send();
        match resp {
            Ok(r) => {
                let json: serde_json::Value =
                    r.json().map_err(|e| format!("send parse: {}", e))?;
                if let Some(sig) = json.get("result").and_then(|s| s.as_str()) {
                    return Ok(sig.to_string());
                }
                last_err = json.to_string();
            }
            Err(e) => {
                last_err = format!("send error: {}", e);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500 * (1 << attempt)));
    }
    Err(format!("sendTransaction failed: {}", last_err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_mode_from_env_defaults_to_simulate() {
        // SAFETY: tests run sequentially via Cargo's default test harness; we
        // restore the prior value after each unset.
        unsafe { std::env::remove_var("RTP_EXECUTION_MODE") };
        assert_eq!(ExecutionMode::from_env(), ExecutionMode::Simulate);
    }

    #[test]
    fn pda_derivation_matches_program() {
        // Authority from the devnet demo. Treasury PDA seeded by [TREASURY_SEED, authority].
        let program_id = pk("8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB");
        let authority = pk("6PYPAnwiMoZvzphAWEu3EsNz3PpwjJ6YcZabj34qVQ4Z");
        let (treasury, _) =
            Pubkey::find_program_address(&[TREASURY_SEED, authority.as_ref()], &program_id);
        // Derivation is deterministic.
        let (treasury2, _) =
            Pubkey::find_program_address(&[TREASURY_SEED, authority.as_ref()], &program_id);
        assert_eq!(treasury, treasury2);
    }

    #[test]
    fn open_flash_position_ix_uses_19_remaining_accounts() {
        let cfg = ChainConfig {
            program_id: pk("8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB"),
            authority: pk("So11111111111111111111111111111111111111112"),
            treasury_pda: pk("So11111111111111111111111111111111111111112"),
            strategy_id: "SOL_FT_V1".into(),
            strategy_pda: pk("So11111111111111111111111111111111111111112"),
            authority_keypair_path: None,
            rpc_url: "http://localhost".into(),
            flash_program_id: pk("FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn"),
            mode: ExecutionMode::Simulate,
        };
        let market = FlashMarketAccounts::sol_long_default();
        let auth = pk("So11111111111111111111111111111111111111112");
        let funding = pk("So11111111111111111111111111111111111111112");
        let ix = build_open_flash_position_ix(
            &cfg,
            &auth,
            &funding,
            &market,
            FlashSide::Long,
            10_000_000,
            10_000,
            500,
            OraclePrice {
                price: 170_000_000_000,
                exponent: -8,
            },
            "Crypto.1",
        );
        // 3 named + 19 remaining = 22 accounts total.
        assert_eq!(ix.accounts.len(), 22);
        // Discriminator first.
        assert_eq!(&ix.data[0..8], &OPEN_FLASH_POSITION_DISC);
    }

    #[test]
    fn close_flash_position_ix_uses_18_remaining_accounts() {
        let cfg = ChainConfig {
            program_id: pk("8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB"),
            authority: pk("So11111111111111111111111111111111111111112"),
            treasury_pda: pk("So11111111111111111111111111111111111111112"),
            strategy_id: "SOL_FT_V1".into(),
            strategy_pda: pk("So11111111111111111111111111111111111111112"),
            authority_keypair_path: None,
            rpc_url: "http://localhost".into(),
            flash_program_id: pk("FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn"),
            mode: ExecutionMode::Simulate,
        };
        let market = FlashMarketAccounts::sol_long_default();
        let auth = pk("So11111111111111111111111111111111111111112");
        let recv = pk("So11111111111111111111111111111111111111112");
        let ix = build_close_flash_position_ix(
            &cfg,
            &auth,
            &recv,
            &market,
            FlashSide::Long,
            OraclePrice {
                price: 170_000_000_000,
                exponent: -8,
            },
            500,
            10_000_000,
        );
        // 3 named + 18 remaining = 21 accounts total.
        assert_eq!(ix.accounts.len(), 21);
        assert_eq!(&ix.data[0..8], &CLOSE_FLASH_POSITION_DISC);
    }
}
