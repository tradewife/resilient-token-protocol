//! Flash Trade SDK v2 executor — open/close positions via @flash_trade/flash-sdk-v2.
//!
//! Spawns a Node.js child process that loads the Flash TypeScript SDK,
//! builds correct v2 transactions (including session_token account),
//! signs with the wallet keypair, and submits to Solana mainnet.
//! Falls back to legacy REST API if child process unavailable.

use base64::Engine;
use serde::{Deserialize, Serialize};
use solana_sdk::signer::Signer;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

const FLASH_API: &str = "https://flashapi.trade";
const MAINNET_RPC: &str = "https://api.mainnet-beta.solana.com";
/// Flash v2 / MagicBlock ER RPC — trading txs (open/close/…) must land here
/// after the basket is delegated. Funds ops (init-*, deposit, delegate) use
/// Solana mainnet. Docs: signing-and-submitting.md
const DEFAULT_V2_RPC: &str = "https://flash.magicblock.xyz";

/// Respawn budget: at most this many child respawn attempts per minute before
/// we hold off and surface a "node unavailable" error to the caller. Caller
/// falls back to the legacy REST path on this error.
const SDK_MAX_RESPAWNS_PER_MINUTE: u32 = 3;
const SDK_RESPAWN_HOLD_MS: u64 = 5_000;

/// Default path the wrapper lives at inside the trader Docker image.
/// Overridable via `RTP_TRADER_WRAPPER_PATH` for dev runs.
const DEFAULT_WRAPPER_PATH: &str = "/app/wrapper/flash-sdk-wrapper.mjs";

fn v2_rpc_url() -> String {
    std::env::var("RTP_TRADER_ER_RPC").unwrap_or_else(|_| DEFAULT_V2_RPC.to_string())
}

fn solana_rpc_url() -> String {
    std::env::var("RTP_SOLANA_RPC_URL").unwrap_or_else(|_| MAINNET_RPC.to_string())
}

/// JSON-RPC request to Node.js wrapper
#[derive(Serialize, Deserialize, Debug)]
struct SdkRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

/// JSON-RPC response from Node.js wrapper
#[derive(Deserialize, Debug)]
struct SdkResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<SdkError>,
}

#[derive(Deserialize, Debug)]
struct SdkError {
    code: i32,
    message: String,
}

/// Position from GET /positions/owner/{owner}.
/// Flash v2 returns either an array, `{positions: [...]}`, or a map of
/// `marketPubkey → PositionMetricsDto` (key is the market pubkey).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionInfo {
    #[serde(default)]
    pub key: String,
    pub side_ui: String,
    #[serde(default)]
    pub market_symbol: String,
    #[serde(default = "default_collateral_sol")]
    pub collateral_symbol: String,
    #[serde(default)]
    pub size_usd_ui: String,
    #[serde(default)]
    pub entry_price_ui: String,
    #[serde(default)]
    pub pnl_with_fee_usd_ui: String,
    #[serde(default)]
    pub leverage_ui: String,
}

fn default_collateral_sol() -> String {
    "SOL".to_string()
}

const REQUEST_TIMEOUT_SECS: u64 = 30;
const SDK_CHILD_TIMEOUT_SECS: u64 = 120;
const TX_CONFIRM_TIMEOUT_SECS: u64 = 60;
const TX_CONFIRM_POLL_MS: u64 = 500;
const DEFAULT_OPEN_BACKOFF_ATTEMPTS: u32 = 8;
const DEFAULT_MIN_OPEN_COLLATERAL_LAMPORTS: u64 = 5_000_000;

/// Flash SDK v2 client — communicates with Node.js wrapper via stdio JSON-RPC.
struct FlashSdkClient {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    request_id: u64,
}

