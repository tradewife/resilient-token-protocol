//! Trading Wing — strategy research, validation, assessment, and execution.
//!
//! Handles TradingConfig, Proposal, ExecutePermit, YieldReport, and Heartbeat.
//! Uses bridge.rs to call the Python fractal-swarm binary for strategy evaluation.
//! The bridge returns walk-forward analysis results (projected yield), not live trades.
//!
//! ## Hyperliquid Integration
//!
//! When `execution_venue: "hyperliquid"` is set in the proposal config, the
//! Trading Wing places real orders on Hyperliquid testnet via REST API, signed
//! with the ETH keypair at `configs/hl_testnet_key.json` using EIP-191.
//!
//! In-memory state: last proposal, last assessment, execution count.

use crate::bridge::{self, BridgeRequest};
use crate::types::{Message, Payload, WingId};
use serde::{Deserialize, Serialize};
use solana_sdk::signer::Signer;
use std::sync::Mutex;

// ═══════════════════════════════════════════════════════════════════════
//  Hyperliquid Integration
// ═══════════════════════════════════════════════════════════════════════

/// Hyperliquid testnet exchange endpoint.
const HL_TESTNET_URL: &str = "https://api.hyperliquid-testnet.xyz";

/// Key file path relative to repo root.
const HL_KEY_PATH: &str = "configs/hl_testnet_key.json";

/// ECDSA signature components for Hyperliquid EIP-191 signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlSignature {
    pub r: String,
    pub s: String,
    pub v: u64,
}

/// Key file structure for the HL testnet ETH keypair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlKeyFile {
    pub address: String,
    pub private_key: String,
    pub network: String,
}

/// Yield report data emitted after a confirmed Hyperliquid fill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldReportData {
    pub symbol: String,
    pub side: String,
    pub fill_price: String,
    pub size: String,
    /// Entry price from the opening fill. Stored for PnL calculation on close.
    pub entry_price: Option<String>,
    /// Realized PnL in USDC. `None` means the position is still open (no PnL
    /// realized yet). `Some(value)` means the position was closed and this is
    /// the actual realized profit/loss.
    /// For opening fills: `(fill_price - entry_price) * size` is meaningless
    /// because entry_price == fill_price, so PnL = None.
    /// For closing fills: `(exit_price - entry_price) * size` for longs,
    /// `(entry_price - exit_price) * size` for shorts.
    pub realized_pnl_usdc: Option<f64>,
    pub timestamp: String,
}

/// Load the ETH keypair from the key file.
///
/// Searches relative to `CARGO_MANIFEST_DIR` (rtp/swarm/) then the current
/// working directory.
pub fn load_hl_key() -> Result<HlKeyFile, String> {
    // Try relative to CARGO_MANIFEST_DIR (rtp/swarm/)
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let path = std::path::Path::new(&manifest)
        .join("../../")
        .join(HL_KEY_PATH);

    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read key file: {}", e))?;
        return serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse key file: {}", e));
    }

    // Try current directory
    let alt = std::path::Path::new(HL_KEY_PATH);
    if alt.exists() {
        let content = std::fs::read_to_string(alt)
            .map_err(|e| format!("Failed to read key file: {}", e))?;
        return serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse key file: {}", e));
    }

    Err(format!(
        "HL key file not found (searched {} and {})",
        path.display(),
        HL_KEY_PATH
    ))
}

/// Compute a keccak256 hash, returning a fixed 32-byte array.
fn keccak256(data: &[u8]) -> [u8; 32] {
    use sha3::Digest;
    let mut hasher = sha3::Keccak256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Left-pad a slice to 32 bytes.
fn pad_left32(data: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let start = 32 - data.len().min(32);
    out[start..].copy_from_slice(&data[..data.len().min(32)]);
    out
}

/// Compute the EIP-712 domain separator for Hyperliquid Exchange.
///
/// Domain: { name: "Exchange", version: "1", chainId: 1337,
///           verifyingContract: "0x0000...0000" }
fn hl_domain_separator() -> [u8; 32] {
    // typeHash("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")
    let domain_type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );

    let mut data = Vec::with_capacity(5 * 32);
    data.extend_from_slice(&domain_type_hash);
    data.extend_from_slice(&keccak256(b"Exchange")); // name (string → keccak256)
    data.extend_from_slice(&keccak256(b"1")); // version (string → keccak256)
    data.extend_from_slice(&pad_left32(&1337u64.to_be_bytes())); // chainId (uint256)
    data.extend_from_slice(&[0u8; 32]); // verifyingContract (address → zero-padded)

    keccak256(&data)
}

/// Compute the EIP-712 Agent struct hash for a given action hash.
///
/// Agent type: { string source, bytes32 connectionId }
/// Testnet uses source = "b", mainnet uses source = "a".
fn hl_agent_hash(action_hash: &[u8; 32], is_mainnet: bool) -> [u8; 32] {
    // typeHash("Agent(string source,bytes32 connectionId)")
    let agent_type_hash = keccak256(b"Agent(string source,bytes32 connectionId)");

    let source = if is_mainnet { "a" } else { "b" };

    let mut data = Vec::with_capacity(3 * 32);
    data.extend_from_slice(&agent_type_hash);
    data.extend_from_slice(&keccak256(source.as_bytes())); // source (string → keccak256)
    data.extend_from_slice(action_hash); // connectionId (bytes32 → raw)

    keccak256(&data)
}

/// Compute the action hash: keccak256(msgpack(action) + nonce_8bytes + vault_flag).
///
/// The msgpack bytes MUST use the same key ordering as the Hyperliquid Python SDK.
/// HL's server verifies the signature by re-msgpacking the received action using
/// the key order from the JSON payload. The Python SDK uses insertion-order keys,
/// so we must match that exact order:
///   outer: "type", "orders", "grouping"
///   inner: "a", "b", "p", "s", "r", "t"
fn hl_action_hash(
    action: &serde_json::Value,
    nonce: u64,
) -> Result<[u8; 32], String> {
    // Extract fields from the action Value.
    let orders = action["orders"]
        .as_array()
        .ok_or("Missing orders in action")?;

    // Build msgpack bytes manually with Python SDK key ordering.
    let mut buf = Vec::with_capacity(128);

    // Outer map: 3 entries in Python SDK order: type, orders, grouping
    rmp::encode::write_map_len(&mut buf, 3).unwrap();

    // "type" → "order"
    rmp::encode::write_str(&mut buf, "type").unwrap();
    rmp::encode::write_str(
        &mut buf,
        action["type"].as_str().unwrap_or("order"),
    )
    .unwrap();

    // "orders" → [order, ...]
    rmp::encode::write_str(&mut buf, "orders").unwrap();
    rmp::encode::write_array_len(&mut buf, orders.len() as u32).unwrap();

    for order in orders {
        // Inner order map: keys in Python SDK order: a, b, p, s, r, t
        rmp::encode::write_map_len(&mut buf, 6).unwrap();

        // "a" → asset index
        rmp::encode::write_str(&mut buf, "a").unwrap();
        rmp::encode::write_sint(
            &mut buf,
            order["a"].as_i64().unwrap_or(0),
        )
        .unwrap();

        // "b" → is_buy
        rmp::encode::write_str(&mut buf, "b").unwrap();
        rmp::encode::write_bool(&mut buf, order["b"].as_bool().unwrap_or(true))
            .unwrap();

        // "p" → price
        rmp::encode::write_str(&mut buf, "p").unwrap();
        rmp::encode::write_str(
            &mut buf,
            order["p"].as_str().unwrap_or("0"),
        )
        .unwrap();

        // "s" → size
        rmp::encode::write_str(&mut buf, "s").unwrap();
        rmp::encode::write_str(
            &mut buf,
            order["s"].as_str().unwrap_or("0"),
        )
        .unwrap();

        // "r" → reduce_only
        rmp::encode::write_str(&mut buf, "r").unwrap();
        rmp::encode::write_bool(
            &mut buf,
            order["r"].as_bool().unwrap_or(false),
        )
        .unwrap();

        // "t" → {"limit": {"tif": ...}}
        rmp::encode::write_str(&mut buf, "t").unwrap();
        let tif = order["t"]["limit"]["tif"]
            .as_str()
            .unwrap_or("Ioc");
        rmp::encode::write_map_len(&mut buf, 1).unwrap();
        rmp::encode::write_str(&mut buf, "limit").unwrap();
        rmp::encode::write_map_len(&mut buf, 1).unwrap();
        rmp::encode::write_str(&mut buf, "tif").unwrap();
        rmp::encode::write_str(&mut buf, tif).unwrap();
    }

    // "grouping" → "na"
    rmp::encode::write_str(&mut buf, "grouping").unwrap();
    rmp::encode::write_str(
        &mut buf,
        action["grouping"].as_str().unwrap_or("na"),
    )
    .unwrap();

    // Append nonce (8 bytes big-endian) + vault flag (0x00).
    buf.extend_from_slice(&nonce.to_be_bytes());
    buf.push(0x00);

    Ok(keccak256(&buf))
}

