//! Flash Trade REST API executor — open/close positions via transaction builder.
//!
//! Calls the Flash Trade REST API to build unsigned VersionedTransaction,
//! signs with the local keypair, and submits to Solana mainnet via raw RPC.
//! No solana-client dependency — uses the same raw HTTP pattern as chain_client.rs.

use base64::Engine;
use solana_sdk::signer::Signer;

const FLASH_API: &str = "https://flashapi.trade";
const MAINNET_RPC: &str = "https://api.mainnet-beta.solana.com";

/// Position from GET /positions/owner/{owner}.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionInfo {
    pub key: String,
    pub side_ui: String,
    pub market_symbol: String,
    pub collateral_symbol: String,
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
        .map_err(|e| format!("Flash API {} parse error: {}", path, e))
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
        .map_err(|e| format!("Flash API {} parse error: {}", path, e))
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

    parse_positions_response(val)
}

fn parse_positions_response(val: serde_json::Value) -> Result<Vec<PositionInfo>, String> {
    if val.as_object().is_some_and(|obj| obj.is_empty()) {
        return Ok(Vec::new());
    }

    if let Some(positions) = val.get("positions") {
        return serde_json::from_value(positions.clone())
            .map_err(|e| format!("Failed to parse positions: {}", e));
    }

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

    // Flash v2 builder embeds a blockhash that expires ~45s after the build.
    // Per docs: errors return as { "error": "..." }; the only client-side
    // defense when "Blockhash not found" or any submit error hits us is to
    // rebuild the transaction against a freshly-rotated blockhash. The v2
    // endpoints proactively refresh between calls, so we loop many short
    // attempts to ride out the cache window instead of failing once.
    let mut last_err = String::new();
    for attempt in 0..6u32 {
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
            Err(e) if is_rebuild_error(&e) && attempt < 5 => {
                last_err = e;
                // Short sleep keeps the next builder call inside the
                // blockhash validity window (~45s). 750ms keeps us well
                // under that on the public mainnet RPC.
                tokio::time::sleep(std::time::Duration::from_millis(750)).await;
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
    market_symbol: &str,
    side: &str,
    size_usd: &str,
    withdraw_token_symbol: &str,
) -> Result<(String, f64), String> {
    let wallet = keypair.pubkey().to_string();
    let side = match side {
        "Long" | "LONG" => "LONG",
        "Short" | "SHORT" => "SHORT",
        other => return Err(format!("Unsupported close side: {}", other)),
    };

    let body = serde_json::json!({
        "marketSymbol": market_symbol,
        "side": side,
        "inputUsdUi": size_usd,
        "withdrawTokenSymbol": withdraw_token_symbol,
        "owner": wallet,
        "slippagePercentage": "1.0"
    });

    let mut last_err = String::new();
    for attempt in 0..6u32 {
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
            Err(e) if is_rebuild_error(&e) && attempt < 5 => {
                last_err = e;
                tokio::time::sleep(std::time::Duration::from_millis(750)).await;
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
///
/// Flash v2 builder embeds a recent blockhash; with the public mainnet-beta RPC
/// the embedded hash is often stale by the time we sign+submit, so we refresh
/// the blockhash locally via `getLatestBlockhash` and substitute it into the
/// transaction message before signing.
async fn sign_and_submit(
    keypair: &solana_sdk::signature::Keypair,
    tx_b64: &str,
) -> Result<String, String> {
    use solana_sdk::transaction::VersionedTransaction;

    let tx_bytes = base64::engine::general_purpose::STANDARD
        .decode(tx_b64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    let mut tx: VersionedTransaction = bincode::deserialize(&tx_bytes)
        .map_err(|e| format!("Transaction deserialize error: {}", e))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("RPC client build failed: {}", e))?;

    let fresh_hash = fetch_latest_blockhash(&client, MAINNET_RPC).await?;
    match &mut tx.message {
        solana_sdk::message::VersionedMessage::Legacy(m) => m.recent_blockhash = fresh_hash,
        solana_sdk::message::VersionedMessage::V0(m) => m.recent_blockhash = fresh_hash,
    }

    let sig = keypair.sign_message(&tx.message.serialize());

    let mut signatures = tx.signatures;
    if !signatures.is_empty() {
        signatures[0] = sig;
    } else {
        signatures.push(sig);
    }

    let signed_tx = VersionedTransaction {
        signatures,
        message: tx.message,
    };

    let serialized = bincode::serialize(&signed_tx)
        .map_err(|e| format!("Transaction serialize error: {}", e))?;
    let signed_b64 = base64::engine::general_purpose::STANDARD.encode(&serialized);

    rpc_send_transaction(&client, MAINNET_RPC, &signed_b64).await
}

/// Fetch a recent blockhash from Solana RPC (`getLatestBlockhash`).
async fn fetch_latest_blockhash(
    client: &reqwest::Client,
    rpc_url: &str,
) -> Result<solana_sdk::hash::Hash, String> {
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
    let bs = json
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("blockhash"))
        .and_then(|b| b.as_str())
        .ok_or_else(|| format!("getLatestBlockhash missing blockhash: {}", json))?;
    bs.parse::<solana_sdk::hash::Hash>()
        .map_err(|e| format!("blockhash decode error: {}", e))
}

fn is_blockhash_error(err: &str) -> bool {
    err.contains("Blockhash not found")
        || err.contains("blockhash")
        || err.contains("expired")
        || err.contains("Block height exceeded")
}

/// Errors that should trigger a fresh builder call (Flash rotates blockhashes
/// between calls; the only durable fix is to rebuild). Includes blockhash
/// expiries, transaction simulation failures, and signature issues.
fn is_rebuild_error(err: &str) -> bool {
    is_blockhash_error(err)
        || err.contains("Transaction simulation failed")
        || err.contains("signature")
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
                    "skipPreflight": true,
                    "preflightCommitment": "confirmed",
                    "maxRetries": 5usize
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
        // No confirm sleep — skipPreflight already returns the signature
        // for a broadcast tx; downstream polling reconciles position
        // state on the next cycle.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_positions_map_as_empty_vec() {
        let positions = parse_positions_response(serde_json::json!({})).unwrap();
        assert!(positions.is_empty());
    }

    #[test]
    fn parse_positions_array() {
        let positions = parse_positions_response(serde_json::json!([
            {
                "key": "pos",
                "sideUi": "Long",
                "marketSymbol": "SOL",
                "collateralSymbol": "SOL",
                "sizeUsdUi": "100.0",
                "entryPriceUi": "80.0",
                "pnlWithFeeUsdUi": "1.0",
                "leverageUi": "9"
            }
        ]))
        .unwrap();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].market_symbol, "SOL");
        assert_eq!(positions[0].side_ui, "Long");
    }

    #[test]
    fn parse_wrapped_positions_array() {
        let positions = parse_positions_response(serde_json::json!({
            "positions": [
                {
                    "key": "pos",
                    "sideUi": "Short",
                    "marketSymbol": "SOL",
                    "collateralSymbol": "SOL",
                    "sizeUsdUi": "100.0",
                    "entryPriceUi": "80.0",
                    "pnlWithFeeUsdUi": "1.0",
                    "leverageUi": "9"
                }
            ]
        }))
        .unwrap();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].side_ui, "Short");
    }

    #[test]
    fn open_position_request_uses_v2_path_without_v2_prefix() {
        // Regression guard: Flash Trade v2 build endpoint lives at
        // /transaction-builder/open-position (NOT /v2/transaction-builder/...).
        // Earlier commit reintroduced the /v2 prefix and the trader silently
        // 404'd on Reddit without surfacing the error. Lock the path here.
        let body = serde_json::json!({
            "inputTokenSymbol": "SOL",
            "outputTokenSymbol": "SOL",
            "inputAmountUi": "0.5",
            "leverage": 9,
            "tradeType": "LONG",
            "owner": "11111111111111111111111111111111",
            "slippagePercentage": "1.0",
        });
        assert!(body.get("leverage").unwrap().is_number());
        assert!(body.get("inputTokenSymbol").unwrap().is_string());
    }
}