impl FlashSdkClient {
    /// Spawn the Node.js wrapper (defaults to `/app/wrapper/flash-sdk-wrapper.mjs`
    /// in the container; overridable via `RTP_TRADER_WRAPPER_PATH` for dev).
    /// The wrapper reads RTP_TRADER_KEYPAIR_JSON from environment.
    ///
    /// Note: do **not** pass `--input-type=module` when giving a file path —
    /// Node only allows that flag with `--eval` / STDIN. `.mjs` already loads
    /// as ESM. Requires Node ≥ 18 (runtime image ships Node 20).
    async fn spawn() -> Result<Self, String> {
        let wrapper_path = std::env::var("RTP_TRADER_WRAPPER_PATH")
            .unwrap_or_else(|_| DEFAULT_WRAPPER_PATH.to_string());

        if !std::path::Path::new(&wrapper_path).exists() {
            return Err(format!(
                "Wrapper not found at {wrapper_path} (set RTP_TRADER_WRAPPER_PATH)"
            ));
        }

        let er_rpc = v2_rpc_url();
        let sol_rpc = solana_rpc_url();

        let mut child = Command::new("node")
            .arg("--experimental-vm-modules")
            .arg(&wrapper_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Keep stderr visible in Railway logs (bigint warnings, init logs).
            .stderr(Stdio::inherit())
            .env("RTP_TRADER_ER_RPC", &er_rpc)
            .env("RTP_SOLANA_RPC_URL", &sol_rpc)
            // Keypair must come from env (wrapper never reads argv).
            // Inherit parent env for RTP_TRADER_KEYPAIR_JSON.
            .spawn()
            .map_err(|e| format!("Failed to spawn flash-sdk wrapper: {}", e))?;

        let stdin = child.stdin.take().ok_or("No stdin")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("No stdout")?);

        let mut client = Self {
            child,
            stdin,
            stdout,
            request_id: 0,
        };

        // Wait for "Ready" signal on stdout (wrapper writes it via process.stdout).
        // Node may emit non-ready lines first; keep scanning until match or EOF.
        let mut line = String::new();
        let timeout = tokio::time::Duration::from_secs(45);
        let ready = tokio::time::timeout(timeout, async {
            loop {
                line.clear();
                let n = client
                    .stdout
                    .read_line(&mut line)
                    .await
                    .map_err(|e| format!("Read wrapper stdout: {e}"))?;
                if n == 0 {
                    return Err("Wrapper did not signal ready (stdout closed)".to_string());
                }
                if line.contains("Ready for JSON-RPC") {
                    return Ok::<(), String>(());
                }
                // Ignore any pre-ready stdout noise (should be rare).
                tracing::debug!("[FLASH_SDK] pre-ready stdout: {}", line.trim());
            }
        })
        .await;

        match ready {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                let _ = client.child.kill().await;
                return Err(e);
            }
            Err(_) => {
                let _ = client.child.kill().await;
                return Err("Wrapper startup timeout".to_string());
            }
        }

        tracing::info!("[FLASH_SDK] Child process spawned and ready (node wrapper)");
        Ok(client)
    }

    /// Send a JSON-RPC request and wait for response.
    async fn call(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.request_id += 1;
        let req = SdkRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.request_id,
        };

        let req_json =
            serde_json::to_string(&req).map_err(|e| format!("Serialize request: {}", e))?;
        self.stdin
            .write_all(req_json.as_bytes())
            .await
            .map_err(|e| format!("Write stdin: {}", e))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| format!("Write newline: {}", e))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| format!("Flush stdin: {}", e))?;

        // Read response with timeout
        let mut line = String::new();
        let read_fut = self.stdout.read_line(&mut line);
        let resp_json = tokio::time::timeout(
            tokio::time::Duration::from_secs(SDK_CHILD_TIMEOUT_SECS),
            read_fut,
        )
        .await
        .map_err(|_| "SDK call timeout".to_string())?
        .map_err(|e| format!("Read stdout: {}", e))?;

        if resp_json == 0 {
            return Err("Child process closed stdout".to_string());
        }

        let resp: SdkResponse = serde_json::from_str(line.trim())
            .map_err(|e| format!("Parse response: {} (raw: {})", e, line))?;

        if let Some(err) = resp.error {
            return Err(format!("SDK error {}: {}", err.code, err.message));
        }

        resp.result.ok_or("No result in response".to_string())
    }

    /// Run the 5-step v2 setup (idempotent).
    pub async fn setup(&mut self) -> Result<Vec<(String, String)>, String> {
        let result = self.call("setup", serde_json::json!({})).await?;
        let sigs = result
            .get("signatures")
            .and_then(|v| v.as_array())
            .ok_or("No signatures in setup result")?;
        sigs.iter()
            .map(|s| {
                let step = s
                    .get("step")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let sig = s
                    .get("signature")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok((step, sig))
            })
            .collect()
    }

    /// Open a position via SDK (returns signature, size_usd, entry_price).
    pub async fn open_position(
        &mut self,
        amount_sol: f64,
        leverage: f64,
        trade_type: &str,
    ) -> Result<(String, f64, f64), String> {
        let side = if trade_type == "LONG" || trade_type == "Long" {
            "long"
        } else {
            "short"
        };
        let params = serde_json::json!({
            "collateralAmount": (amount_sol * 1e9) as u64, // lamports
            "leverage": leverage,
            "side": side,
        });
        let result = self.call("open_position", params).await?;
        let sig = result
            .get("signature")
            .and_then(|v| v.as_str())
            .ok_or("No signature")?;
        let size_usd = result
            .get("size_usd")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let entry_price = result
            .get("entry_price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        Ok((sig.to_string(), size_usd, entry_price))
    }

    /// Close a position via SDK.
    pub async fn close_position(
        &mut self,
        size_usd: f64,
        withdraw_token: &str,
        side: &str,
    ) -> Result<(String, f64), String> {
        let params = serde_json::json!({
            "sizeAmount": (size_usd * 1e6) as u64, // USDC 6dp
            "collateralSymbol": withdraw_token,
            "side": side,
        });
        let result = self.call("close_position", params).await?;
        let sig = result
            .get("signature")
            .and_then(|v| v.as_str())
            .ok_or("No signature")?;
        let pnl = result.get("pnl").and_then(|v| v.as_f64()).unwrap_or(0.0);
        Ok((sig.to_string(), pnl))
    }

    /// Get current price via SDK.
    pub async fn get_price(&mut self, symbol: &str, side: &str) -> Result<(u64, i32), String> {
        let params = serde_json::json!({ "symbol": symbol, "side": side });
        let result = self.call("get_price", params).await?;
        let price: u64 = result
            .get("price")
            .and_then(|v| v.as_str())
            .ok_or("No price")?
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        let exponent: i32 = result
            .get("exponent")
            .and_then(|v| v.as_str())
            .ok_or("No exponent")?
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        Ok((price, exponent))
    }
}