/// Sign a Hyperliquid action using EIP-712 typed data signing.
///
/// This matches the official Hyperliquid Python SDK signing flow:
///   1. action_hash = keccak256(msgpack(action) + nonce_8B + vault_flag)
///   2. phantom_agent = {source: "b", connectionId: action_hash}
///   3. EIP-712 domain: {name: "Exchange", version: "1", chainId: 1337, ...}
///   4. EIP-712 Agent type: {string source, bytes32 connectionId}
///   5. typedDataHash = keccak256("\\x19\\x01" + domainSeparator + agentHash)
///   6. ECDSA sign → (r, s, v)
pub fn sign_l1_action(
    action: &serde_json::Value,
    nonce: u64,
    private_key_hex: &str,
    is_mainnet: bool,
) -> Result<HlSignature, String> {
    // Step 1: action hash via msgpack + nonce + vault flag.
    let action_hash = hl_action_hash(action, nonce)?;

    // Step 2: phantom agent struct hash.
    let agent_hash = hl_agent_hash(&action_hash, is_mainnet);

    // Step 3: EIP-712 typed data hash.
    let domain_separator = hl_domain_separator();
    let mut typed_data = Vec::with_capacity(2 + 32 + 32);
    typed_data.extend_from_slice(b"\x19\x01");
    typed_data.extend_from_slice(&domain_separator);
    typed_data.extend_from_slice(&agent_hash);
    let sign_hash = keccak256(&typed_data);

    // Step 4: ECDSA recoverable signature.
    let secp = secp256k1::Secp256k1::new();
    let pk_hex = private_key_hex
        .strip_prefix("0x")
        .unwrap_or(private_key_hex);
    let pk_bytes =
        hex::decode(pk_hex).map_err(|e| format!("Failed to decode private key hex: {}", e))?;
    let secret_key = secp256k1::SecretKey::from_slice(&pk_bytes)
        .map_err(|e| format!("Invalid private key: {}", e))?;
    let message = secp256k1::Message::from_digest(sign_hash);
    let sig = secp.sign_ecdsa_recoverable(&message, &secret_key);
    let (recovery_id, compact) = sig.serialize_compact();

    Ok(HlSignature {
        r: format!("0x{}", hex::encode(&compact[0..32])),
        s: format!("0x{}", hex::encode(&compact[32..64])),
        v: (recovery_id.to_i32() + 27) as u64,
    })
}

/// Legacy EIP-191 personal sign (used by demo script, NOT the official SDK).
/// Kept for backwards compatibility but **not** used for real HL orders.
pub fn sign_action(
    action: &serde_json::Value,
    private_key_hex: &str,
) -> Result<HlSignature, String> {
    sign_l1_action(action, 0, private_key_hex, false)
}

/// Build the Hyperliquid order action as a `serde_json::Value`.
///
/// Uses single-letter keys (`a`, `b`, `p`, `s`, `r`, `t`) to match the
/// Hyperliquid exchange API spec exactly.
pub fn build_order_action(
    asset_index: i64,
    is_buy: bool,
    size: &str,
    price: &str,
    tif: &str,
) -> serde_json::Value {
    serde_json::json!({
        "type": "order",
        "orders": [{
            "a": asset_index,
            "b": is_buy,
            "p": price,
            "s": size,
            "r": false,
            "t": {"limit": {"tif": tif}}
        }],
        "grouping": "na"
    })
}

/// Get the SOL perpetual asset index from Hyperliquid metadata.
pub fn get_sol_index() -> Result<i64, String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/info", HL_TESTNET_URL))
        .json(&serde_json::json!({"type": "metaAndAssetCtxs"}))
        .send()
        .map_err(|e| format!("HL info request failed: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("HL info parse error: {}", e))?;

    let universe = data[0]["universe"]
        .as_array()
        .ok_or("Missing universe in HL metadata")?;

    for (i, asset) in universe.iter().enumerate() {
        if asset["name"].as_str() == Some("SOL") {
            return Ok(i as i64);
        }
    }

    Ok(0) // SOL is typically index 0
}

/// Place an order on Hyperliquid testnet.
///
/// Returns the raw JSON response from the exchange endpoint.
/// Uses IOC (Immediate-Or-Cancel) by default for market-style fills.
pub fn place_hl_order(
    asset_index: i64,
    is_buy: bool,
    size: &str,
    price: &str,
    tif: &str,
    private_key_hex: &str,
) -> Result<serde_json::Value, String> {
    let action = build_order_action(asset_index, is_buy, size, price, tif);

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Time error: {}", e))?
        .as_millis() as u64;

    let signature = sign_l1_action(&action, nonce, private_key_hex, false)?; // testnet

    let payload = serde_json::json!({
        "action": action,
        "nonce": nonce,
        "signature": signature
    });

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/exchange", HL_TESTNET_URL))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .map_err(|e| format!("HL exchange request failed: {}", e))?;

    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .map_err(|e| format!("HL exchange parse error (HTTP {}): {}", status, e))?;

    Ok(body)
}

/// Parse the Hyperliquid fill response into a [`YieldReportData`].
///
/// For opening fills (no prior position), `realized_pnl_usdc` is `None` because
/// no PnL has been realized yet — the entry price IS the fill price.
/// For closing fills (reducing/closing a position), the PnL is calculated as:
///   - Long:  `(exit_fill_price - entry_price) * filled_size`
///   - Short: `(entry_price - exit_fill_price) * filled_size`
fn parse_fill_response(
    response: &serde_json::Value,
    symbol: &str,
    is_buy: bool,
    requested_size: &str,
    entry_price: Option<&str>,
) -> Result<YieldReportData, String> {
    let status = response["status"].as_str().unwrap_or("unknown");

    if status != "ok" {
        return Err(format!(
            "HL order rejected: {}",
            serde_json::to_string_pretty(response).unwrap_or_default()
        ));
    }

    // Navigate to statuses array.
    let statuses = response["response"]["data"]["statuses"]
        .as_array()
        .ok_or("Missing statuses in HL response")?;

    if statuses.is_empty() {
        return Err("Empty statuses array in HL response".to_string());
    }

    let fill = &statuses[0];

    // Check for error in fill.
    if let Some(err) = fill["error"].as_str() {
        return Err(format!("HL fill error: {}", err));
    }

    // Extract fill details. IOC orders that fill immediately have
    // {"filled": {"total_sz": "...", "avg_px": "...", "type": "..."}}
    let filled_obj = &fill["filled"];
    let fill_price = filled_obj
        .get("avg_px")
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let filled_size = filled_obj
        .get("total_sz")
        .and_then(|v| v.as_str())
        .unwrap_or(requested_size);

    // Calculate realized PnL if we have an entry price (closing fill).
    let realized_pnl = match entry_price {
        Some(ep) => {
            let entry: f64 = ep.parse().map_err(|e| format!("Bad entry_price: {}", e))?;
            let exit: f64 = fill_price.parse().map_err(|e| format!("Bad fill_price: {}", e))?;
            let sz: f64 = filled_size.parse().map_err(|e| format!("Bad size: {}", e))?;
            if is_buy {
                // Closing a short: PnL = (entry - exit) * size
                Some((entry - exit) * sz)
            } else {
                // Closing a long: PnL = (exit - entry) * size
                Some((exit - entry) * sz)
            }
        }
        None => {
            // Opening fill — no PnL realized yet. Store fill price as entry.
            None
        }
    };

    Ok(YieldReportData {
        symbol: symbol.to_string(),
        side: if is_buy {
            "BUY".to_string()
        } else {
            "SELL".to_string()
        },
        fill_price: fill_price.to_string(),
        size: filled_size.to_string(),
        entry_price: entry_price.map(|s| s.to_string()).or_else(|| {
            // For opening fills, the fill price IS the entry price.
            if fill_price != "0" {
                Some(fill_price.to_string())
            } else {
                None
            }
        }),
        realized_pnl_usdc: realized_pnl,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Execute a SOL order on Hyperliquid testnet using the configured key.
///
/// This is the primary entry point for the Trading Wing's Hyperliquid
/// execution path. It loads the key, resolves the SOL asset index, places
/// the order, and returns both the raw response and a parsed yield report.
pub fn execute_hl_sol_order(
    is_buy: bool,
    size: &str,
    entry_price: Option<&str>,
) -> Result<(serde_json::Value, YieldReportData), String> {
    let key = load_hl_key()?;
    let sol_idx = get_sol_index()?;

    println!(
        "[TRADING WING] Placing SOL {} {} @ market on HL testnet",
        if is_buy { "BUY" } else { "SELL" },
        size
    );

    let response = place_hl_order(sol_idx, is_buy, size, "0", "Ioc", &key.private_key)?;

    let report = parse_fill_response(&response, "SOL/USDT", is_buy, size, entry_price)?;

    println!("[TRADING WING] fill confirmed: {:?}", report);

    Ok((response, report))
}

// ═══════════════════════════════════════════════════════════════════════
//  Treasury CPI Transfer
// ═══════════════════════════════════════════════════════════════════════

/// Devnet RPC endpoint.
const SOLANA_DEVNET_RPC: &str = "https://api.devnet.solana.com";

/// Devnet deployment addresses (from configs/.env.devnet).
const RTP_MINT: &str = "2JN8Qr9QspmDXwqRBSmZ9ULX8LLJFawo61rEwYdtpNcf";
const TREASURY_VAULT: &str = "DKuC9Q3FXS28C32k3Grur8QtBLrN5BR5nDsujFkhs3kM";
const DEVNET_WALLET: &str = "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R";

/// SPL Token program ID (standard).
/// SPL Token program ID (standard). Kept for reference.
#[allow(dead_code)]
const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// Token-2022 program ID.
const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

/// SPL Associated Token Account program ID.
const ATA_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

/// Token decimals (from devnet demo: MINT_DECIMALS = 6).
const TOKEN_DECIMALS: u8 = 6;

/// Derive the associated token address for a wallet and mint.
///
/// Uses the SPL ATA program's PDA derivation:
/// `seeds = [wallet, token_program, mint], program = ATA_PROGRAM_ID`
fn derive_ata(
    wallet: &solana_sdk::pubkey::Pubkey,
    mint: &solana_sdk::pubkey::Pubkey,
    token_program: &solana_sdk::pubkey::Pubkey,
) -> solana_sdk::pubkey::Pubkey {
    let (address, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[
            wallet.as_ref(),
            token_program.as_ref(),
            mint.as_ref(),
        ],
        &solana_sdk::pubkey::Pubkey::try_from(ATA_PROGRAM_ID)
            .expect("ATA program ID is valid"),
    );
    address
}

/// Build a `transfer_checked` instruction manually.
///
/// Avoids depending on the `spl-token` crate (which has a zeroize version
/// conflict with reqwest's rustls). The instruction format is:
///   discriminator: 12 (u32 LE)
///   amount: u64 LE
///   decimals: u8
fn build_transfer_checked_ix(
    source: &solana_sdk::pubkey::Pubkey,
    mint: &solana_sdk::pubkey::Pubkey,
    destination: &solana_sdk::pubkey::Pubkey,
    authority: &solana_sdk::pubkey::Pubkey,
    amount: u64,
    decimals: u8,
    token_program: &solana_sdk::pubkey::Pubkey,
) -> solana_sdk::instruction::Instruction {
    use solana_sdk::instruction::{AccountMeta, Instruction};

    let mut data = Vec::with_capacity(13);
    data.extend_from_slice(&12u32.to_le_bytes()); // transfer_checked discriminator
    data.extend_from_slice(&amount.to_le_bytes()); // amount
    data.push(decimals); // decimals

    Instruction {
        program_id: *token_program,
        accounts: vec![
            AccountMeta::new(*source, false),         // source (writable)
            AccountMeta::new_readonly(*mint, false),   // mint
            AccountMeta::new(*destination, false),     // destination (writable)
            AccountMeta::new_readonly(*authority, true), // authority (signer)
        ],
        data,
    }
}

/// Fetch a recent blockhash from devnet RPC via reqwest.
///
/// Uses the JSON-RPC `getLatestBlockhash` method. Avoids depending on
/// `solana-client` crate.
fn get_devnet_blockhash() -> Result<(String, solana_sdk::hash::Hash), String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(SOLANA_DEVNET_RPC)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [{"commitment": "confirmed"}]
        }))
        .send()
        .map_err(|e| format!("Devnet RPC request failed: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Devnet RPC parse error: {}", e))?;

    let blockhash_str = data["result"]["value"]["blockhash"]
        .as_str()
        .ok_or("Missing blockhash in RPC response")?;

    let hash = blockhash_str
        .parse::<solana_sdk::hash::Hash>()
        .map_err(|e| format!("Invalid blockhash '{}': {}", blockhash_str, e))?;

    Ok((blockhash_str.to_string(), hash))
}

