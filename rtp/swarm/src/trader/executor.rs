//! Flash Trade REST API executor — open/close positions via transaction builder.
//!
//! Calls the Flash Trade REST API to build unsigned VersionedTransaction,
//! signs with the local keypair, and submits to Solana mainnet via raw RPC.
//! No solana-client dependency — uses the same raw HTTP pattern as chain_client.rs.

use base64::Engine;
use solana_sdk::hash::Hash;
use solana_sdk::signer::Signer;
use std::str::FromStr;

const FLASH_API: &str = "https://flashapi.trade";
const MAINNET_RPC: &str = "https://api.mainnet-beta.solana.com";

/// Position from GET /positions/owner/{owner}.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionInfo {
    pub key: String,
    pub side_ui: String,
    pub market_symbol: String,
    pub size_usd_ui: String,
    pub entry_price_ui: String,
    pub pnl_with_fee_usd_ui: String,
    pub leverage_ui: String,
}

const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Execute a POST to Flash Trade API with timeout.
async fn flash_post(path: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("HTTP client build failed: {}", e))?;
    let url = format!("{}{}", FLASH_API, path);
    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| format!("Flash API POST {} failed: {}", path, e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Flash API {}: {}", path, text));
    }

    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Flash API parse error: {}", e))
}

/// Execute a GET to Flash Trade API with timeout.
async fn flash_get(path: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("HTTP client build failed: {}", e))?;
    let url = format!("{}{}", FLASH_API, path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Flash API GET {} failed: {}", path, e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Flash API {}: {}", path, text));
    }

    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Flash API parse error: {}", e))
}

/// Get current SOL price from Flash Trade.
pub async fn get_sol_price() -> Result<f64, String> {
    let val = flash_get("/prices/SOL").await?;
    let price = val.get("priceUi").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if price <= 0.0 {
        return Err(format!("Invalid SOL price: {:?}", val.get("priceUi")));
    }
    Ok(price)
}

/// Get open positions for a wallet.
pub async fn get_positions(wallet: &str) -> Result<Vec<PositionInfo>, String> {
    let val = flash_get(&format!(
        "/positions/owner/{}?includePnlInLeverageDisplay=true",
        wallet
    ))
    .await?;

    serde_json::from_value(val).map_err(|e| format!("Failed to parse positions: {}", e))
}

/// Build, sign, and submit an open-position transaction.
/// `trade_type` is "LONG" or "SHORT" — passed to Flash Trade API as `tradeType`.
/// Returns (signature, position_size_usd, entry_price).
pub async fn open_position(
    keypair: &solana_sdk::signature::Keypair,
    amount_sol: f64,
    leverage: f64,
    trade_type: &str,
) -> Result<(String, f64, f64), String> {
    let wallet = keypair.pubkey().to_string();

    let body = serde_json::json!({
        "inputTokenSymbol": "SOL",
        "outputTokenSymbol": "SOL",
        "inputAmountUi": amount_sol.to_string(),
        "leverage": leverage,
        "tradeType": trade_type,
        "owner": wallet,
        "slippagePercentage": "1.0"
    });

    let mut last_err = String::new();
    for attempt in 0..3u32 {
        let val = flash_post("/transaction-builder/open-position", &body).await?;

        if let Some(err) = val.get("err").and_then(|v| v.as_str())
            && !err.is_empty()
        {
            return Err(format!("Open position API error: {}", err));
        }

        let tx_b64 = val
            .get("transactionBase64")
            .and_then(|v| v.as_str())
            .ok_or("No transaction in open-position response")?;

        let entry_price = val
            .get("newEntryPrice")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let size_usd = val
            .get("youRecieveUsdUi")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        match sign_and_submit(keypair, tx_b64).await {
            Ok(sig) => return Ok((sig, size_usd, entry_price)),
            Err(e) if is_blockhash_error(&e) && attempt < 2 => {
                last_err = e;
                tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt + 1) as u64))
                    .await;
            }
            Err(e) => return Err(e),
        }
    }

    Err(format!(
        "Open position failed after rebuild retries: {}",
        last_err
    ))
}