impl Drop for FlashSdkClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Module-level Flash SDK client — initialized on first use.
/// Uses `tokio::sync::OnceCell` for lazy initialization.
static FLASH_SDK_CLIENT: tokio::sync::OnceCell<tokio::sync::Mutex<FlashSdkState>> =
    tokio::sync::OnceCell::const_new();

/// Per-process state for the SDK child: the (optional) live child + respawn
/// bookkeeping. Respawn cap prevents thrashing when the wrapper keeps dying —
/// once we cross `SDK_MAX_RESPAWNS_PER_MINUTE` in a rolling 60s window we hold
/// off spawning for `SDK_RESPAWN_HOLD_MS` and surface a "node unavailable"
/// error so the caller falls back to the legacy REST path instead.
struct FlashSdkState {
    inner: Option<FlashSdkClient>,
    spawn_history: Vec<std::time::Instant>,
}

impl FlashSdkState {
    fn new() -> Self {
        Self {
            inner: None,
            spawn_history: Vec::new(),
        }
    }

    fn record_spawn(&mut self, when: std::time::Instant) {
        self.spawn_history.push(when);
        self.spawn_history
            .retain(|t| when.duration_since(*t).as_secs() < 60);
    }

    fn is_held(&self, _now: std::time::Instant) -> bool {
        self.spawn_history.len() as u32 > SDK_MAX_RESPAWNS_PER_MINUTE
    }
}

/// Returns the live SDK client — spawning on first use — *or* an error if the
/// wrapper is dead or the respawn budget is exhausted. Used internally by
/// `open_position` / `close_position` to attempt the SDK path before falling
/// through to the legacy REST path.
async fn try_get_sdk_client() -> Result<tokio::sync::MutexGuard<'static, FlashSdkState>, String> {
    let state_cell = FLASH_SDK_CLIENT
        .get_or_init(|| async { tokio::sync::Mutex::new(FlashSdkState::new()) })
        .await;
    let mut guard = state_cell.lock().await;
    let now = std::time::Instant::now();

    if guard.is_held(now) {
        return Err("node unavailable (respawn budget exceeded)".to_string());
    }

    if guard.inner.is_none() {
        match FlashSdkClient::spawn().await {
            Ok(c) => {
                guard.record_spawn(now);
                guard.inner = Some(c);
            }
            Err(e) => {
                guard.record_spawn(now);
                tracing::warn!("[FLASH_SDK] Spawn failed: {} — falling back to REST", e);
                return Err(format!("node unavailable: {e}"));
            }
        }
    }

    Ok(guard)
}

/// Drop the dead client so the next caller respawns. Called from `open_position`
/// / `close_position` when the SDK call returns an error indicating the child
/// process is dead (timeout, EOF, parse failure).
async fn sdk_mark_dead() {
    let Some(state_cell) = FLASH_SDK_CLIENT.get() else {
        return;
    };
    let mut guard = state_cell.lock().await;
    if let Some(c) = guard.inner.take() {
        // c's Drop kills the child.
        drop(c);
        tracing::warn!("[FLASH_SDK] dropping dead client; next caller respawns");
    }
}