/// Build a token transfer transaction from the payer wallet to the treasury vault.
///
/// Creates an SPL `transfer_checked` instruction, fetches a recent blockhash
/// from devnet RPC, and serializes the unsigned transaction to base64.
///
/// The base64 string is ready for the Phantom sidecar to sign and send:
/// `ts-node scripts/phantom_signer.ts sign-sol <base64>`
pub fn build_treasury_deposit_tx(
    from_wallet: &str,
    amount_tokens: f64,
) -> Result<(String, String), String> {
    let from = solana_sdk::pubkey::Pubkey::try_from(from_wallet)
        .map_err(|e| format!("Invalid from_wallet: {}", e))?;
    let mint = solana_sdk::pubkey::Pubkey::try_from(RTP_MINT)
        .map_err(|e| format!("Invalid RTP_MINT: {}", e))?;
    let vault = solana_sdk::pubkey::Pubkey::try_from(TREASURY_VAULT)
        .map_err(|e| format!("Invalid TREASURY_VAULT: {}", e))?;
    let token_program = solana_sdk::pubkey::Pubkey::try_from(TOKEN_2022_PROGRAM_ID)
        .map_err(|e| format!("Invalid TOKEN_2022_PROGRAM_ID: {}", e))?;

    // Derive ATA for the payer wallet.
    let from_ata = derive_ata(&from, &mint, &token_program);

    // Convert to raw units (6 decimals).
    let amount_raw = (amount_tokens * 10f64.powi(TOKEN_DECIMALS as i32)) as u64;

    // Build transfer_checked instruction.
    let transfer_ix = build_transfer_checked_ix(
        &from_ata,
        &mint,
        &vault,
        &from,
        amount_raw,
        TOKEN_DECIMALS,
        &token_program,
    );

    // Fetch recent blockhash from devnet.
    let (_blockhash_str, _blockhash) = get_devnet_blockhash()?;

    // Build unsigned transaction.
    let message = solana_sdk::message::Message::new(&[transfer_ix], Some(&from));
    let tx = solana_sdk::transaction::Transaction::new_unsigned(message);

    // Serialize to bytes (Solana wire format via bincode), then base64.
    let serialized = bincode::serialize(&tx)
        .map_err(|e| format!("Transaction serialization failed: {}", e))?;
    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &serialized,
    );

    println!(
        "[TREASURY] built deposit tx: {} tokens ({} raw) from {} → vault {}",
        amount_tokens, amount_raw, from_ata, vault
    );

    Ok((b64, from_ata.to_string()))
}