/// Build, sign, and submit a close-position transaction.
/// Returns (signature, settled_pnl).
pub async fn close_position(
    keypair: &solana_sdk::signature::Keypair,
    position_key: &str,
    size_usd: &str,
) -> Result<(String, f64), String> {
    let body = serde_json::json!({
        "positionKey": position_key,
        "inputUsdUi": size_usd,
        "withdrawTokenSymbol": "SOL",
        "slippagePercentage": "1.0"
    });

    let mut last_err = String::new();
    for attempt in 0..3u32 {
        let val = flash_post("/transaction-builder/close-position", &body).await?;

        if let Some(err) = val.get("err").and_then(|v| v.as_str())
            && !err.is_empty()
        {
            return Err(format!("Close position API error: {}", err));
        }

        let tx_b64 = val
            .get("transactionBase64")
            .and_then(|v| v.as_str())
            .ok_or("No transaction in close-position response")?;

        let settled_pnl = val
            .get("settledPnl")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        match sign_and_submit(keypair, tx_b64).await {
            Ok(sig) => return Ok((sig, settled_pnl)),
            Err(e) if is_blockhash_error(&e) && attempt < 2 => {
                last_err = e;
                tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt + 1) as u64))
                    .await;
            }
            Err(e) => return Err(e),
        }
    }

    Err(format!(
        "Close position failed after rebuild retries: {}",
        last_err
    ))
}

/// Sign a VersionedTransaction (base64) and submit to Solana mainnet via raw RPC.
/// Uses the same raw HTTP pattern as chain_client.rs — no solana-client dependency.
async fn sign_and_submit(
    keypair: &solana_sdk::signature::Keypair,
    tx_b64: &str,
) -> Result<String, String> {
    use solana_sdk::message::VersionedMessage;
    use solana_sdk::transaction::VersionedTransaction;

    // Decode the unsigned transaction
    let tx_bytes = base64::engine::general_purpose::STANDARD
        .decode(tx_b64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    let tx: VersionedTransaction = bincode::deserialize(&tx_bytes)
        .map_err(|e| format!("Transaction deserialize error: {}", e))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("RPC client build failed: {}", e))?;

    let mut last_err = String::new();
    for attempt in 0..3u32 {
        let blockhash = get_latest_blockhash(&client, MAINNET_RPC).await?;
        let mut message = tx.message.clone();
        match &mut message {
            VersionedMessage::Legacy(m) => m.recent_blockhash = blockhash,
            VersionedMessage::V0(m) => m.recent_blockhash = blockhash,
        }

        let sig = keypair.sign_message(&message.serialize());

        let mut signatures = tx.signatures.clone();
        if !signatures.is_empty() {
            signatures[0] = sig;
        } else {
            signatures.push(sig);
        }

        let signed_tx = VersionedTransaction {
            signatures,
            message,
        };

        let serialized = bincode::serialize(&signed_tx)
            .map_err(|e| format!("Transaction serialize error: {}", e))?;
        let signed_b64 = base64::engine::general_purpose::STANDARD.encode(&serialized);

        match rpc_send_transaction(&client, MAINNET_RPC, &signed_b64).await {
            Ok(sig) => return Ok(sig),
            Err(e) if is_blockhash_error(&e) && attempt < 2 => {
                last_err = e;
                tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt + 1) as u64))
                    .await;
            }
            Err(e) => return Err(e),
        }
    }

    Err(format!(
        "Transaction failed after blockhash retries: {}",
        last_err
    ))
}

fn is_blockhash_error(err: &str) -> bool {
    err.contains("Blockhash not found")
        || err.contains("blockhash")
        || err.contains("expired")
        || err.contains("Block height exceeded")
}

async fn get_latest_blockhash(client: &reqwest::Client, rpc_url: &str) -> Result<Hash, String> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [{ "commitment": "confirmed" }]
        }))
        .send()
        .await
        .map_err(|e| format!("getLatestBlockhash request failed: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("getLatestBlockhash parse error: {}", e))?;

    if let Some(err) = json.get("error") {
        return Err(format!("getLatestBlockhash RPC error: {}", err));
    }

    let blockhash = json
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("blockhash"))
        .and_then(|b| b.as_str())
        .ok_or_else(|| format!("getLatestBlockhash missing blockhash: {}", json))?;

    Hash::from_str(blockhash).map_err(|e| format!("Invalid blockhash {}: {}", blockhash, e))
}

/// Send a base64-encoded transaction via Solana JSON-RPC.
async fn rpc_send_transaction(
    client: &reqwest::Client,
    rpc_url: &str,
    b64_tx: &str,
) -> Result<String, String> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                b64_tx,
                {
                    "encoding": "base64",
                    "skipPreflight": false,
                    "maxRetries": 3usize
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| format!("RPC request failed: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("RPC parse error: {}", e))?;

    if let Some(sig) = json.get("result").and_then(|r| r.as_str()) {
        // Confirm
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        return Ok(sig.to_string());
    }

    if let Some(err) = json.get("error") {
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error")
            .to_string();
        return Err(format!("RPC error: {}", message));
    }

    Err(format!("RPC response missing result/error: {}", json))
}