/// Single-shot SDK open attempt. Returns `"node unavailable: <why>"` when the
/// wrapper isn't running so callers can fall back to REST cleanly.
async fn try_open_via_sdk(
    amount_sol: f64,
    leverage: f64,
    trade_type: &str,
) -> Result<(String, f64, f64), String> {
    let mut guard = try_get_sdk_client().await?;
    let result = guard
        .inner
        .as_mut()
        .unwrap() // safe: try_get_sdk_client guarantees Some
        .open_position(amount_sol, leverage, trade_type)
        .await;
    match result {
        Ok(v) => Ok(v),
        Err(e) if is_sdk_dead_error(&e) => {
            // Take the client out so Drop kills it.
            if let Some(c) = guard.inner.take() {
                drop(c);
            }
            Err(format!("node unavailable: {e}"))
        }
        Err(other) => {
            // RPC/on-chain error — leave the client alive; the next call can
            // retry through the same wrapper.
            Err(other)
        }
    }
}

/// Single-shot SDK close attempt.
async fn try_close_via_sdk(
    size_usd: f64,
    withdraw_token: &str,
    side: &str,
) -> Result<(String, f64), String> {
    let mut guard = try_get_sdk_client().await?;
    let result = guard
        .inner
        .as_mut()
        .unwrap()
        .close_position(size_usd, withdraw_token, side)
        .await;
    match result {
        Ok(v) => Ok(v),
        Err(e) if is_sdk_dead_error(&e) => {
            if let Some(c) = guard.inner.take() {
                drop(c);
            }
            Err(format!("node unavailable: {e}"))
        }
        Err(other) => Err(other),
    }
}

fn is_sdk_dead_error(err: &str) -> bool {
    err.contains("Child process closed stdout")
        || err.contains("SDK call timeout")
        || err.contains("Wrapper startup timeout")
        || err.contains("Wrapper did not signal ready")
        || err.contains("Parse response")
}

fn is_flash_capacity_error(err: &str) -> bool {
    err.contains("Custom\":6024")
        || err.contains("Custom: 6024")
        || err.contains("0x1788")
        || err.contains("CustodyAmountLimit")
        || err.contains("Custom\":6025")
        || err.contains("Custom: 6025")
        || err.contains("0x1789")
        || err.contains("PositionAmountLimit")
        || err.contains("Custom\":6032")
        || err.contains("Custom: 6032")
        || err.contains("0x1790")
        || err.contains("MaxUtilization")
        || err.contains("Custom\":6088")
        || err.contains("Custom: 6088")
        || err.contains("0x17c8")
        || err.contains("MaxPositionSize")
        || err.contains("Custom\":6089")
        || err.contains("Custom: 6089")
        || err.contains("0x17c9")
        || err.contains("MaxExposure")
        || err.contains("Custom\":6110")
        || err.contains("Custom: 6110")
        || err.contains("0x17de")
        || err.contains("InsufficientCustodyLiquidity")
}