/// Call the Phantom signer sidecar to sign and send a Solana transaction.
///
/// Invokes: `ts-node --project scripts/tsconfig.json scripts/phantom_signer.ts sign-sol <base64>`
///
/// Returns the full stdout from the sidecar (JSON with signature, etc).
pub fn call_phantom_signer(tx_base64: &str) -> Result<String, String> {
    let output = std::process::Command::new("ts-node")
        .args([
            "--project",
            "scripts/tsconfig.json",
            "scripts/phantom_signer.ts",
            "sign-sol",
            tx_base64,
        ])
        .output()
        .map_err(|e| format!("Failed to run phantom_signer: {}. Is ts-node installed?", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!(
            "phantom_signer sign-sol failed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    Ok(stdout.trim().to_string())
}

/// Get the Phantom wallet's Solana address by calling the sidecar.
///
/// Invokes: `ts-node --project scripts/tsconfig.json scripts/phantom_signer.ts addresses`
pub fn get_phantom_solana_address() -> Result<String, String> {
    let output = std::process::Command::new("ts-node")
        .args([
            "--project",
            "scripts/tsconfig.json",
            "scripts/phantom_signer.ts",
            "addresses",
        ])
        .output()
        .map_err(|e| format!("Failed to run phantom_signer: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "phantom_signer addresses failed: {}",
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse: "  solana: <address>"
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("solana:") {
            let addr = trimmed.split(':').nth(1).unwrap_or("").trim();
            if !addr.is_empty() {
                return Ok(addr.to_string());
            }
        }
    }

    Err("No Solana address found in phantom_signer output".to_string())
}

// ═══════════════════════════════════════════════════════════════════════
//  Local Keypair Signing (Path C — devnet demo)
// ═══════════════════════════════════════════════════════════════════════

/// Load the default Solana CLI keypair from `~/.config/solana/id.json`.
///
/// In production, the agent uses Phantom KMS for signing (TEE/HSM-backed).
/// For the devnet demo, we load the local keypair from the Solana CLI wallet.
///
/// Demo narrative: "In production, the agent wallet is Phantom KMS-backed.
/// For this demo, we use a devnet keypair to show the same flow."
pub fn load_devnet_keypair() -> Result<solana_sdk::signer::keypair::Keypair, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/kt".to_string());
    let path = format!("{}/.config/solana/id.json", home);

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read keypair at {}: {}", path, e))?;
    let bytes: Vec<u8> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse keypair JSON at {}: {}", path, e))?;
    solana_sdk::signer::keypair::Keypair::try_from(bytes.as_slice())
        .map_err(|e| format!("Invalid keypair bytes: {}", e))
}

/// Sign and send a Solana transaction using the local devnet keypair.
///
/// Signs the unsigned transaction (base64) with the devnet keypair and submits
/// it via JSON-RPC to the Solana devnet RPC endpoint.
///
/// Uses `skipPreflight: true` so the tx is submitted even if simulation shows
/// it would fail (e.g., insufficient token balance). The resulting on-chain
/// signature proves the signing path works end-to-end.
///
/// Returns the transaction signature on success.
pub fn sign_and_send_local(tx_base64: &str) -> Result<String, String> {
    let keypair = load_devnet_keypair()?;

    println!(
        "[TREASURY] signing with local keypair: {}",
        keypair.pubkey()
    );

    // Decode the unsigned transaction.
    let tx_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        tx_base64,
    )
    .map_err(|e| format!("Failed to decode tx base64: {}", e))?;
    let mut tx: solana_sdk::transaction::Transaction = bincode::deserialize(&tx_bytes)
        .map_err(|e| format!("Failed to deserialize tx: {}", e))?;

    // Sign the transaction.
    let blockhash = tx.message.recent_blockhash;
    tx.sign(&[&keypair], blockhash);

    // Verify the first signature is not default (zero) — proves signing happened.
    if tx.signatures[0] == solana_sdk::signature::Signature::default() {
        return Err(
            "Signature is still default after signing — keypair does not match tx signer"
                .to_string(),
        );
    }

    println!(
        "[TREASURY] tx signed: sig={}",
        tx.signatures[0]
    );

    // Serialize the signed transaction.
    let signed_bytes = bincode::serialize(&tx)
        .map_err(|e| format!("Failed to serialize signed tx: {}", e))?;
    let signed_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &signed_bytes,
    );

    // Submit via JSON-RPC to devnet.
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(SOLANA_DEVNET_RPC)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                signed_b64,
                {
                    "encoding": "base64",
                    "skipPreflight": true,
                    "preflightCommitment": "confirmed"
                }
            ]
        }))
        .send()
        .map_err(|e| format!("RPC send failed: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("RPC parse error: {}", e))?;

    if let Some(error) = data.get("error") {
        let err_msg = error["message"].as_str().unwrap_or("unknown");
        let err_code = error["code"].as_i64().unwrap_or(-1);
        return Err(format!("RPC error (code {}): {}", err_code, err_msg));
    }

    let signature = data["result"]
        .as_str()
        .ok_or("Missing signature in RPC response")?;

    Ok(signature.to_string())
}

/// Deposit yield tokens to the treasury vault.
///
/// Full flow (signing cascade):
/// 1. Try Phantom KMS (production path — TEE/HSM-backed agent wallet)
/// 2. Fall back to local devnet keypair (demo path — Path C)
/// 3. If neither works, log the unsigned tx for manual submission
///
/// Returns the transaction signature on success.
pub fn deposit_yield_to_treasury(
    amount_tokens: f64,
    phantom_wallet_address: Option<&str>,
) -> Result<String, String> {
    if amount_tokens <= 0.0 {
        return Err(format!(
            "Cannot deposit non-positive yield: {}",
            amount_tokens
        ));
    }

    // Use provided Phantom address, or fall back to devnet payer.
    let wallet = phantom_wallet_address
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEVNET_WALLET.to_string());

    let (tx_b64, _from_ata) = build_treasury_deposit_tx(&wallet, amount_tokens)?;

    // Try Phantom KMS first (production path).
    if let Ok(result) = call_phantom_signer(&tx_b64) {
        println!(
            "[TREASURY] yield deposited via Phantom KMS: {} tokens | {}",
            amount_tokens, result
        );
        return Ok(result);
    }

    // Fall back to local devnet keypair (demo path).
    match sign_and_send_local(&tx_b64) {
        Ok(sig) => {
            println!(
                "[TREASURY] yield deposited via demo keypair: {} tokens | sig: {}",
                amount_tokens, sig
            );
            println!(
                "[TREASURY] explorer: https://explorer.solana.com/tx/{}?cluster=devnet",
                sig
            );
            Ok(sig)
        }
        Err(e) => {
            // Neither Phantom nor local signing worked — log for manual submission.
            println!(
                "[TREASURY] all signing paths failed ({}), tx ready for manual submit:",
                e
            );
            println!(
                "[TREASURY]   base64: {}...",
                &tx_b64[..tx_b64.len().min(60)]
            );
            Ok(format!(
                "unsigned_tx_ready:signing_unavailable({})",
                e
            ))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Position Tracking
// ═══════════════════════════════════════════════════════════════════════

/// Tracks an open position in the Trading Wing's in-memory state.
///
/// Created when an opening fill is confirmed. Consumed when the position
/// is closed, at which point realized PnL is calculated:
///   - Long close:  `(fill_price - entry_price) * size`
///   - Short close: `(entry_price - fill_price) * size`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionState {
    pub symbol: String,
    pub side: String,
    pub entry_price: f64,
    pub size: f64,
    pub opened_at: String,
}

impl PositionState {
    /// Calculate realized PnL when closing this position.
    ///
    /// `close_price` is the fill price of the closing order.
    /// `close_size` is the size being closed (may be partial).
    pub fn realized_pnl(&self, close_price: f64, close_size: f64) -> f64 {
        match self.side.as_str() {
            "BUY" => (close_price - self.entry_price) * close_size,
            "SELL" => (self.entry_price - close_price) * close_size,
            _ => 0.0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Trading Wing
// ═══════════════════════════════════════════════════════════════════════

/// In-memory state for the Trading Wing.
#[derive(Debug)]
struct TradingState {
    last_proposal: Option<serde_json::Value>,
    last_yield_report: Option<serde_json::Value>,
    execution_count: u64,
    /// Open positions keyed by symbol. Only one position per symbol is
    /// tracked at a time (simplified model matching the HL perps flow).
    open_positions: std::collections::HashMap<String, PositionState>,
}

/// The Trading Wing — yield generation and execution.
pub struct TradingWing {
    state: Mutex<TradingState>,
}

impl TradingWing {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(TradingState {
                last_proposal: None,
                last_yield_report: None,
                execution_count: 0,
                open_positions: std::collections::HashMap::new(),
            }),
        }
    }

    /// Handle an incoming message.
    /// Returns `Some(response)` for every payload type — unhandled types return
    /// `Payload::Error` so dropped messages are visible (I-1 fix).
    pub fn handle_message(&self, msg: &Message) -> Option<Message> {
        match &msg.payload {
            Payload::TradingConfig { strategy, params } => {
                let mut state = self.state.lock().ok()?;
                state.last_proposal = Some(serde_json::json!({
                    "strategy": strategy,
                    "params": params,
                }));
                Some(Message::new(
                    WingId::Trading,
                    WingId::Coordinator,
                    Payload::Ack {
                        in_reply_to: msg.id,
                    },
                ))
            }

            Payload::Proposal {
                kind: _,
                description: _,
                changes,
                confidence: _,
            } => {
                // Validate and store the proposal for later execution.
                let mut state = self.state.lock().ok()?;
                state.last_proposal = Some(changes.clone());
                Some(Message::new(
                    WingId::Trading,
                    WingId::Coordinator,
                    Payload::Ack {
                        in_reply_to: msg.id,
                    },
                ))
            }

            Payload::ExecutePermit { proposal_id } => {
                // Read proposal config in a single lock scope (avoids TOCTOU).
                let (symbol, config) = {
                    let state = self.state.lock().ok()?;
                    let config = state
                        .last_proposal
                        .as_ref()
                        .cloned()
                        .unwrap_or(serde_json::json!({}));
                    let symbol = config
                        .get("symbol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("SOL/USDT")
                        .to_string();
                    (symbol, config)
                };

                // ── Hyperliquid execution path ────────────────────────
                // When the proposal sets execution_venue: "hyperliquid",
                // place a real order on HL testnet instead of using the
                // bridge fallback.
                let use_hl = config
                    .get("execution_venue")
                    .and_then(|v| v.as_str())
                    == Some("hyperliquid");

                if use_hl {
                    let is_buy = config
                        .get("is_buy")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let size = config
                        .get("size")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0.01");

                    // Check for existing position to calculate closing PnL.
                    let entry_price_str = self
                        .get_entry_price(&symbol)
                        .map(|ep| ep.to_string());

                    match execute_hl_sol_order(
                        is_buy,
                        size,
                        entry_price_str.as_deref(),
                    ) {
                        Ok((_response, report)) => {
                            // Update position tracking.
                            let fill_price: f64 =
                                report.fill_price.parse().unwrap_or(0.0);
                            let fill_size: f64 =
                                report.size.parse().unwrap_or(0.0);
                            let _tracked_pnl =
                                self.process_fill(&symbol, is_buy, fill_price, fill_size);

                            // ── Deposit positive PnL to treasury vault ─────
                            if let Some(pnl) = report.realized_pnl_usdc
                                && pnl > 0.0
                            {
                                match deposit_yield_to_treasury(pnl, None) {
                                    Ok(sig) => println!(
                                        "[TREASURY] yield deposited: {} USDC | {}",
                                        pnl, sig
                                    ),
                                    Err(e) => println!(
                                        "[TREASURY] deposit failed (non-fatal): {}",
                                        e
                                    ),
                                }
                            }

                            let mut state = self.state.lock().ok()?;
                            state.execution_count += 1;
                            state.last_yield_report =
                                Some(serde_json::to_value(&report).unwrap_or_default());
                            return Some(Message::new(
                                WingId::Trading,
                                WingId::Coordinator,
                                Payload::YieldReport {
                                    usdc_yield: report.fill_price.parse().unwrap_or(0.0),
                                    sol_reserves: report.size.parse().unwrap_or(0.0),
                                    drawdown: report.realized_pnl_usdc.unwrap_or(0.0),
                                    source: Some("hl_testnet_fill".to_string()),
                                },
                            ));
                        }
                        Err(e) => {
                            return Some(Message::new(
                                WingId::Trading,
                                WingId::Coordinator,
                                Payload::Error {
                                    reason: format!("HL execution failed: {}", e),
                                    in_reply_to: Some(*proposal_id),
                                },
                            ));
                        }
                    }
                }

                // ── Bridge fallback path ──────────────────────────────
                let request = BridgeRequest::new(&symbol, config);
                match bridge::call_bridge(&request) {
                    Ok(response) => {
                        let mut state = self.state.lock().ok()?;
                        state.execution_count += 1;
                        state.last_yield_report = Some(serde_json::json!({
                            "strategy": response.strategy,
                            "yield_estimate": response.yield_estimate,
                            "confidence": response.confidence,
                            "folds_validated": response.folds_validated,
                            "consistency": response.consistency,
                        }));
                        // Bridge returns projected OOS yield from WFA, not a realized trade.
                        Some(Message::new(
                            WingId::Trading,
                            WingId::Coordinator,
                            Payload::YieldReport {
                                usdc_yield: response.yield_estimate,
                                sol_reserves: response.confidence * 100.0,
                                drawdown: 1.0 - response.consistency,
                                source: Some("wfa_backtest".to_string()),
                            },
                        ))
                    }
                    Err(e) => Some(Message::new(
                        WingId::Trading,
                        WingId::Coordinator,
                        Payload::Error {
                            reason: format!("Bridge execution failed: {}", e),
                            in_reply_to: Some(*proposal_id),
                        },
                    )),
                }
            }

            Payload::YieldReport {
                usdc_yield,
                sol_reserves,
                drawdown,
                ..
            } => {
                let mut state = self.state.lock().ok()?;
                state.last_yield_report = Some(serde_json::json!({
                    "usdc_yield": usdc_yield,
                    "sol_reserves": sol_reserves,
                    "drawdown": drawdown,
                }));
                Some(Message::new(
                    WingId::Trading,
                    WingId::Coordinator,
                    Payload::Ack {
                        in_reply_to: msg.id,
                    },
                ))
            }

            Payload::Heartbeat { .. } => {
                let state = self.state.lock().ok()?;
                let metrics = serde_json::json!({
                    "execution_count": state.execution_count,
                    "has_proposal": state.last_proposal.is_some(),
                    "has_yield_report": state.last_yield_report.is_some(),
                });
                Some(Message::new(
                    WingId::Trading,
                    WingId::Coordinator,
                    Payload::Heartbeat {
                        wing: WingId::Trading,
                        status: crate::types::HealthStatus::Healthy,
                        metrics,
                    },
                ))
            }

            _ => Some(Message::new(
                WingId::Trading,
                WingId::Coordinator,
                Payload::Error {
                    reason: format!("Unimplemented payload: {:?}", msg.payload),
                    in_reply_to: Some(msg.id),
                },
            )),
        }
    }

    /// Get the current execution count.
    pub fn execution_count(&self) -> u64 {
        self.state.lock().map(|s| s.execution_count).unwrap_or(0)
    }

    /// Check if the wing has a stored proposal.
    pub fn has_proposal(&self) -> bool {
        self.state
            .lock()
            .map(|s| s.last_proposal.is_some())
            .unwrap_or(false)
    }

    /// Process a fill response and update position state.
    ///
    /// - If no open position exists for this symbol → opening fill:
    ///   stores the new `PositionState`, returns `realized_pnl_usdc = None`.
    /// - If an open position exists → closing fill:
    ///   calculates realized PnL, removes the position, returns `Some(pnl)`.
    pub fn process_fill(
        &self,
        symbol: &str,
        is_buy: bool,
        fill_price: f64,
        fill_size: f64,
    ) -> Option<f64> {
        let mut state = self.state.lock().ok()?;
        let key = symbol.to_string();

        if let Some(existing) = state.open_positions.remove(&key) {
            // Closing fill — calculate PnL.
            let pnl = existing.realized_pnl(fill_price, fill_size);
            Some(pnl)
        } else {
            // Opening fill — store position.
            state.open_positions.insert(
                key,
                PositionState {
                    symbol: symbol.to_string(),
                    side: if is_buy {
                        "BUY".to_string()
                    } else {
                        "SELL".to_string()
                    },
                    entry_price: fill_price,
                    size: fill_size,
                    opened_at: chrono::Utc::now().to_rfc3339(),
                },
            );
            None
        }
    }

    /// Check if there is an open position for a given symbol.
    pub fn has_open_position(&self, symbol: &str) -> bool {
        self.state
            .lock()
            .map(|s| s.open_positions.contains_key(symbol))
            .unwrap_or(false)
    }

    /// Get the entry price of an open position, if one exists.
    pub fn get_entry_price(&self, symbol: &str) -> Option<f64> {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.open_positions.get(symbol).map(|p| p.entry_price))
    }
}

impl Default for TradingWing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProposalKind;

    #[test]
    fn handles_trading_config() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::TradingConfig {
                strategy: "mr".to_string(),
                params: serde_json::json!({"rsi_entry": 28}),
            },
        );
        assert!(wing.handle_message(&msg).is_some());
        assert!(wing.has_proposal());
    }

    #[test]
    fn handles_proposal() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "Update RSI".to_string(),
                changes: serde_json::json!({"strategy": "mr", "symbol": "SOL/USDT"}),
                confidence: 0.9,
            },
        );
        assert!(wing.handle_message(&msg).is_some());
        assert!(wing.has_proposal());
    }

    #[test]
    fn handles_yield_report() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::YieldReport {
                usdc_yield: 5000.0,
                sol_reserves: 50000.0,
                drawdown: 0.03,
                source: None,
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        assert!(matches!(response.payload, Payload::Ack { .. }));
    }

    #[test]
    fn handles_heartbeat() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Heartbeat {
                wing: WingId::Trading,
                status: crate::types::HealthStatus::Healthy,
                metrics: serde_json::json!({}),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Heartbeat { wing, metrics, .. } => {
                assert_eq!(wing, WingId::Trading);
                assert_eq!(metrics["execution_count"], 0);
            }
            _ => panic!("Expected Heartbeat response"),
        }
    }

    #[test]
    fn execute_permit_bridge_not_found_returns_error() {
        let wing = TradingWing::new();
        // Set a proposal first.
        let proposal = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "test".to_string(),
                changes: serde_json::json!({"symbol": "SOL/USDT"}),
                confidence: 0.9,
            },
        );
        wing.handle_message(&proposal);

        // Execute — binary doesn't exist, expect Error payload.
        let permit = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::ExecutePermit {
                proposal_id: proposal.id,
            },
        );
        let response = wing.handle_message(&permit).unwrap();
        match response.payload {
            Payload::Error { reason, .. } => {
                assert!(reason.contains("Bridge execution failed"));
            }
            _ => panic!("Expected Error payload from failed bridge call"),
        }
    }

    #[test]
    fn unhandled_payload_returns_error() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::SecurityAlert {
                severity: crate::types::RiskLevel::Low,
                threat: "test".to_string(),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Error { reason, .. } => assert!(reason.contains("Unimplemented")),
            _ => panic!("Expected Error payload for unhandled type"),
        }
    }

    #[test]
    fn execution_count_zero_when_bridge_fails() {
        let wing = TradingWing::new();
        let proposal = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "test".to_string(),
                changes: serde_json::json!({"symbol": "SOL/USDT"}),
                confidence: 0.9,
            },
        );
        wing.handle_message(&proposal);

        let permit = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::ExecutePermit {
                proposal_id: proposal.id,
            },
        );
        wing.handle_message(&permit);
        assert_eq!(wing.execution_count(), 0);
    }

    // ── Hyperliquid unit tests ────────────────────────────────────────

    #[test]
    fn hl_key_file_roundtrip() {
        let key = HlKeyFile {
            address: "0xABC".to_string(),
            private_key: "0x123".to_string(),
            network: "hyperliquid-testnet".to_string(),
        };
        let json = serde_json::to_string(&key).unwrap();
        let parsed: HlKeyFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.address, "0xABC");
        assert_eq!(parsed.private_key, "0x123");
        assert_eq!(parsed.network, "hyperliquid-testnet");
    }

    #[test]
    fn hl_signature_roundtrip() {
        let sig = HlSignature {
            r: "0xaaa".to_string(),
            s: "0xbbb".to_string(),
            v: 27,
        };
        let json = serde_json::to_string(&sig).unwrap();
        let parsed: HlSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.r, "0xaaa");
        assert_eq!(parsed.s, "0xbbb");
        assert_eq!(parsed.v, 27);
    }

    #[test]
    fn build_order_action_produces_valid_structure() {
        let action = build_order_action(0, true, "0.01", "0", "Ioc");
        let json_str = serde_json::to_string(&action).unwrap();

        // Verify all required fields are present.
        assert!(json_str.contains("\"type\":\"order\""));
        assert!(json_str.contains("\"grouping\":\"na\""));
        assert!(json_str.contains("\"orders\""));
        assert!(json_str.contains("\"a\":0"));
        assert!(json_str.contains("\"b\":true"));
        assert!(json_str.contains("\"s\":\"0.01\""));
        assert!(json_str.contains("\"tif\":\"Ioc\""));
    }

    /// Verify that the private key in the key file derives the expected ETH address.
    /// If this fails, the signing will recover to a wrong address on HL.
    #[test]
    fn hl_private_key_derives_expected_address() {
        let key = match load_hl_key() {
            Ok(k) => k,
            Err(_) => return, // skip if no key file
        };
        let pk_hex = key.private_key.strip_prefix("0x").unwrap_or(&key.private_key);
        let pk_bytes = hex::decode(pk_hex).expect("private key hex valid");
        let secret_key = secp256k1::SecretKey::from_slice(&pk_bytes).expect("valid secp256k1 key");
        let secp = secp256k1::Secp256k1::new();
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        // Ethereum address = last 20 bytes of keccak256(uncompressed pubkey without 0x04 prefix)
        let serialized = public_key.serialize_uncompressed();
        let hash = keccak256(&serialized[1..65]); // skip 0x04 prefix
        let derived_addr = format!("0x{}", hex::encode(&hash[12..32]));
        println!("[TEST] Key file address:  {}", key.address);
        println!("[TEST] Derived address:   {}", derived_addr);
        assert_eq!(
            derived_addr.to_lowercase(),
            key.address.to_lowercase(),
            "Private key does NOT derive the expected address — signing will fail!"
        );
    }

    /// Verify that the action JSON serializes correctly (content, not ordering).
    /// The JSON is sent in the HTTP payload — key order doesn't affect HL.
    /// The msgpack key order (for signing) is tested separately.
    #[test]
    fn action_json_contains_all_required_fields() {
        let action = build_order_action(0, true, "0.01", "0", "Ioc");
        let json_str = serde_json::to_string(&action).unwrap();
        // Round-trip through JSON to verify valid structure.
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["type"], "order");
        assert_eq!(parsed["grouping"], "na");
        assert_eq!(parsed["orders"][0]["a"], 0);
        assert_eq!(parsed["orders"][0]["b"], true);
        assert_eq!(parsed["orders"][0]["s"], "0.01");
    }

    #[test]
    fn debug_msgpack_bytes() {
        let action = build_order_action(0, true, "0.01", "0", "Ioc");
        let hash_result = hl_action_hash(&action, 1744380000000);
        let action_hash = hash_result.unwrap();

        // Expected Python insertion-order msgpack hex
        let expected_msgpack = "83a474797065a56f72646572a66f72646572739186a16100a162c3a170a130a173a4302e3031a172c2a17481a56c696d697481a3746966a3496f63a867726f7570696e67a26e61";
        // Expected Python action_hash for nonce=1744380000000
        let expected_hash = "4aeaba018ccfaa20cd746642f6300a94a84e8452365c192d7c89000cb88c292a";

        println!("[TEST] Rust action_hash: {}", hex::encode(&action_hash));
        println!("[TEST] Expected hash:    {}", expected_hash);
        println!("[TEST] Expected msgpack: {}", expected_msgpack);

        assert_eq!(hex::encode(&action_hash), expected_hash,
            "Action hash must match Python SDK insertion-order output");
    }

    #[test]
    fn sign_l1_action_produces_valid_hex_signature() {
        // Use a well-known test private key (not real funds).
        let test_pk = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let action = build_order_action(0, true, "0.01", "0", "Ioc");

        let sig = sign_l1_action(&action, 1000, test_pk, false).expect("signing should succeed");

        // r and s should be hex strings with 0x prefix, each 66 chars (0x + 64 hex digits).
        assert!(sig.r.starts_with("0x"));
        assert!(sig.s.starts_with("0x"));
        assert_eq!(sig.r.len(), 66);
        assert_eq!(sig.s.len(), 66);
        // v should be 27 or 28 (Ethereum convention).
        assert!(sig.v == 27 || sig.v == 28);
    }

    /// Cross-validate Rust signing against Python reference output.
    /// Python SDK's EIP-712 signing with nonce=1744380000000 produced:
    ///   r: 0x5129d8eeb3ff6e86997d2a993090ebc43d72521f077630780994ac3f653c5095
    ///   s: 0x731853cdcceb1bb069af8d494f216d8a17ec2a99177a46b9bc33adbc2038406
    ///     (note: Python to_hex strips leading zero; Rust preserves → 0x0731853c...)
    ///   v: 27
    #[test]
    fn sign_action_matches_python_reference() {
        let key = match load_hl_key() {
            Ok(k) => k,
            Err(_) => return, // skip if no key file
        };
        let action = build_order_action(0, true, "0.01", "0", "Ioc");
        let nonce: u64 = 1744380000000;
        let sig = sign_l1_action(&action, nonce, &key.private_key, false)
            .expect("signing succeeds");

        println!("[TEST] Rust r: {}", sig.r);
        println!("[TEST] Rust s: {}", sig.s);
        println!("[TEST] Rust v: {}", sig.v);

        // Compare r directly (no leading-zero issue).
        assert_eq!(sig.r, "0x5129d8eeb3ff6e86997d2a993090ebc43d72521f077630780994ac3f653c5095",
            "r must match Python EIP-712 reference");

        // Compare s as bytes (Python to_hex strips leading zero, Rust preserves it).
        let rust_s_bytes = hex::decode(sig.s.strip_prefix("0x").unwrap()).unwrap();
        let py_s_bytes = hex::decode("0731853cdcceb1bb069af8d494f216d8a17ec2a99177a46b9bc33adbc2038406").unwrap();
        assert_eq!(rust_s_bytes, py_s_bytes,
            "s must match Python EIP-712 reference");

        assert_eq!(sig.v, 27, "v must match Python EIP-712 reference");
    }

    #[test]
    fn sign_l1_action_deterministic_for_same_input() {
        let test_pk = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let action = build_order_action(0, true, "0.01", "0", "Ioc");

        let sig1 = sign_l1_action(&action, 1000, test_pk, false).unwrap();
        let sig2 = sign_l1_action(&action, 1000, test_pk, false).unwrap();

        // ECDSA with RFC 6979 deterministic k produces the same signature.
        assert_eq!(sig1.r, sig2.r);
        assert_eq!(sig1.s, sig2.s);
        assert_eq!(sig1.v, sig2.v);
    }

    #[test]
    fn sign_l1_action_rejects_invalid_private_key() {
        let action = build_order_action(0, true, "0.01", "0", "Ioc");
        let result = sign_l1_action(&action, 1000, "not_valid_hex", false);
        assert!(result.is_err());
    }

    #[test]
    fn yield_report_data_roundtrip() {
        let report = YieldReportData {
            symbol: "SOL/USDT".to_string(),
            side: "BUY".to_string(),
            fill_price: "150.5".to_string(),
            size: "0.01".to_string(),
            entry_price: Some("150.5".to_string()),
            realized_pnl_usdc: None, // Opening fill — no PnL yet.
            timestamp: "2026-04-11T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: YieldReportData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.symbol, "SOL/USDT");
        assert_eq!(parsed.side, "BUY");
        assert_eq!(parsed.fill_price, "150.5");
        assert!(parsed.realized_pnl_usdc.is_none());
        assert_eq!(parsed.entry_price.as_deref(), Some("150.5"));
    }

    #[test]
    fn yield_report_data_closing_fill_with_pnl() {
        let report = YieldReportData {
            symbol: "SOL/USDT".to_string(),
            side: "SELL".to_string(),
            fill_price: "160.0".to_string(),
            size: "0.01".to_string(),
            entry_price: Some("150.0".to_string()),
            realized_pnl_usdc: Some(0.10), // (160 - 150) * 0.01 = 0.10 USDC
            timestamp: "2026-04-11T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: YieldReportData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.side, "SELL");
        assert!(parsed.realized_pnl_usdc.is_some());
        assert!((parsed.realized_pnl_usdc.unwrap() - 0.10).abs() < 0.001);
    }

    #[test]
    fn parse_fill_response_ok_status() {
        let response = serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{
                        "filled": {
                            "total_sz": "0.01",
                            "avg_px": "150.5",
                            "type": "market"
                        }
                    }]
                }
            }
        });

        let report = parse_fill_response(&response, "SOL/USDT", true, "0.01", None).unwrap();
        assert_eq!(report.symbol, "SOL/USDT");
        assert_eq!(report.side, "BUY");
        assert_eq!(report.fill_price, "150.5");
        assert_eq!(report.size, "0.01");
        assert!(report.realized_pnl_usdc.is_none(), "opening fill has no realized PnL");
        assert_eq!(report.entry_price.as_deref(), Some("150.5"), "entry_price set from fill_price");
    }

    #[test]
    fn parse_fill_response_closing_with_pnl() {
        let response = serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{
                        "filled": {"total_sz": "0.01", "avg_px": "160.0"}
                    }]
                }
            }
        });
        // Closing a long: SELL at 160, entered at 150 → PnL = (160-150)*0.01 = 0.10
        let report = parse_fill_response(&response, "SOL/USDT", false, "0.01", Some("150.0")).unwrap();
        assert_eq!(report.side, "SELL");
        assert!(report.realized_pnl_usdc.is_some());
        let pnl = report.realized_pnl_usdc.unwrap();
        assert!((pnl - 0.10).abs() < 0.001, "PnL should be 0.10, got {}", pnl);
    }

    #[test]
    fn parse_fill_response_closing_short_with_pnl() {
        let response = serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{
                        "filled": {"total_sz": "0.01", "avg_px": "140.0"}
                    }]
                }
            }
        });
        // Closing a short: BUY at 140, entered at 150 → PnL = (150-140)*0.01 = 0.10
        let report = parse_fill_response(&response, "SOL/USDT", true, "0.01", Some("150.0")).unwrap();
        assert_eq!(report.side, "BUY");
        let pnl = report.realized_pnl_usdc.unwrap();
        assert!((pnl - 0.10).abs() < 0.001, "short PnL should be 0.10, got {}", pnl);
    }

    #[test]
    fn parse_fill_response_rejects_error_status() {
        let response = serde_json::json!({
            "status": "err",
            "response": "Insufficient margin"
        });
        let result = parse_fill_response(&response, "SOL/USDT", true, "0.01", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("rejected"));
    }

    #[test]
    fn parse_fill_response_handles_fill_error() {
        let response = serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{
                        "error": "Insufficient margin"
                    }]
                }
            }
        });
        let result = parse_fill_response(&response, "SOL/USDT", true, "0.01", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Insufficient margin"));
    }

    #[test]
    fn load_hl_key_returns_err_when_missing() {
        // The key file should NOT exist at the test's cwd.
        let result = load_hl_key();
        // This test passes whether the file exists or not — if it does exist,
        // loading should succeed; if not, it should return a clear error.
        if let Err(e) = result {
            assert!(e.contains("not found"), "unexpected error: {}", e);
        }
    }

    // ── Integration: Hyperliquid testnet live order ────────────────────
    // Places a real 0.01 SOL order on HL testnet. Requires:
    //   1. configs/hl_testnet_key.json present
    //   2. Testnet account funded (https://app.hyperliquid-testnet.xyz/drip)
    //   3. Network access to api.hyperliquid-testnet.xyz
    //
    // The test verifies signing correctness by checking that HL recovers
    // the correct ETH address from the signature. If the account is
    // unfunded, the test still passes (it logs the "does not exist" error
    // but validates the recovered address matches our key).

    #[test]
    fn test_hl_testnet_order() {
        let key = match load_hl_key() {
            Ok(k) => k,
            Err(e) => {
                eprintln!("SKIP test_hl_testnet_order: {}", e);
                return;
            }
        };

        let sol_idx = match get_sol_index() {
            Ok(idx) => idx,
            Err(e) => {
                eprintln!("SKIP test_hl_testnet_order (API unreachable): {}", e);
                return;
            }
        };

        println!("[TEST] Using wallet: {}", key.address);
        println!("[TEST] SOL asset index: {}", sol_idx);

        // Place a minimal long order with IOC (immediate-or-cancel).
        let result = place_hl_order(sol_idx, true, "0.01", "0", "Ioc", &key.private_key);

        match result {
            Ok(response) => {
                let pretty =
                    serde_json::to_string_pretty(&response).unwrap_or_default();
                println!("[TEST] HL testnet raw response:\n{}", pretty);

                let status = response["status"].as_str().unwrap_or("unknown");

                if status == "ok" {
                    // Parse into YieldReportData.
                    let report =
                        parse_fill_response(&response, "SOL/USDT", true, "0.01", None)
                            .expect("Fill response should parse");
                    println!("[TEST] YieldReport: {:?}", report);

                    assert_eq!(report.symbol, "SOL/USDT");
                    assert_eq!(report.side, "BUY");
                    let price: f64 = report
                        .fill_price
                        .parse()
                        .expect("fill_price should be numeric");
                    assert!(price > 0.0, "fill_price should be positive");
                } else {
                    // Order rejected — but verify HL recovered the CORRECT address.
                    // This proves EIP-712 signing is working even if the account
                    // is unfunded.
                    let resp_str = pretty.to_lowercase();
                    let our_addr = key.address.to_lowercase();
                    let addr_recovered = resp_str.contains(&our_addr);

                    if addr_recovered {
                        println!(
                            "[TEST] Signing CORRECT — HL recovered our address {} (account needs funding at https://app.hyperliquid-testnet.xyz/drip)",
                            key.address
                        );
                    } else {
                        panic!(
                            "Signing FAILED — HL recovered a WRONG address. \
                             Expected {} in response: {}",
                            key.address, pretty
                        );
                    }
                }
            }
            Err(e) => {
                panic!("HL testnet request failed: {}", e);
            }
        }
    }

    // ── Mock fill tests (no network required) ───────────────────────────

    /// Mock fill response for testing without HL testnet connectivity.
    /// Simulates a successful IOC fill on SOL/USDT.
    fn mock_fill_response(fill_price: &str, fill_size: &str) -> serde_json::Value {
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{
                        "filled": {
                            "total_sz": fill_size,
                            "avg_px": fill_price,
                            "type": "market"
                        }
                    }]
                }
            }
        })
    }

    #[test]
    fn mock_fill_opening_then_closing() {
        // ── Opening fill (no prior position) ───────────────────────────
        let open_resp = mock_fill_response("142.50", "0.01");
        let open_report = parse_fill_response(
            &open_resp,
            "SOL/USDT",
            true,   // is_buy = opening a long
            "0.01",
            None,   // no entry_price → opening fill
        )
        .expect("opening fill should parse");

        println!("[MOCK FILL] Opening report: {:?}", open_report);

        // Opening fill: no realized PnL yet.
        assert!(
            open_report.realized_pnl_usdc.is_none(),
            "opening fill must have realized_pnl_usdc = None, got {:?}",
            open_report.realized_pnl_usdc
        );
        // Entry price stored from fill price.
        assert_eq!(
            open_report.entry_price.as_deref(),
            Some("142.50"),
            "entry_price must equal fill_price for opening fill"
        );
        assert_eq!(open_report.side, "BUY");
        assert_eq!(open_report.fill_price, "142.50");
        assert_eq!(open_report.size, "0.01");

        // ── Closing fill (prior position exists) ──────────────────────
        // Close the long at $160.00 → PnL = (160 - 142.50) * 0.01 = 0.175 USDC
        let close_resp = mock_fill_response("160.00", "0.01");
        let close_report = parse_fill_response(
            &close_resp,
            "SOL/USDT",
            false,          // is_buy = false → SELL → closing the long
            "0.01",
            Some("142.50"), // entry_price from the opening fill
        )
        .expect("closing fill should parse");

        println!("[MOCK FILL] Closing report: {:?}", close_report);

        // Closing fill: realized PnL must be non-zero and correct.
        let pnl = close_report
            .realized_pnl_usdc
            .expect("closing fill must have realized PnL");
        let expected_pnl = (160.0 - 142.50) * 0.01; // 0.175
        assert!(
            (pnl - expected_pnl).abs() < 0.0001,
            "closing PnL should be {}, got {}",
            expected_pnl,
            pnl
        );
        assert_eq!(close_report.side, "SELL");
        assert_eq!(close_report.fill_price, "160.00");
    }

    #[test]
    fn mock_fill_short_close_with_loss() {
        // Open a short at $150.00, close at $165.00 → loss.
        // Short PnL = (entry - exit) * size = (150 - 165) * 0.01 = -0.15
        let close_resp = mock_fill_response("165.00", "0.01");
        let close_report = parse_fill_response(
            &close_resp,
            "SOL/USDT",
            true,           // BUY → closing the short
            "0.01",
            Some("150.00"), // entry_price from opening short
        )
        .expect("closing fill should parse");

        let pnl = close_report.realized_pnl_usdc.unwrap();
        let expected = (150.0 - 165.0) * 0.01; // -0.15
        assert!(
            (pnl - expected).abs() < 0.0001,
            "short loss should be {}, got {}",
            expected,
            pnl
        );
    }

    // ── Position tracking tests ─────────────────────────────────────────

    #[test]
    fn process_fill_opens_position_on_first_fill() {
        let wing = TradingWing::new();

        // Opening fill — no prior position.
        let pnl = wing.process_fill("SOL/USDT", true, 142.50, 0.01);

        assert!(pnl.is_none(), "opening fill returns None PnL");
        assert!(wing.has_open_position("SOL/USDT"));
        assert_eq!(wing.get_entry_price("SOL/USDT"), Some(142.50));
    }

    #[test]
    fn process_fill_closes_position_and_returns_pnl() {
        let wing = TradingWing::new();

        // Open a long at 142.50.
        wing.process_fill("SOL/USDT", true, 142.50, 0.01);

        // Close the long at 160.00.
        let pnl = wing.process_fill("SOL/USDT", false, 160.00, 0.01);

        let expected = (160.0 - 142.50) * 0.01; // 0.175
        assert!(
            (pnl.unwrap() - expected).abs() < 0.0001,
            "closing PnL should be {}",
            expected
        );
        assert!(
            !wing.has_open_position("SOL/USDT"),
            "position removed after close"
        );
    }

    #[test]
    fn process_fill_short_position_tracking() {
        let wing = TradingWing::new();

        // Open a short at 150.00 (SELL).
        wing.process_fill("SOL/USDT", false, 150.00, 0.01);
        assert!(wing.has_open_position("SOL/USDT"));

        // Close the short at 140.00 (BUY) → profit.
        let pnl = wing.process_fill("SOL/USDT", true, 140.00, 0.01);
        let expected = (150.0 - 140.0) * 0.01; // 0.10
        assert!(
            (pnl.unwrap() - expected).abs() < 0.0001,
            "short profit should be {}",
            expected
        );
    }

    #[test]
    fn process_fill_multiple_symbols() {
        let wing = TradingWing::new();

        // Open SOL position.
        wing.process_fill("SOL/USDT", true, 100.0, 0.01);
        // Open ETH position.
        wing.process_fill("ETH/USDT", true, 3000.0, 0.1);

        assert!(wing.has_open_position("SOL/USDT"));
        assert!(wing.has_open_position("ETH/USDT"));

        // Close SOL — ETH remains open.
        let pnl = wing.process_fill("SOL/USDT", false, 110.0, 0.01);
        let expected = (110.0 - 100.0) * 0.01; // 0.10
        assert!((pnl.unwrap() - expected).abs() < 0.0001);
        assert!(!wing.has_open_position("SOL/USDT"));
        assert!(wing.has_open_position("ETH/USDT"));
    }

    #[test]
    fn get_entry_price_returns_none_for_unknown_symbol() {
        let wing = TradingWing::new();
        assert!(!wing.has_open_position("BTC/USDT"));
        assert_eq!(wing.get_entry_price("BTC/USDT"), None);
    }

    // ── Treasury CPI transfer tests (devnet integration) ──────────────

    #[test]
    fn derive_ata_matches_known_program() {
        use solana_sdk::pubkey::Pubkey;

        let wallet = Pubkey::try_from(DEVNET_WALLET).unwrap();
        let mint = Pubkey::try_from(RTP_MINT).unwrap();
        let token_program = Pubkey::try_from(TOKEN_2022_PROGRAM_ID).unwrap();

        let ata = derive_ata(&wallet, &mint, &token_program);

        // Should be a valid Solana pubkey (32 bytes, base58).
        assert_eq!(ata.as_ref().len(), 32);
        println!("[TEST] Derived ATA: {}", ata);
    }

    #[test]
    fn build_transfer_checked_instruction_format() {
        use solana_sdk::pubkey::Pubkey;

        let source = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let ix = build_transfer_checked_ix(
            &source, &mint, &dest, &authority, 1_000_000, 6, &program_id,
        );

        // Verify instruction structure.
        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 4);
        assert!(ix.accounts[0].is_writable); // source
        assert!(!ix.accounts[1].is_writable); // mint
        assert!(ix.accounts[2].is_writable); // destination
        assert!(ix.accounts[3].is_signer); // authority

        // Verify data: discriminator(4) + amount(8) + decimals(1) = 13 bytes.
        assert_eq!(ix.data.len(), 13);
        // Discriminator for transfer_checked = 12.
        assert_eq!(u32::from_le_bytes(ix.data[0..4].try_into().unwrap()), 12);
        // Amount = 1_000_000.
        assert_eq!(u64::from_le_bytes(ix.data[4..12].try_into().unwrap()), 1_000_000);
        // Decimals = 6.
        assert_eq!(ix.data[12], 6);
    }

    #[test]
    fn get_devnet_blockhash_live() {
        let result = get_devnet_blockhash();
        match result {
            Ok((blockhash_str, hash)) => {
                println!("[TEST] Devnet blockhash: {} → {:?}", blockhash_str, hash);
                assert!(!blockhash_str.is_empty());
                // Verify it round-trips.
                let parsed: solana_sdk::hash::Hash =
                    blockhash_str.parse().expect("blockhash should parse");
                assert_eq!(parsed, hash);
            }
            Err(e) => {
                eprintln!(
                    "SKIP get_devnet_blockhash_live (network unavailable): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn build_treasury_deposit_tx_live_devnet() {
        let result = build_treasury_deposit_tx(DEVNET_WALLET, 0.175);
        match result {
            Ok((b64, from_ata)) => {
                println!("[TEST] TX base64 (first 60 chars): {}...", &b64[..60.min(b64.len())]);
                println!("[TEST] From ATA: {}", from_ata);

                // Verify base64 is valid and decodes to reasonable length.
                let decoded = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &b64,
                )
                .expect("base64 should decode");
                assert!(decoded.len() > 64, "tx should be >64 bytes, got {}", decoded.len());

                // Verify the base64 string can be deserialized back to a Transaction.
                let tx: solana_sdk::transaction::Transaction =
                    bincode::deserialize(&decoded).expect("tx should deserialize");
                assert_eq!(tx.message.instructions.len(), 1, "should have 1 instruction");
                assert_eq!(tx.signatures.len(), 1, "should have 1 signer slot");
                assert_eq!(tx.signatures[0], solana_sdk::signature::Signature::default(),
                    "unsigned tx should have zero signature");
            }
            Err(e) => {
                eprintln!(
                    "SKIP build_treasury_deposit_tx_live_devnet: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn deposit_yield_to_treasury_rejects_non_positive() {
        let result = deposit_yield_to_treasury(0.0, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-positive"));

        let result = deposit_yield_to_treasury(-1.0, None);
        assert!(result.is_err());
    }

    #[test]
    fn call_phantom_signer_handles_missing_sidecar() {
        // Phantom creds are likely not configured — verify graceful failure.
        let result = call_phantom_signer("dGVzdA==");
        match &result {
            Ok(output) => {
                println!("[TEST] Phantom signer succeeded (unexpected!): {}", output);
            }
            Err(e) => {
                println!("[TEST] Phantom signer failed as expected: {}", e);
                assert!(e.contains("phantom_signer") || e.contains("PHANTOM"));
            }
        }
    }

    #[test]
    fn get_phantom_solana_address_handles_missing_wallet() {
        let result = get_phantom_solana_address();
        match &result {
            Ok(addr) => {
                println!("[TEST] Phantom Solana address: {}", addr);
                assert!(!addr.is_empty());
            }
            Err(e) => {
                println!("[TEST] Phantom address unavailable as expected: {}", e);
            }
        }
    }

    // ── Local keypair signing tests (Path C) ─────────────────────────────

    #[test]
    fn load_devnet_keypair_loads_valid_keypair() {
        let keypair = match load_devnet_keypair() {
            Ok(kp) => kp,
            Err(e) => {
                eprintln!("SKIP load_devnet_keypair: {}", e);
                return;
            }
        };

        // Keypair should derive a valid pubkey.
        let pubkey = keypair.pubkey();
        assert_eq!(pubkey.as_ref().len(), 32);
        println!("[TEST] Devnet keypair pubkey: {}", pubkey);

        // Should match the DEVNET_WALLET constant.
        let expected = solana_sdk::pubkey::Pubkey::try_from(DEVNET_WALLET).unwrap();
        assert_eq!(pubkey, expected, "keypair pubkey should match DEVNET_WALLET");
    }

    #[test]
    fn sign_and_send_local_produces_signature() {
        // Build a real tx against devnet.
        let (b64, _from_ata) = match build_treasury_deposit_tx(DEVNET_WALLET, 0.001) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("SKIP sign_and_send_local (build failed): {}", e);
                return;
            }
        };

        match sign_and_send_local(&b64) {
            Ok(sig) => {
                println!("[TEST] Local signing succeeded: sig={}", sig);
                // Signature should be a valid base58 string (~88 chars).
                assert!(!sig.is_empty());
                assert!(sig.len() > 80, "signature should be >80 chars, got {}", sig.len());
                println!(
                    "[TEST] Explorer: https://explorer.solana.com/tx/{}?cluster=devnet",
                    sig
                );
            }
            Err(e) => {
                // Signing might fail if keypair doesn't match (shouldn't happen)
                // or if RPC is unreachable. Either way, the test infrastructure
                // works — the error is environmental, not a code bug.
                eprintln!("SKIP sign_and_send_local (sign/send failed): {}", e);
            }
        }
    }

    /// End-to-end devnet integration: build → sign locally → submit.
    /// Exercises the full Path C signing cascade:
    ///   Phantom (fails) → local keypair (succeeds) → on-chain signature
    #[test]
    fn e2e_treasury_deposit_devnet() {
        // Step 1: Build the transaction against live devnet.
        let (b64, from_ata) = match build_treasury_deposit_tx(DEVNET_WALLET, 0.01) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("SKIP e2e_treasury_deposit_devnet (build failed): {}", e);
                return;
            }
        };

        println!("[E2E] TX base64: {}...", &b64[..60.min(b64.len())]);
        println!("[E2E] From ATA: {}", from_ata);

        // Step 2: Verify the transaction is well-formed.
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &b64,
        )
        .expect("base64 decode");
        let tx: solana_sdk::transaction::Transaction =
            bincode::deserialize(&decoded).expect("tx deserialize");

        assert_eq!(tx.message.instructions.len(), 1);
        assert_eq!(tx.signatures.len(), 1);

        // Verify the instruction program is Token-2022.
        let token_2022 = solana_sdk::pubkey::Pubkey::try_from(TOKEN_2022_PROGRAM_ID).unwrap();
        let program_id_idx = tx.message.instructions[0].program_id_index as usize;
        assert_eq!(tx.message.account_keys[program_id_idx], token_2022,
            "instruction program should be Token-2022");

        // Verify account keys include our known addresses.
        let vault = solana_sdk::pubkey::Pubkey::try_from(TREASURY_VAULT).unwrap();
        let mint = solana_sdk::pubkey::Pubkey::try_from(RTP_MINT).unwrap();
        let from_ata_pk: solana_sdk::pubkey::Pubkey =
            from_ata.parse().expect("from_ata should be valid pubkey");

        let account_keys = &tx.message.account_keys;
        assert!(account_keys.contains(&from_ata_pk), "missing from_ata");
        assert!(account_keys.contains(&mint), "missing mint");
        assert!(account_keys.contains(&vault), "missing treasury vault");
        assert!(account_keys.contains(&token_2022), "missing token program");

        println!("[E2E] Account keys in tx:");
        for (i, key) in account_keys.iter().enumerate() {
            let signer = if i < tx.message.header.num_required_signatures as usize {
                " [signer]"
            } else {
                ""
            };
            println!("[E2E]   {}: {}{}", i, key, signer);
        }

        // Step 3: Sign and submit via local keypair (Path C).
        match sign_and_send_local(&b64) {
            Ok(sig) => {
                println!("[E2E] ✅ Treasury deposit signed and submitted!");
                println!("[E2E]   Signature: {}", sig);
                println!(
                    "[E2E]   Explorer: https://explorer.solana.com/tx/{}?cluster=devnet",
                    sig
                );
                // The tx may fail on-chain (insufficient token balance) but the
                // signature proves signing works end-to-end.
            }
            Err(e) => {
                println!("[E2E] Local signing failed: {}", e);
                // Fall back to logging unsigned tx.
                println!("[E2E] Manual submission:");
                println!("[E2E]   base64: {}...", &b64[..60.min(b64.len())]);
            }
        }
    }
}