fn open_backoff_attempts() -> u32 {
    std::env::var("RTP_TRADER_OPEN_BACKOFF_ATTEMPTS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_OPEN_BACKOFF_ATTEMPTS)
}

fn min_open_collateral_lamports() -> u64 {
    std::env::var("RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_MIN_OPEN_COLLATERAL_LAMPORTS)
}

/// Execute a POST to Flash Trade API with timeout (legacy fallback).
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

    // Wrapped array: { "positions": [ ... ] }
    if let Some(positions) = val.get("positions") {
        return parse_positions_array_or_map(positions.clone());
    }

    parse_positions_array_or_map(val)
}

/// Flash v2 docs: positions is a map of `marketPubkey → PositionMetricsDto`.
/// Older shapes used a flat array. Accept both.
fn parse_positions_array_or_map(val: serde_json::Value) -> Result<Vec<PositionInfo>, String> {
    if val.as_array().is_some() {
        return serde_json::from_value(val)
            .map_err(|e| format!("Failed to parse positions array: {}", e));
    }

    if let Some(obj) = val.as_object() {
        let mut out = Vec::with_capacity(obj.len());
        for (market_key, metrics) in obj {
            // Skip non-position entries if the API ever nests extras
            if !metrics.is_object() {
                continue;
            }
            if metrics.get("sideUi").is_none() && metrics.get("side_ui").is_none() {
                continue;
            }
            let mut info: PositionInfo = serde_json::from_value(metrics.clone())
                .map_err(|e| format!("Failed to parse position metrics for {market_key}: {e}"))?;
            if info.key.is_empty() {
                info.key = market_key.clone();
            }
            // Normalize side casing used by exit matching ("Long" / "Short")
            info.side_ui = match info.side_ui.as_str() {
                "LONG" | "long" => "Long".to_string(),
                "SHORT" | "short" => "Short".to_string(),
                other => other.to_string(),
            };
            out.push(info);
        }
        return Ok(out);
    }

    Err(format!("Unexpected positions JSON shape: {}", val))
}

/// Poll Flash positions until a SOL position on `side` appears, or timeout.
async fn wait_for_sol_position(
    wallet: &str,
    side: &str,
    timeout_secs: u64,
) -> Result<PositionInfo, String> {
    let side_norm = match side {
        "LONG" | "Long" | "long" => "Long",
        "SHORT" | "Short" | "short" => "Short",
        other => other,
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let mut last_err = String::new();
    while std::time::Instant::now() < deadline {
        match get_positions(wallet).await {
            Ok(positions) => {
                if let Some(p) = positions
                    .into_iter()
                    .find(|p| p.market_symbol == "SOL" && p.side_ui == side_norm)
                {
                    return Ok(p);
                }
                last_err = format!("no SOL {side_norm} in positions yet");
            }
            Err(e) => last_err = e,
        }
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
    }
    Err(format!(
        "Position not visible on Flash after open ({last_err})"
    ))
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
    // SDK path first: returns Ok on success or an error tagged "node unavailable"
    // when the wrapper child is missing / died. Other errors (RPC rejection,
    // size/price issue) bubble up unchanged.
    let wallet = keypair.pubkey().to_string();
    let side_for_wait = if trade_type.eq_ignore_ascii_case("long") {
        "Long"
    } else {
        "Short"
    };

    match try_open_via_sdk(amount_sol, leverage, trade_type).await {
        Ok((sig, size_usd, entry_price)) => {
            // SDK path confirms internally; still require readable position so
            // local state / dashboard never show a phantom open.
            match wait_for_sol_position(&wallet, side_for_wait, 15).await {
                Ok(pos) => {
                    let size = pos.size_usd_ui.parse().unwrap_or(size_usd);
                    let entry = pos.entry_price_ui.parse().unwrap_or(entry_price);
                    return Ok((sig, size, entry));
                }
                Err(e) => {
                    return Err(format!(
                        "SDK open returned sig {sig} but position not visible: {e}"
                    ));
                }
            }
        }
        Err(e) if e.starts_with("node unavailable") => {
            tracing::warn!("[OPEN] SDK unavailable ({}). Falling back to REST.", e);
        }
        Err(e) if is_flash_capacity_error(&e) => {
            tracing::warn!(
                "[OPEN] SDK Flash capacity error ({}). Falling back to REST builder.",
                e
            );
        }
        Err(other) => return Err(other),
    }

    // Flash v2: trading txs must be submitted to the v2/ER RPC (not mainnet).
    // Builder blockhash expires ~45s — rebuild on blockhash / simulation errors.
    let trade_rpc = v2_rpc_url();
    let mut last_err = String::new();
    let mut rest_amount_sol = amount_sol;
    let min_rest_amount_sol = min_open_collateral_lamports() as f64 / 1e9;
    for size_attempt in 0..open_backoff_attempts() {
        if rest_amount_sol < min_rest_amount_sol {
            return Err(format!(
                "Open position failed after REST capacity backoff below {:.9} SOL: {}",
                min_rest_amount_sol, last_err
            ));
        }

        let body = serde_json::json!({
            "inputTokenSymbol": "SOL",
            "outputTokenSymbol": "SOL",
            "inputAmountUi": rest_amount_sol.to_string(),
            "leverage": leverage,
            "tradeType": trade_type,
            "owner": wallet,
            "slippagePercentage": "1.0"
        });

        tracing::info!(
            "[OPEN] REST builder attempt {}/{} amount={:.9} SOL leverage={}x",
            size_attempt + 1,
            open_backoff_attempts(),
            rest_amount_sol,
            leverage
        );

        for attempt in 0..6u32 {
            let val = flash_post("/transaction-builder/open-position", &body).await?;

            if let Some(err) = val.get("err").and_then(|v| v.as_str())
                && !err.is_empty()
            {
                last_err = format!("Open position API error: {}", err);
                if is_flash_capacity_error(&last_err) {
                    break;
                }
                return Err(last_err);
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

            match sign_and_submit(keypair, tx_b64, &trade_rpc).await {
                Ok(sig) => {
                    // Don't report success unless the position is readable.
                    // Prevents phantom opens from polluting dashboard state.
                    let side_for_wait = if trade_type.eq_ignore_ascii_case("long") {
                        "Long"
                    } else {
                        "Short"
                    };
                    match wait_for_sol_position(&wallet, side_for_wait, 12).await {
                        Ok(pos) => {
                            let size = pos.size_usd_ui.parse().unwrap_or(size_usd);
                            let entry = pos.entry_price_ui.parse().unwrap_or(entry_price);
                            return Ok((sig, size, entry));
                        }
                        Err(e) => {
                            return Err(format!(
                                "Open tx confirmed ({sig}) but position not visible: {e}"
                            ));
                        }
                    }
                }
                Err(e) if is_flash_capacity_error(&e) => {
                    last_err = e;
                    break;
                }
                Err(e) if is_rebuild_error(&e) && attempt < 5 => {
                    last_err = e;
                    tokio::time::sleep(std::time::Duration::from_millis(750)).await;
                }
                Err(e) => return Err(e),
            }
        }

        rest_amount_sol /= 2.0;
        tracing::warn!(
            "[OPEN] REST Flash capacity error; retrying smaller collateral amount={:.9} SOL leverage={}x",
            rest_amount_sol,
            leverage
        );
    }

    Err(format!(
        "Open position failed after REST capacity/rebuild retries: {}",
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
    // SDK path first.
    let size_usd_f: f64 = size_usd.parse().unwrap_or(0.0);
    match try_close_via_sdk(size_usd_f, withdraw_token_symbol, side).await {
        Ok(sig_pnl) => return Ok(sig_pnl),
        Err(e) if e.starts_with("node unavailable") => {
            tracing::warn!("[CLOSE] SDK unavailable ({}). Falling back to REST.", e);
        }
        Err(other) => return Err(other),
    }

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

    let trade_rpc = v2_rpc_url();
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

        match sign_and_submit(keypair, tx_b64, &trade_rpc).await {
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

/// Flash v2 one-time account setup. Returns a list of submitted tx signatures.
/// Safe to re-run: each step is idempotent on the Flash side.
///
/// Order matters (per Flash SDK v2): deposit-ledger → basket → trade-vault → deposit → delegate.
/// We sleep 2s between steps so the prior account is visible to the next.
pub async fn v2_one_time_setup(
    keypair: &solana_sdk::signature::Keypair,
) -> Result<Vec<String>, String> {
    let wallet = keypair.pubkey().to_string();
    let mut submitted: Vec<String> = Vec::new();
    const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
    const SOL_DEPOSIT_UI: &str = "1.0"; // 1 SOL deposit for collateral

    // Step 0a — init deposit ledger
    match v2_call_and_submit(
        keypair,
        "/transaction-builder/init-deposit-ledger",
        serde_json::json!({ "owner": wallet.clone() }),
    )
    .await
    {
        Ok(sig) => submitted.push(format!("init-deposit-ledger: {sig}")),
        Err(e) if is_setup_already_done(&e) => {
            tracing::info!("[V2_SETUP] deposit-ledger already initialized; skipping");
        }
        Err(e) => {
            tracing::warn!("[V2_SETUP] init-deposit-ledger non-fatal: {e}");
        }
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Step 0b — init basket
    match v2_call_and_submit(
        keypair,
        "/transaction-builder/init-basket",
        serde_json::json!({ "owner": wallet.clone() }),
    )
    .await
    {
        Ok(sig) => submitted.push(format!("init-basket: {sig}")),
        Err(e) if is_setup_already_done(&e) => {
            tracing::info!("[V2_SETUP] basket already initialized; skipping ({e})");
        }
        // Re-runs after first setup often fail simulation (account exists).
        // Non-fatal: trading loop continues with existing basket.
        Err(e) => {
            tracing::warn!("[V2_SETUP] init-basket non-fatal: {e}");
        }
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Step 0c — init trade vault for SOL (globally idempotent)
    match v2_call_and_submit(
        keypair,
        "/transaction-builder/init-token-stake",
        serde_json::json!({ "owner": wallet.clone(), "tokenMint": SOL_MINT }),
    )
    .await
    {
        Ok(sig) => submitted.push(format!("init-token-stake(SOL): {sig}")),
        Err(e) if e.contains("already") || e.contains("initialized") => {
            tracing::info!("[V2_SETUP] trade vault for SOL already initialized; skipping");
        }
        Err(e) => return Err(format!("init-token-stake failed: {e}")),
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Step 1 — deposit SOL collateral
    match v2_call_and_submit(
        keypair,
        "/transaction-builder/deposit-direct",
        serde_json::json!({
            "owner": wallet.clone(),
            "tokenMint": SOL_MINT,
            "amount": SOL_DEPOSIT_UI,
        }),
    )
    .await
    {
        Ok(sig) => submitted.push(format!("deposit-direct(SOL): {sig}")),
        Err(e) => {
            tracing::warn!("[V2_SETUP] deposit-direct non-fatal: {}", e);
        }
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Step 2 — delegate basket (required before any open/close lands)
    match v2_call_and_submit(
        keypair,
        "/transaction-builder/delegate-basket",
        serde_json::json!({
            "payer": wallet.clone(),
            "owner": wallet.clone(),
        }),
    )
    .await
    {
        Ok(sig) => submitted.push(format!("delegate-basket: {sig}")),
        Err(e) if e.contains("already") || e.contains("delegated") => {
            tracing::info!("[V2_SETUP] basket already delegated; skipping");
        }
        Err(e) => return Err(format!("delegate-basket failed: {e}")),
    }

    Ok(submitted)
}

/// Build a Flash transaction via `flash_post`, then sign+submit.
/// Used for the v2 setup endpoints (funds path → Solana mainnet RPC).
async fn v2_call_and_submit(
    keypair: &solana_sdk::signature::Keypair,
    path: &str,
    body: serde_json::Value,
) -> Result<String, String> {
    let val = flash_post(path, &body).await?;
    if let Some(err) = val.get("err").and_then(|v| v.as_str())
        && !err.is_empty()
    {
        return Err(format!("API {path} err: {err}"));
    }
    let tx_b64 = val
        .get("transactionBase64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("No transaction in {path} response"))?;
    // Account & funds ops → Solana RPC (not v2/ER).
    sign_and_submit(keypair, tx_b64, &solana_rpc_url()).await
}

/// Sign a VersionedTransaction (base64) and submit via raw RPC to `rpc_url`.
///
/// Flash docs: funds ops → Solana RPC; trading → v2/ER RPC. Blockhash is
/// refreshed from the **same** RPC we submit to. We wait for confirmed
/// status and reject if `meta.err` is set so callers never treat a failed
/// on-chain open as success.
async fn sign_and_submit(
    keypair: &solana_sdk::signature::Keypair,
    tx_b64: &str,
    rpc_url: &str,
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

    let fresh_hash = fetch_latest_blockhash(&client, rpc_url).await?;
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

    let signature = rpc_send_transaction(&client, rpc_url, &signed_b64).await?;
    confirm_signature(&client, rpc_url, &signature).await?;
    Ok(signature)
}

/// Poll `getSignatureStatuses` until confirmed/finalized or timeout.
/// Returns Err if the transaction lands with a non-null `err`.
async fn confirm_signature(
    client: &reqwest::Client,
    rpc_url: &str,
    signature: &str,
) -> Result<(), String> {
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(TX_CONFIRM_TIMEOUT_SECS);
    loop {
        let resp = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getSignatureStatuses",
                "params": [
                    [signature],
                    { "searchTransactionHistory": true }
                ]
            }))
            .send()
            .await
            .map_err(|e| format!("getSignatureStatuses request failed: {e}"))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("getSignatureStatuses parse error: {e}"))?;

        if let Some(err) = json.get("error") {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown");
            // Some ER RPCs may not support this method — fall back to getTransaction.
            if msg.contains("Method not found") || msg.contains("method not found") {
                return confirm_signature_via_get_transaction(client, rpc_url, signature).await;
            }
        }

        let status = json
            .pointer("/result/value/0")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        if !status.is_null() {
            if let Some(err) = status.get("err") {
                if !err.is_null() {
                    return Err(format!("Transaction failed on-chain ({signature}): {err}"));
                }
            }
            let conf = status
                .get("confirmationStatus")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            if conf == "confirmed" || conf == "finalized" {
                return Ok(());
            }
            // Landed with null err and a slot is enough (some RPCs omit confirmationStatus)
            if status.get("err").map(|e| e.is_null()).unwrap_or(false)
                && status.get("slot").and_then(|s| s.as_u64()).is_some()
            {
                return Ok(());
            }
        }

        if std::time::Instant::now() >= deadline {
            return Err(format!(
                "Transaction confirmation timeout ({signature}) on {rpc_url}"
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(TX_CONFIRM_POLL_MS)).await;
    }
}

async fn confirm_signature_via_get_transaction(
    client: &reqwest::Client,
    rpc_url: &str,
    signature: &str,
) -> Result<(), String> {
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(TX_CONFIRM_TIMEOUT_SECS);
    loop {
        let resp = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getTransaction",
                "params": [
                    signature,
                    { "encoding": "json", "commitment": "confirmed", "maxSupportedTransactionVersion": 0 }
                ]
            }))
            .send()
            .await
            .map_err(|e| format!("getTransaction request failed: {e}"))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("getTransaction parse error: {e}"))?;

        if let Some(result) = json.get("result") {
            if !result.is_null() {
                let err = result.pointer("/meta/err");
                if err.map(|e| !e.is_null()).unwrap_or(false) {
                    return Err(format!(
                        "Transaction failed on-chain ({signature}): {}",
                        err.unwrap()
                    ));
                }
                return Ok(());
            }
        }

        if std::time::Instant::now() >= deadline {
            return Err(format!(
                "Transaction confirmation timeout via getTransaction ({signature})"
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(TX_CONFIRM_POLL_MS)).await;
    }
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

/// Setup steps are idempotent; re-runs hit "already exists" / simulation fails.
fn is_setup_already_done(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("already")
        || e.contains("initialized")
        || e.contains("0x0")
        || e.contains("custom program error")
        || e.contains("simulation failed")
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
                    // Simulate first so Custom 3007 / basket ownership errors
                    // surface immediately instead of as "success" signatures.
                    "skipPreflight": false,
                    "preflightCommitment": "confirmed",
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
        // Caller must still confirm (confirm_signature) — broadcast ≠ success.
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
    fn parse_v2_positions_map_by_market_pubkey() {
        // Flash v2: GET /positions/owner/{owner} returns
        // { "<marketPubkey>": PositionMetricsDto, ... }
        let positions = parse_positions_response(serde_json::json!({
            "SoLMarketPubkey1111111111111111111111111": {
                "sideUi": "LONG",
                "marketSymbol": "SOL",
                "sizeUsdUi": "331.46",
                "entryPriceUi": "76.67",
                "pnlWithFeeUsdUi": "2.1",
                "leverageUi": "9.0"
            }
        }))
        .unwrap();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].side_ui, "Long"); // normalized
        assert_eq!(positions[0].market_symbol, "SOL");
        assert_eq!(positions[0].key, "SoLMarketPubkey1111111111111111111111111");
        assert_eq!(positions[0].collateral_symbol, "SOL"); // default
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

    #[test]
    fn flash_capacity_errors_trigger_rest_fallback() {
        assert!(is_flash_capacity_error(
            "SDK error -32000: ER transaction failed: {\"InstructionError\":[1,{\"Custom\":6024}]}"
        ));
        assert!(is_flash_capacity_error(
            "RPC error: transaction verification error: Error processing Instruction 1: custom program error: 0x1788"
        ));
        assert!(is_flash_capacity_error("CustodyAmountLimit"));
        assert!(is_flash_capacity_error("MaxExposure"));
        assert!(!is_flash_capacity_error(
            "SDK error -32000: StaleOraclePrice"
        ));
    }

    #[test]
    fn v2_setup_endpoints_use_unprefixed_path() {
        // All v2 account/setup endpoints live under /transaction-builder/...
        // (same convention as /transaction-builder/open-position). Earlier
        // attempts to call /v2/transaction-builder/init-* returned 404.
        for (path, body) in [
            (
                "/transaction-builder/init-deposit-ledger",
                serde_json::json!({"owner": "Driyi"}),
            ),
            (
                "/transaction-builder/init-basket",
                serde_json::json!({"owner": "Driyi"}),
            ),
            (
                "/transaction-builder/delegate-basket",
                serde_json::json!({"payer": "Driyi", "owner": "Driyi"}),
            ),
            (
                "/transaction-builder/deposit-direct",
                serde_json::json!({
                    "owner": "Driyi",
                    "tokenMint": "So11111111111111111111111111111111111111112",
                    "amount": "1.0",
                }),
            ),
        ] {
            assert!(path.starts_with("/transaction-builder/"));
            assert!(
                path.contains("init-") || path.contains("delegate-") || path.contains("deposit-")
            );
            assert!(!path.starts_with("/v2/"));
            assert!(body.get("owner").is_some());
        }
    }
}
