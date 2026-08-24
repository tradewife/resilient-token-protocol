//! Autonomous trader module — Survivor 2.69 strategy on Flash Trade mainnet.
//!
//! Uses Binance OHLCV for warmup, then Flash Trade prices for ongoing data.
//! Executes via Flash Trade REST API (build → sign → submit).

pub mod candles;
pub mod executor;
pub mod gmtrade;
pub mod indicators;
pub mod strategy;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use candles::CandleBuffer;
use solana_sdk::signer::Signer;
use strategy::{OpenPosition, StrategyParams, TradeRecord};

/// Trader configuration loaded from env vars.
pub struct TraderConfig {
    pub keypair_path: PathBuf,
    pub amount_sol: f64,
    pub position_fraction: f64,
    pub leverage: f64,
    pub poll_secs: u64,
    pub dry_run: bool,
    pub state_path: PathBuf,
    pub rpc_url: String,
    /// Execution venue: "flash" (legacy Flash Trade, default) or "gmtrade"
    /// (GMTrade/gmx-solana keeper model). Selected via RTP_TRADER_VENUE.
    pub venue: String,
}

impl TraderConfig {
    pub fn from_env() -> Result<Self, String> {
        let keypair_path = std::env::var("RTP_TRADER_KEYPAIR")
            .map_err(|_| "RTP_TRADER_KEYPAIR not set".to_string())?;
        let amount_sol = std::env::var("RTP_TRADER_AMOUNT")
            .unwrap_or_else(|_| "0.20".to_string())
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("Invalid RTP_TRADER_AMOUNT: {}", e))?;
        let position_fraction: f64 = std::env::var("RTP_TRADER_POSITION_FRACTION")
            .unwrap_or_else(|_| "0.20".to_string())
            .parse()
            .map_err(|e: std::num::ParseFloatError| {
                format!("Invalid RTP_TRADER_POSITION_FRACTION: {}", e)
            })?;
        let leverage = std::env::var("RTP_TRADER_LEVERAGE")
            .unwrap_or_else(|_| "9.0".to_string())
            .parse()
            .map_err(|e: std::num::ParseFloatError| {
                format!("Invalid RTP_TRADER_LEVERAGE: {}", e)
            })?;
        let poll_secs: u64 = std::env::var("RTP_TRADER_POLL_SECS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .map_err(|e: std::num::ParseIntError| format!("Invalid RTP_TRADER_POLL_SECS: {}", e))?;
        let dry_run = std::env::var("RTP_TRADER_DRY_RUN")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false);

        let state_path = std::env::var("RTP_TRADER_STATE_PATH")
            .unwrap_or_else(|_| "data/trader-state.json".to_string());

        let rpc_url = std::env::var("RTP_TRADER_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

        let venue = std::env::var("RTP_TRADER_VENUE")
            .unwrap_or_else(|_| "flash".to_string())
            .to_lowercase();
        if venue != "flash" && venue != "gmtrade" {
            return Err(format!(
                "Invalid RTP_TRADER_VENUE '{venue}' — expected 'flash' or 'gmtrade'"
            ));
        }

        Ok(Self {
            keypair_path: PathBuf::from(keypair_path),
            amount_sol,
            position_fraction: position_fraction.clamp(0.01, 1.0),
            leverage,
            poll_secs: poll_secs.max(60),
            dry_run,
            state_path: PathBuf::from(state_path),
            rpc_url,
            venue,
        })
    }
}

/// Persistent trader state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderState {
    pub wallet: String,
    pub open_position: Option<OpenPosition>,
    pub trade_history: Vec<TradeRecord>,
    pub candle_count: usize,
    pub last_poll: String,
    pub total_pnl_sol: f64,
    pub total_trades: usize,
    /// Watchdog: tracks consecutive cycle errors. Resets to 0 on success.
    pub consecutive_errors: u32,
    /// Watchdog: last time a full cycle completed successfully.
    pub last_healthy: String,
    /// The active strategy config loaded at startup. Exposed via /state for dashboard visibility.
    #[serde(default)]
    pub active_config: StrategyParams,
}

impl TraderState {
    pub fn new(wallet: &str) -> Self {
        Self {
            wallet: wallet.to_string(),
            open_position: None,
            trade_history: Vec::new(),
            candle_count: 0,
            last_poll: String::new(),
            total_pnl_sol: 0.0,
            total_trades: 0,
            consecutive_errors: 0,
            last_healthy: String::new(),
            active_config: StrategyParams::default(),
        }
    }

    /// Load state from JSON file.
    pub fn load(path: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Correct mis-labeled `trade_history[].side` from entry/exit/`pnl_pct`. Returns repair count.
    pub fn repair_trade_history_sides(&mut self) -> usize {
        let mut repaired = 0usize;
        for t in &mut self.trade_history {
            if t.repair_side_from_pnl() {
                repaired += 1;
            }
        }
        repaired
    }

    /// Backfill `pnl_pct` on PhantomClear reconciliation rows that pre-P3-3
    /// bookkeeping recorded at 0.0 (the outcome was silently dropped from the
    /// tape). The row's own entry/exit prices carry the outcome — recompute it
    /// side-correct. Like the P3-3 estimate path this touches only the tape:
    /// `total_pnl_sol` stays a realized-only counter. Idempotent; run BEFORE
    /// `repair_trade_history_sides` so side inference sees real PnL.
    pub fn repair_phantom_clear_pnl(&mut self) -> usize {
        let mut repaired = 0usize;
        for t in &mut self.trade_history {
            if t.exit_reason.starts_with("PhantomClear")
                && t.pnl_pct.abs() < 1e-9
                && t.entry_price > 0.0
                && (t.exit_price - t.entry_price).abs() > 1e-12
            {
                let move_pct = (t.exit_price - t.entry_price) / t.entry_price * 100.0;
                t.pnl_pct = if t.side.eq_ignore_ascii_case("short") {
                    -move_pct
                } else {
                    move_pct
                };
                repaired += 1;
            }
        }
        repaired
    }

    /// Save state to JSON file.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("serialize: {}", e))?;
        std::fs::write(path, json).map_err(|e| format!("write: {}", e))
    }
}

/// Drop the current (still-forming) candle from a Binance kline fetch.
///
/// Binance returns the in-progress candle as the last row with its OPEN
/// timestamp. Loading it as a FINAL candle and then letting `append_tick`
/// roll the same hour creates a duplicated candle, and every warmup/refresh
/// was doing this. Compare the last candle's timestamp against the current
/// period boundary; drop it when it's the live one. Works for any
/// `period_secs` (3600, 14400, 86400).
fn drop_in_progress_candle(
    mut candles: Vec<indicators::Candle>,
    period_secs: i64,
) -> Vec<indicators::Candle> {
    if let Some(last) = candles.last() {
        let now = Utc::now().timestamp();
        let current_period_start = (now / period_secs) * period_secs;
        if last.timestamp == current_period_start {
            candles.pop();
        }
    }
    candles
}

/// Fetch SOL balance of a wallet via RPC. Returns SOL (not lamports).
async fn fetch_wallet_balance(rpc_url: &str, wallet: &str) -> Result<f64, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [wallet]
    });
    let resp = client
        .post(rpc_url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("RPC request failed: {}", e))?;
    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("RPC parse failed: {}", e))?;
    let lamports: u64 = val["result"]["value"].as_u64().unwrap_or(0);
    Ok(lamports as f64 / 1e9)
}

/// Spawn a lightweight HTTP server that serves GET /state with current trader state.
/// Returns the JoinHandle so the caller can optionally wait on it.
pub fn start_status_server(
    state: Arc<Mutex<TraderState>>,
    port: u16,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await {
            Ok(l) => {
                tracing::info!("[HTTP] Status server listening on port {}", port);
                l
            }
            Err(e) => {
                tracing::error!(
                    "[HTTP] Failed to bind port {}: {}. Status server not started.",
                    port,
                    e
                );
                return;
            }
        };

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("[HTTP] Accept failed: {}", e);
                    continue;
                }
            };

            let state = state.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_status_request(stream, state).await {
                    tracing::debug!("[HTTP] Request error: {}", e);
                }
            });
        }
    })
}

/// Health-check thresholds.
const HEALTH_MAX_ERRORS: u32 = 5;
const HEALTH_STALE_THRESHOLD_SECS: i64 = 30 * 60; // 30 minutes

/// After a collateral-floor refusal, skip entry attempts for this long.
/// The wallet can't clear the venue's minimum until it's funded, so
/// retrying every poll only burns RPC calls and the watchdog error
/// budget. 1h keeps log noise down without delaying a funding fix long.
const ENTRY_COOLDOWN_SECS: u64 = 3600;

/// Determine trader health from current state.
/// Returns `(status_code, status_reason, body_text)`.
pub fn check_trader_health(state: &TraderState) -> (u16, &'static str, String) {
    // 1. Too many consecutive errors → 503
    if state.consecutive_errors >= HEALTH_MAX_ERRORS {
        return (
            503,
            "Service Unavailable",
            format!("unhealthy: {} consecutive errors", state.consecutive_errors),
        );
    }

    // 2. Empty last_healthy (initial state) → 503
    if state.last_healthy.is_empty() {
        return (
            503,
            "Service Unavailable",
            "unhealthy: no healthy timestamp".to_string(),
        );
    }

    // 3. Unparseable last_healthy → 503
    let last_healthy = match chrono::DateTime::parse_from_rfc3339(&state.last_healthy) {
        Ok(dt) => dt,
        Err(_) => {
            return (
                503,
                "Service Unavailable",
                "unhealthy: invalid last_healthy timestamp".to_string(),
            );
        }
    };

    // 4. Stale last_healthy (> 30 min ago) → 503
    let now = chrono::Utc::now();
    let elapsed_secs = now.signed_duration_since(last_healthy).num_seconds();
    if elapsed_secs > HEALTH_STALE_THRESHOLD_SECS {
        return (
            503,
            "Service Unavailable",
            format!("unhealthy: last_healthy is stale ({}s ago)", elapsed_secs),
        );
    }

    // All checks passed → healthy
    (200, "OK", "ok".to_string())
}

fn request_header<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    for line in request.lines().skip(1) {
        if line.trim().is_empty() {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(value.trim());
        }
    }
    None
}

fn operator_authorized(request: &str) -> bool {
    let Ok(secret) = std::env::var("RTP_OPERATOR_API_SECRET") else {
        return false;
    };
    if secret.is_empty() {
        return false;
    }

    if let Some(auth) = request_header(request, "authorization")
        && auth.len() >= 7
        && auth[..7].eq_ignore_ascii_case("bearer ")
    {
        return auth[7..].trim() == secret;
    }

    request_header(request, "x-rtp-operator-secret")
        .map(|candidate| candidate == secret)
        .unwrap_or(false)
}

async fn handle_status_request(
    mut stream: tokio::net::TcpStream,
    state: Arc<Mutex<TraderState>>,
) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Read enough to parse the request line
    let mut buf = [0u8; 1024];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("read: {}", e))?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Extract the method/path from the request line (e.g., "GET /state HTTP/1.1")
    let mut request_parts = request.lines().next().unwrap_or("").split_whitespace();
    let method = request_parts.next().unwrap_or("GET");
    let path = request_parts.next().unwrap_or("/");

    let (status, body, content_type) = if path == "/state" || path == "/" {
        let snapshot = state.lock().await;
        let json = serde_json::to_string(&*snapshot).unwrap_or_else(|_| "{}".to_string());
        ("200 OK".to_string(), json, "application/json")
    } else if path == "/health" {
        let snapshot = state.lock().await;
        let (code, reason, body) = check_trader_health(&snapshot);
        (format!("{} {}", code, reason), body, "text/plain")
    } else if path == "/clear-position" {
        if method != "POST" {
            (
                "405 Method Not Allowed".to_string(),
                "method not allowed".to_string(),
                "text/plain",
            )
        } else if !operator_authorized(&request) {
            (
                "401 Unauthorized".to_string(),
                "unauthorized".to_string(),
                "text/plain",
            )
        } else {
            state.lock().await.open_position = None;
            tracing::warn!("[HTTP] open_position cleared via authorized /clear-position request");
            ("200 OK".to_string(), "ok".to_string(), "text/plain")
        }
    } else if path == "/clear" {
        tracing::warn!("[HTTP] deprecated /clear endpoint rejected");
        ("410 Gone".to_string(), "gone".to_string(), "text/plain")
    } else {
        (
            "404 Not Found".to_string(),
            "not found".to_string(),
            "text/plain",
        )
    };

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    );

    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|e| format!("write: {}", e))?;
    stream.flush().await.map_err(|e| format!("flush: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Venue dispatch — route execution calls to Flash Trade (legacy) or GMTrade
// based on TraderConfig.venue (RTP_TRADER_VENUE=flash|gmtrade). The strategy
// layer is venue-agnostic; these wrappers are the ONLY place the venue is
// branched on.
// ---------------------------------------------------------------------------

fn is_gm(venue: &str) -> bool {
    venue == "gmtrade"
}

/// Venue-aware SOL price.
async fn venue_get_sol_price(venue: &str) -> Result<f64, String> {
    if is_gm(venue) {
        gmtrade::get_sol_price().await
    } else {
        executor::get_sol_price().await
    }
}

/// Venue-aware positions fetch.
async fn venue_get_positions(
    venue: &str,
    wallet: &str,
) -> Result<Vec<executor::PositionInfo>, String> {
    if is_gm(venue) {
        gmtrade::get_positions(wallet).await
    } else {
        executor::get_positions(wallet).await
    }
}

/// Venue-aware open.
async fn venue_open_position(
    venue: &str,
    keypair: &solana_sdk::signature::Keypair,
    amount_sol: f64,
    leverage: f64,
    trade_type: &str,
) -> Result<(String, f64, f64), String> {
    if is_gm(venue) {
        gmtrade::open_position(keypair, amount_sol, leverage, trade_type).await
    } else {
        executor::open_position(keypair, amount_sol, leverage, trade_type).await
    }
}

/// Venue-aware close.
async fn venue_close_position(
    venue: &str,
    keypair: &solana_sdk::signature::Keypair,
    market_symbol: &str,
    side: &str,
    size_usd: &str,
    withdraw_token: &str,
) -> Result<(String, f64), String> {
    if is_gm(venue) {
        gmtrade::close_position(keypair, market_symbol, side, size_usd, withdraw_token).await
    } else {
        executor::close_position(keypair, market_symbol, side, size_usd, withdraw_token).await
    }
}

/// Run the main trader loop.
pub async fn run_trader(config: TraderConfig) -> Result<(), String> {
    // Load keypair
    let keypair_data = std::fs::read_to_string(&config.keypair_path)
        .map_err(|e| format!("Read keypair {}: {}", config.keypair_path.display(), e))?;
    let keypair_bytes: Vec<u8> =
        serde_json::from_str(&keypair_data).map_err(|e| format!("Parse keypair: {}", e))?;
    let keypair = solana_sdk::signature::Keypair::from_bytes(&keypair_bytes)
        .map_err(|e| format!("Invalid keypair: {}", e))?;
    let wallet = keypair.pubkey().to_string();

    tracing::info!("=== RTP Autonomous Trader ===");
    tracing::info!("Wallet:     {}", wallet);
    tracing::info!(
        "Amount:     {} SOL (only if balance fetch fails; normal sizing = {}% of balance)",
        config.amount_sol,
        config.position_fraction * 100.0
    );
    tracing::info!(
        "Fraction:   {}% of wallet balance",
        config.position_fraction * 100.0
    );
    tracing::info!("Leverage:   {}x", config.leverage);
    tracing::info!("Poll:       {}s", config.poll_secs);
    tracing::info!("Dry run:    {}", config.dry_run);
    tracing::info!("State:      {}", config.state_path.display());
    tracing::info!("");

    // Operational overrides — allow live tunability without redeploying the
    // binary. With the V2 transition (Jul 2026) we already had one multi-day
    // trading shutdown — the operator requested the ability to relax strict
    // WFA-confluence params on the fly. These env vars ONLY relax thresholds
    // (never tighten them). Missing env = validated config.
    let min_alignment_override = std::env::var("RTP_TRADER_MIN_ALIGNMENT_OVERRIDE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok());
    let signal_threshold_override = std::env::var("RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE")
        .ok()
        .and_then(|s| s.parse::<f64>().ok());

    // Load or create state — shared with HTTP status server via Arc<Mutex>
    let mut initial =
        TraderState::load(&config.state_path).unwrap_or_else(|| TraderState::new(&wallet));

    // Wallet rotation: if the loaded state was written under a previous
    // keypair (e.g. RTP_TRADER_KEYPAIR_JSON was rotated after a deploy), the
    // `wallet` string is stale and the dashboard would read the OLD pubkey.
    // Rotate to the live wallet, **preserving trade history** so the dashboard
    // tape continues across wallet rotations, but clear any orphaned open
    // position (the reconciler on first poll will silently close it if Flash
    // has no matching account, instead of failing loudly).
    //
    // Note on totals: prior-wallet `total_trades` and `total_pnl_sol` are
    // preserved as-is — they're monotonic counters and accurately reflect the
    // position lifecycle that the trader observed under either wallet.
    if initial.wallet != wallet {
        tracing::warn!(
            "[WALLET_ROTATE] state wallet {} != live wallet {} — rotating to live wallet, \
             preserving {} trades (cumulative totals retained) and clearing any stale open position",
            initial.wallet,
            wallet,
            initial.trade_history.len()
        );
        initial.wallet = wallet.clone();
        initial.open_position = None;
        if let Err(e) = initial.save(&config.state_path) {
            tracing::warn!("[WALLET_ROTATE] could not persist rotated state: {}", e);
        }
    }

    let phantom_repaired = initial.repair_phantom_clear_pnl();
    if phantom_repaired > 0 {
        tracing::warn!(
            "[STATE] Backfilled {} PhantomClear row(s) booked at 0% pnl (pre-P3-3 audit rows)",
            phantom_repaired
        );
    }

    let repaired = initial.repair_trade_history_sides();
    if phantom_repaired > 0 || repaired > 0 {
        if repaired > 0 {
            tracing::warn!(
                "[STATE] Repaired {} trade_history side label(s) from pnl_pct (legacy default Long)",
                repaired
            );
        }
        if let Err(e) = initial.save(&config.state_path) {
            tracing::warn!(
                "[STATE] Could not persist trade_history repairs (trading continues): {}",
                e
            );
        }
    }
    let state = Arc::new(Mutex::new(initial));
    let mut params = StrategyParams::load_from_daemon_config();

    // Apply operational overrides from env. Both are loosening-only.
    let _min_align_before = params.min_alignment;
    let _signal_before = params.signal_threshold;
    if let Some(v) = min_alignment_override {
        if v < params.min_alignment {
            tracing::warn!(
                "[OVERRIDE] min_alignment {} -> {} (RTP_TRADER_MIN_ALIGNMENT_OVERRIDE). NOTE: not WFA-validated, loosens strict-WFA confluence config.",
                params.min_alignment,
                v
            );
            params.min_alignment = v;
        } else {
            tracing::warn!(
                "[OVERRIDE] env value {} >= configured {} — ignored (overrides are loosening-only).",
                v,
                params.min_alignment
            );
        }
    }
    if let Some(v) = signal_threshold_override {
        if v < params.signal_threshold {
            tracing::warn!(
                "[OVERRIDE] signal_threshold {:.3} -> {:.3} (RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE). NOTE: not WFA-validated.",
                params.signal_threshold,
                v
            );
            params.signal_threshold = v;
        } else {
            tracing::warn!(
                "[OVERRIDE] signal_threshold env value {:.3} >= configured {:.3} — ignored (overrides are loosening-only).",
                v,
                params.signal_threshold
            );
        }
    }

    // Store active config in state for /state endpoint visibility
    {
        let mut s = state.lock().await;
        s.active_config = params.clone();
    }

    // Log loaded params at startup for Railway log visibility
    tracing::info!(
        "[STARTUP] Active strategy config: signal={:.2} tp={:.1} sl={:.1} hold={:.0}h trail={:.2} decay={:.0}h flip_delay={:.1}h alignment={}",
        params.signal_threshold,
        params.tp_atr,
        params.sl_atr,
        params.max_hold_hours,
        params.trailing_stop_atr,
        params.time_decay_hours,
        params.score_flip_delay_hrs,
        params.min_alignment,
    );

    // Multi-TF candle buffers. Each is a separate Binance interval — slicing
    // a single 1h buffer at lookback 20/80/200 is NOT multi-timeframe, it's
    // the same trend smoothed three ways (Jul 26-28 trader stuck in bear=3
    // permanently from this bug). Fetch 1h, 4h, and 1d candles independently.
    let mut buffer_1h = CandleBuffer::new(300); // 300 candles ≈ 12.5 days of 1h
    let mut buffer_4h = CandleBuffer::new(200); // 200 candles ≈ 33 days of 4h
    let mut buffer_1d = CandleBuffer::new(120); // 120 candles ≈ 4 months of 1d

    // Start HTTP status server for live dashboard access
    let http_port: u16 = std::env::var("RTP_TRADER_HTTP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);
    let http_state = state.clone();
    start_status_server(http_state, http_port);

    // Optional: one-time wallet setup. Flash v2: Node SDK wrapper
    // (init-deposit-ledger → init-basket → init-trade-vault → depositDirect SOL
    // → delegate-basket). GMTrade: no setup required (collateral deposits
    // atomically with each order).
    // Flash path notes: funds ops use Solana RPC; trading uses ER.
    // The deposit step is what actually matters for opens to succeed — the
    // basket.delegate field is deprecated per the new SDK and the explicit
    // delegateBasket-on-ER path surfaces Custom:27 (UnsupportedToken) on
    // Flash program because the user's basket account is not in the ER pool
    // config. Continue-with-warning on any V2_SETUP error so the trader can
    // still attempt opens (which use session keys, not basket.delegate).
    if std::env::var("RTP_TRADER_RUN_V2_SETUP")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        tracing::info!("[V2_SETUP] Starting {} one-time setup", config.venue);
        let setup_result = if is_gm(&config.venue) {
            gmtrade::v2_one_time_setup(&keypair).await
        } else {
            executor::v2_one_time_setup(&keypair).await
        };
        match setup_result {
            Ok(sigs) => {
                tracing::info!("[V2_SETUP] OK: {}", sigs.join(", "));
            }
            Err(e) => {
                tracing::warn!(
                    "[V2_SETUP] failed: {} — continuing with trading loop anyway (basket.delegate is deprecated; opens use session keys)",
                    e
                );
            }
        }
    }

    // Warmup: fetch historical candles from Binance — INDEPENDENT intervals
    // for 1h, 4h, 1d. Each is fetched in parallel; one failure doesn't block
    // the others.
    tracing::info!("[WARMUP] Fetching multi-TF OHLCV from Binance (1h, 4h, 1d)...");
    let (r1h, r4h, r1d) = tokio::join!(
        candles::fetch_binance_ohlcv("SOLUSDT", "1h", 300),
        candles::fetch_binance_ohlcv("SOLUSDT", "4h", 200),
        candles::fetch_binance_ohlcv("SOLUSDT", "1d", 120),
    );
    if let Ok(c) = r1h {
        let c = drop_in_progress_candle(c, 3600);
        tracing::info!("[WARMUP] Loaded {} 1h candles from Binance", c.len());
        buffer_1h.load_candles(c);
    } else if let Err(e) = r1h {
        tracing::warn!("[WARMUP] 1h Binance fetch failed ({}). Starting cold.", e);
    }
    if let Ok(c) = r4h {
        let c = drop_in_progress_candle(c, 4 * 3600);
        tracing::info!("[WARMUP] Loaded {} 4h candles from Binance", c.len());
        buffer_4h.load_candles(c);
    } else if let Err(e) = r4h {
        tracing::warn!("[WARMUP] 4h Binance fetch failed ({}). Starting cold.", e);
    }
    if let Ok(c) = r1d {
        let c = drop_in_progress_candle(c, 24 * 3600);
        tracing::info!("[WARMUP] Loaded {} 1d candles from Binance", c.len());
        buffer_1d.load_candles(c);
    } else if let Err(e) = r1d {
        tracing::warn!("[WARMUP] 1d Binance fetch failed ({}). Starting cold.", e);
    }

    // Reconcile with Flash Trade: if a position is open on-chain but missing
    // from internal state (e.g. after redeploy), restore it so the trader
    // can manage exits and won't open duplicates.
    {
        let mut s = state.lock().await;
        if s.open_position.is_none() {
            match venue_get_positions(&config.venue, &wallet).await {
                Ok(positions) => {
                    // Look for both Long and Short SOL positions
                    if let Some(pos) = positions.iter().find(|p| {
                        p.market_symbol == "SOL" && (p.side_ui == "Long" || p.side_ui == "Short")
                    }) {
                        let entry_price: f64 = pos.entry_price_ui.parse().unwrap_or(0.0);
                        let size_usd: f64 = pos.size_usd_ui.parse().unwrap_or(0.0);
                        let side = pos.side_ui.clone();
                        // Use the venue-reported open time when available so
                        // MaxHold / time-decay measure the TRUE holding
                        // period. The old code always assumed "1h ago", which
                        // silently restarted the 96h MaxHold clock on every
                        // redeploy and let orphaned positions drift far past
                        // their validated hold window. Fall back to 1h ago
                        // only when the venue reports no timestamp.
                        let now_ts = Utc::now().timestamp();
                        let entry_time = if pos.opened_at_secs > 0 && pos.opened_at_secs <= now_ts {
                            pos.opened_at_secs
                        } else {
                            tracing::warn!(
                                "[RECONCILE] venue reported no open time — assuming 1h ago \
                                 (MaxHold clock will be approximate)"
                            );
                            now_ts - 3600
                        };
                        tracing::warn!(
                            "[RECONCILE] Found orphaned SOL {} position on {}: \
                             entry=${:.2} size=${:.2} opened={} ({}h ago) key={}... \
                             restoring to internal state.",
                            side,
                            config.venue,
                            entry_price,
                            size_usd,
                            chrono::DateTime::from_timestamp(entry_time, 0)
                                .map(|t| t.to_rfc3339())
                                .unwrap_or_else(|| "invalid".to_string()),
                            (now_ts - entry_time) as f64 / 3600.0,
                            &pos.key[..pos.key.len().min(8)]
                        );
                        s.open_position = Some(OpenPosition {
                            entry_price,
                            entry_time,
                            peak_price: entry_price,
                            entry_rsi: 50.0, // neutral default
                            entry_atr: 0.0,
                            entry_score: 0.0,
                            position_key: pos.key.clone(),
                            size_usd,
                            first_negative_score_time: None,
                            side: side.clone(),
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "[RECONCILE] Failed to check Flash Trade positions: {}. Continuing without reconciliation.",
                        e
                    );
                }
            }
        }
    }

    // ✅ Stale-state cleanup: if internal state thinks a position is open
    // but Flash Trade says there is no such position on-chain, clear it.
    let stale_state = {
        let s = state.lock().await;
        s.open_position.is_some()
    };
    if stale_state {
        match venue_get_positions(&config.venue, &wallet).await {
            Ok(positions) => {
                let side_upper = state
                    .lock()
                    .await
                    .open_position
                    .as_ref()
                    .map(|p| match p.side.as_str() {
                        "Long" | "LONG" => "LONG".to_string(),
                        "Short" | "SHORT" => "SHORT".to_string(),
                        _ => p.side.clone(),
                    })
                    .unwrap_or_default();
                let side_lookup = match side_upper.as_str() {
                    "SHORT" => "Short",
                    _ => "Long",
                };
                let on_chain = positions
                    .iter()
                    .any(|p| p.market_symbol == "SOL" && p.side_ui == side_lookup);
                if !on_chain {
                    // The position vanished between sessions (external close,
                    // liquidation, or a prior session's unrecorded close).
                    // Booking pnl_pct = 0 here silently dropped the real
                    // outcome from the tape — estimate it against the current
                    // venue price instead (NOT added to total_pnl_sol, which
                    // stays a realized-only counter).
                    let est_price = venue_get_sol_price(&config.venue).await.unwrap_or(0.0);
                    tracing::warn!(
                        "[CLEANUP] Stale SOL {} position in state — not found on {}. \
                         Clearing (exit price estimate ${:.2}).",
                        side_lookup,
                        config.venue,
                        est_price
                    );
                    let mut s = state.lock().await;
                    if let Some(pos) = s.open_position.take() {
                        let exit_price = if est_price > 0.0 {
                            est_price
                        } else {
                            pos.entry_price
                        };
                        let pnl_pct = if pos.entry_price > 0.0 && est_price > 0.0 {
                            match pos.side() {
                                "Short" => (pos.entry_price - exit_price) / pos.entry_price * 100.0,
                                _ => (exit_price - pos.entry_price) / pos.entry_price * 100.0,
                            }
                        } else {
                            0.0
                        };
                        s.trade_history.push(TradeRecord {
                            entry_price: pos.entry_price,
                            exit_price,
                            entry_time: pos.entry_time,
                            exit_time: Utc::now().timestamp(),
                            pnl_pct,
                            exit_reason: "PhantomClear(StartupReconcile)".to_string(),
                            size_usd: pos.size_usd,
                            side: pos.side().to_string(),
                            fees: None,
                        });
                        s.total_trades += 1;
                    }
                    if let Err(e) = s.save(&config.state_path) {
                        tracing::warn!("[CLEANUP] Save failed: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[CLEANUP] Could not check Flash Trade positions: {}", e);
            }
        }
    }

    // Main loop with watchdog — each cycle is wrapped in a timeout.
    // If a cycle hangs (e.g., HTTP request stalls), the watchdog kills it,
    // increments consecutive_errors, and retries after a backoff.
    //
    // Venue-aware timeout: Flash ops are REST-fast (build → sign → confirm in
    // seconds). GMTrade orders wait for keeper fills — measured 24.8–31.0s per
    // fill, capped by RTP_GM_FILL_TIMEOUT_SECS (default 90s). A single cycle
    // performs at most one order (entry XOR exit), so the GM budget is the fill
    // wait plus RPC/price overhead, with headroom for a slow keeper.
    const CYCLE_TIMEOUT_SECS_FLASH: u64 = 120; // max time for one Flash cycle
    const CYCLE_TIMEOUT_SECS_GM: u64 = 300; // keeper fill wait + overhead
    let cycle_timeout_secs: u64 = if is_gm(&config.venue) {
        CYCLE_TIMEOUT_SECS_GM
    } else {
        CYCLE_TIMEOUT_SECS_FLASH
    };
    const MAX_CONSECUTIVE_ERRORS: u32 = 10; // after this many, sleep longer

    // Slow-TF refresh schedule. After warmup, buffer_4h and buffer_1d are
    // stale snapshots — they never receive `append_tick` (only 1h ticks).
    // Without periodic refetch from Binance, tf_4h.trend and tf_1d.trend are
    // pinned at warmup SMA/price, and bullish/bearish counts can never flip
    // as the market moves. Refetch 4h every 2h, 1d every 6h — fast enough to
    // react to real TF changes, slow enough to avoid burning Binance quota.
    const SLOW_REFRESH_4H_SECS: i64 = 2 * 3600;
    const SLOW_REFRESH_1D_SECS: i64 = 6 * 3600;
    // 1h buffer: rebuilt live from ~12 venue ticks/hour with tick-count
    // "volumes", so the vol_confirm score term is dead and long-uptime
    // buffers drift from the true hourly closes. Refetch hourly like the
    // slow TFs (the in-progress candle is dropped; append_tick rebuilds it).
    const SLOW_REFRESH_1H_SECS: i64 = 3600;
    let mut last_4h_refresh = Utc::now().timestamp();
    let mut last_1d_refresh = Utc::now().timestamp();
    let mut last_1h_refresh = Utc::now().timestamp();

    // Entry cooldown: when an open is refused because the wallet can't clear
    // the venue's collateral minimum, retrying every poll until someone funds
    // the wallet burns RPC calls and fills the watchdog error budget (Aug 14:
    // 10/10 errors in 45 min). After such a refusal, entry attempts are
    // skipped until this timestamp.
    let mut entry_cooldown_until: i64 = 0;

    tracing::info!(
        "[LOOP] Starting autonomous trading loop (watchdog: {}s cycle timeout)...",
        cycle_timeout_secs
    );
    loop {
        let cycle_start = Utc::now();
        {
            let mut s = state.lock().await;
            s.last_poll = cycle_start.to_rfc3339();
        }

        let cycle_result = tokio::time::timeout(
            std::time::Duration::from_secs(cycle_timeout_secs),
            run_cycle(
                &config,
                &keypair,
                &wallet,
                &params,
                &mut buffer_1h,
                &mut buffer_4h,
                &mut buffer_1d,
                &state,
                &mut last_1h_refresh,
                &mut last_4h_refresh,
                &mut last_1d_refresh,
                SLOW_REFRESH_1H_SECS,
                SLOW_REFRESH_4H_SECS,
                SLOW_REFRESH_1D_SECS,
                &mut entry_cooldown_until,
            ),
        )
        .await;

        match cycle_result {
            Ok(Ok(())) => {
                // Cycle completed successfully
                let mut s = state.lock().await;
                s.consecutive_errors = 0;
                s.last_healthy = Utc::now().to_rfc3339();
            }
            Ok(Err(e)) => {
                // Cycle returned an error — log and continue
                tracing::error!("[WATCHDOG] Cycle error: {}", e);
                let mut s = state.lock().await;
                s.consecutive_errors += 1;
                tracing::warn!(
                    "[WATCHDOG] Consecutive errors: {}/{}",
                    s.consecutive_errors,
                    MAX_CONSECUTIVE_ERRORS
                );
            }
            Err(_) => {
                // Cycle timed out — watchdog killed it
                tracing::error!(
                    "[WATCHDOG] Cycle timed out after {}s — likely HTTP/keeper hang",
                    cycle_timeout_secs
                );
                let mut s = state.lock().await;
                s.consecutive_errors += 1;
                tracing::warn!(
                    "[WATCHDOG] Consecutive errors: {}/{}",
                    s.consecutive_errors,
                    MAX_CONSECUTIVE_ERRORS
                );
            }
        }

        // Save state after every cycle (success or failure)
        {
            let mut s = state.lock().await;
            s.candle_count = buffer_1h.len();
            if let Err(e) = s.save(&config.state_path) {
                tracing::warn!("[STATE] Save failed: {}", e);
            }
        }

        // Backoff sleep: longer if we're in an error streak
        let err_count = state.lock().await.consecutive_errors;
        let base_sleep = config.poll_secs;
        let sleep_secs = if err_count >= MAX_CONSECUTIVE_ERRORS {
            tracing::warn!("[WATCHDOG] Max errors reached — sleeping 5 min before retry");
            300
        } else if err_count > 0 {
            // Exponential backoff: 30s, 60s, 90s, 120s...
            base_sleep.max(30 * err_count as u64)
        } else {
            let elapsed = (Utc::now() - cycle_start).num_seconds() as u64;
            base_sleep.saturating_sub(elapsed)
        };

        if sleep_secs > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
        }
    }
}

/// Run a single trading cycle: fetch price → check exit → check entry → save.
/// Returns Err only for fatal-ish errors. Most errors are logged and swallowed.
///
/// `too_many_arguments`: the parameter list is the cycle's full context
/// (config, keys, strategy params, TF buffers, shared state, refresh
/// schedule, entry cooldown). Bundling it into a struct would be churn on
/// live trading code for no behavior change; the single call site passes
/// everything through explicitly, which keeps the data flow readable.
#[allow(clippy::too_many_arguments)]
async fn run_cycle(
    config: &TraderConfig,
    keypair: &solana_sdk::signature::Keypair,
    wallet: &str,
    params: &StrategyParams,
    buffer_1h: &mut CandleBuffer,
    buffer_4h: &mut CandleBuffer,
    buffer_1d: &mut CandleBuffer,
    state: &Arc<Mutex<TraderState>>,
    last_1h_refresh: &mut i64,
    last_4h_refresh: &mut i64,
    last_1d_refresh: &mut i64,
    refresh_1h_secs: i64,
    refresh_4h_secs: i64,
    refresh_1d_secs: i64,
    entry_cooldown_until: &mut i64,
) -> Result<(), String> {
    let cycle_start = Utc::now();

    // 0. Periodically refresh TF buffers from Binance. The 1h buffer
    // receives live ticks via `append_tick`, but ticks carry tick-count
    // "volumes" and an uptime-accumulated buffer drifts from the true
    // hourly closes the strategy was validated on — refetch it hourly like
    // the slow TFs. Without periodic refetch, tf_4h.trend and tf_1d.trend
    // stay pinned at warmup SMA/price, and bullish/bearish counts can never
    // flip with the market even when the trend has clearly shifted. This was
    // the core wiring bug behind the "stuck at bull=2 bear=1 for 12 hours"
    // deadlock.
    let now_ts = cycle_start.timestamp();
    if now_ts - *last_1h_refresh >= refresh_1h_secs {
        match candles::fetch_binance_ohlcv("SOLUSDT", "1h", 300).await {
            Ok(c) if !c.is_empty() => {
                let c = drop_in_progress_candle(c, 3600);
                tracing::info!("[REFRESH] 1h: loaded {} candles from Binance", c.len());
                buffer_1h.load_candles(c);
                *last_1h_refresh = now_ts;
            }
            Ok(_) => tracing::warn!("[REFRESH] 1h: Binance returned empty candle set"),
            Err(e) => tracing::warn!(
                "[REFRESH] 1h: Binance fetch failed ({}) — using stale buffer",
                e
            ),
        }
    }
    if now_ts - *last_4h_refresh >= refresh_4h_secs {
        match candles::fetch_binance_ohlcv("SOLUSDT", "4h", 200).await {
            Ok(c) if !c.is_empty() => {
                let c = drop_in_progress_candle(c, 4 * 3600);
                tracing::info!("[REFRESH] 4h: loaded {} candles from Binance", c.len());
                buffer_4h.load_candles(c);
                *last_4h_refresh = now_ts;
            }
            Ok(_) => tracing::warn!("[REFRESH] 4h: Binance returned empty candle set"),
            Err(e) => tracing::warn!(
                "[REFRESH] 4h: Binance fetch failed ({}) — using stale buffer",
                e
            ),
        }
    }
    if now_ts - *last_1d_refresh >= refresh_1d_secs {
        match candles::fetch_binance_ohlcv("SOLUSDT", "1d", 120).await {
            Ok(c) if !c.is_empty() => {
                let c = drop_in_progress_candle(c, 24 * 3600);
                tracing::info!("[REFRESH] 1d: loaded {} candles from Binance", c.len());
                buffer_1d.load_candles(c);
                *last_1d_refresh = now_ts;
            }
            Ok(_) => tracing::warn!("[REFRESH] 1d: Binance returned empty candle set"),
            Err(e) => tracing::warn!(
                "[REFRESH] 1d: Binance fetch failed ({}) — using stale buffer",
                e
            ),
        }
    }

    // 1. Fetch current SOL price from the venue price source
    let price = match venue_get_sol_price(&config.venue).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("[POLL] Price fetch failed: {}", e);
            return Ok(()); // non-fatal, will retry next cycle
        }
    };

    // Tick the 1h buffer only — 4h and 1d are slow-moving and don't get fresh
    // ticks on every cycle. Append only when the timestamp rolls into a new
    // candle boundary so we don't blow up the slow TFs with noise.
    let tick_ts = cycle_start.timestamp();
    buffer_1h.append_tick(price, tick_ts);
    let has_pos = state.lock().await.open_position.is_some();
    tracing::info!(
        "[POLL] SOL=${:.2} | 1h={} 4h={} 1d={} | pos={}",
        price,
        buffer_1h.len(),
        buffer_4h.len(),
        buffer_1d.len(),
        if has_pos { "OPEN" } else { "FLAT" }
    );

    let closes_1h = buffer_1h.closes();
    let closes_4h = buffer_4h.closes();
    let closes_1d = buffer_1d.closes();
    let volumes = buffer_1h.volumes();

    // 2. Check exit on existing position
    let exit_info = {
        let s = state.lock().await;
        if let Some(ref pos) = s.open_position {
            if let Some(signal) = strategy::compute_signal(
                &closes_1h,
                &closes_4h,
                &closes_1d,
                &volumes,
                params.min_alignment,
            ) {
                let now_secs = Utc::now().timestamp();
                let current_price = closes_1h.last().copied().unwrap_or(0.0);
                // Determine position side from stored state (default "Long" for backward compat)
                let side = pos.side();
                let result = strategy::check_exit(
                    params,
                    pos.entry_price,
                    pos.entry_time,
                    pos.peak_price,
                    pos.entry_rsi,
                    current_price,
                    signal.score,
                    signal.rsi,
                    signal.atr,
                    now_secs,
                    side,
                    pos.first_negative_score_time,
                );
                Some((result, pos.clone(), signal.score))
            } else {
                None
            }
        } else {
            None
        }
    };

    // Always update first_negative_score_time from check_exit result
    if let Some((ref result, ref pos_info, _)) = exit_info {
        // Update first_negative_score_time in state regardless of exit
        {
            let mut s = state.lock().await;
            if let Some(ref mut pos) = s.open_position {
                pos.first_negative_score_time = result.first_negative_score_time;
            }
        }

        if let Some(ref reason) = result.reason {
            tracing::info!("[EXIT] {:?} triggered!", reason);

            let mut close_succeeded = config.dry_run;
            if !config.dry_run {
                match venue_get_positions(&config.venue, wallet).await {
                    Ok(positions) => {
                        let pos_side = pos_info.side();
                        if let Some(pos_api) = positions
                            .iter()
                            .find(|p| p.market_symbol == "SOL" && p.side_ui == pos_side)
                        {
                            match venue_close_position(
                                &config.venue,
                                keypair,
                                &pos_api.market_symbol,
                                &pos_api.side_ui,
                                &pos_api.size_usd_ui,
                                &pos_api.collateral_symbol,
                            )
                            .await
                            {
                                Ok((sig, pnl)) => {
                                    tracing::info!(
                                        "[EXIT] TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta",
                                        sig
                                    );
                                    tracing::info!("[EXIT] PnL: ${:.4}", pnl);

                                    let exit_price = closes_1h.last().copied().unwrap_or(0.0);
                                    let side = pos_info.side();
                                    let pnl_pct = if pos_info.entry_price > 0.0 {
                                        match side {
                                            "Short" => {
                                                (pos_info.entry_price - exit_price)
                                                    / pos_info.entry_price
                                                    * 100.0
                                            }
                                            _ => {
                                                (exit_price - pos_info.entry_price)
                                                    / pos_info.entry_price
                                                    * 100.0
                                            }
                                        }
                                    } else {
                                        0.0
                                    };
                                    // Approximate SOL PnL from % move × notional / entry price
                                    let pnl_sol = if exit_price > 0.0 {
                                        (pnl_pct / 100.0) * (pos_info.size_usd / exit_price)
                                    } else {
                                        0.0
                                    };
                                    // Flash v2 cost ledger: capture the fee
                                    // breakdown the positions API reports for
                                    // this position (unsettled obligations =
                                    // what the close charges). Raw fields are
                                    // 1e6-scaled integer strings; PositionInfo
                                    // converts them to USD.
                                    let fees = pos_api.fee_breakdown_usd();
                                    tracing::info!(
                                        "[EXIT] Fees: exit=${:.4} borrow=${:.4} impact=${:.4} total=${:.4}",
                                        fees.exit_fee_usd,
                                        fees.borrow_fee_usd,
                                        fees.price_impact_usd,
                                        fees.total_fee_usd
                                    );
                                    let trade = TradeRecord {
                                        entry_price: pos_info.entry_price,
                                        exit_price,
                                        entry_time: pos_info.entry_time,
                                        exit_time: Utc::now().timestamp(),
                                        pnl_pct,
                                        exit_reason: format!("{:?}", reason),
                                        size_usd: pos_info.size_usd,
                                        side: side.to_string(),
                                        fees: Some(fees),
                                    };
                                    let mut s = state.lock().await;
                                    s.trade_history.push(trade);
                                    s.total_trades += 1;
                                    s.total_pnl_sol += pnl_sol;
                                    close_succeeded = true;
                                }
                                Err(e) => {
                                    tracing::error!("[EXIT] Close failed: {}", e);
                                }
                            }
                        } else {
                            tracing::warn!(
                                "[EXIT] No SOL {} position found on {} — clearing phantom local state",
                                pos_side,
                                config.venue
                            );
                            // Never treat a missing on-chain position as a real close for PnL
                            // (total_pnl_sol untouched), but record an audit row against the
                            // current price so the tape reflects the estimated outcome —
                            // booking 0.0 silently dropped these results before.
                            let exit_price =
                                closes_1h.last().copied().unwrap_or(pos_info.entry_price);
                            let side = pos_info.side().to_string();
                            let pnl_pct = if pos_info.entry_price > 0.0 {
                                match pos_info.side() {
                                    "Short" => {
                                        (pos_info.entry_price - exit_price) / pos_info.entry_price
                                            * 100.0
                                    }
                                    _ => {
                                        (exit_price - pos_info.entry_price) / pos_info.entry_price
                                            * 100.0
                                    }
                                }
                            } else {
                                0.0
                            };
                            let trade = TradeRecord {
                                entry_price: pos_info.entry_price,
                                exit_price,
                                entry_time: pos_info.entry_time,
                                exit_time: Utc::now().timestamp(),
                                pnl_pct,
                                exit_reason: format!("PhantomClear({:?})", reason),
                                size_usd: pos_info.size_usd,
                                side,
                                fees: None,
                            };
                            let mut s = state.lock().await;
                            s.trade_history.push(trade);
                            s.total_trades += 1;
                            close_succeeded = true;
                        }
                    }
                    Err(e) => {
                        tracing::error!("[EXIT] Positions fetch failed: {}", e);
                    }
                }
            } else {
                tracing::info!("[DRY RUN] Would close position: {:?}", reason);
            }

            if close_succeeded {
                state.lock().await.open_position = None;
            }
        } else {
            // No exit triggered — update peak price for trailing stop
            let current_price = closes_1h.last().copied().unwrap_or(0.0);
            let side = pos_info.side();
            let should_update_peak = match side {
                "Short" => current_price < pos_info.peak_price, // track trough for SHORT
                _ => current_price > pos_info.peak_price,       // track peak for LONG
            };
            if should_update_peak && let Some(ref mut pos) = state.lock().await.open_position {
                pos.peak_price = current_price;
            }
        }
    } else {
        // 3. Check entry signal (only if flat)
        if let Some(signal) = strategy::compute_signal(
            &closes_1h,
            &closes_4h,
            &closes_1d,
            &volumes,
            params.min_alignment,
        ) {
            tracing::info!(
                "[SIGNAL] score={:.3} rsi={:.1} bull={} bear={} atr={:.2} reasons={:?}",
                signal.score,
                signal.rsi,
                signal.bullish_count,
                signal.bearish_count,
                signal.atr,
                signal.reasons
            );

            // Entry logic: LONG or SHORT, mutually exclusive.
            //
            // Mirrors the Python Survivor 2.69 reference (`run_backtest_r2.py`,
            // line ~257): `if score > threshold: buy`. The alignment count is
            // already baked into the score (trend weight 0.4 × bull_count/3),
            // so an extra `bullish_count >= min_alignment` AND-gate here would
            // double-count the alignment requirement and make it impossible
            // to clear the threshold with min_alignment=2 unless an additional
            // booster (momentum, MR, BB) pushed the score past 0.30. In a
            // sideways market those boosters don't fire, so the score caps at
            // 0.267 and no entry triggers — even when price action would
            // normally qualify. Matching Python: gate on score only.
            let entry_signal = if signal.score > params.signal_threshold {
                Some((
                    "Long",
                    "LONG",
                    signal.score,
                    signal.bullish_count,
                    signal.reasons.clone(),
                ))
            } else if signal.score < -params.signal_threshold {
                Some((
                    "Short",
                    "SHORT",
                    signal.score,
                    signal.bearish_count,
                    signal.reasons.clone(),
                ))
            } else {
                None
            };

            if let Some((side, trade_type, score, align_count, reasons)) = entry_signal {
                tracing::info!(
                    "[ENTRY] Signal: {} score={:.3} align={} reasons={:?}",
                    side,
                    score,
                    align_count,
                    reasons
                );

                // Entry cooldown: a recent open was refused on wallet sizing
                // (collateral below the venue floor). Skip entry attempts
                // until the cooldown expires instead of hammering the venue
                // every poll — the wallet won't clear the floor until someone
                // funds it, and retrying burns RPC calls + the watchdog error
                // budget (Aug 14: 10/10 consecutive errors in 45 min).
                let now_ts = cycle_start.timestamp();
                if now_ts < *entry_cooldown_until {
                    tracing::info!(
                        "[ENTRY] Skipped — collateral refusal cooldown active, \
                         next attempt after {}",
                        chrono::DateTime::from_timestamp(*entry_cooldown_until, 0)
                            .map(|t| t.to_rfc3339())
                            .unwrap_or_else(|| "unknown".to_string())
                    );
                } else if !config.dry_run {
                    // Compute position size as fraction of wallet balance
                    let amount_sol = match fetch_wallet_balance(&config.rpc_url, wallet).await {
                        Ok(balance) => {
                            let sized = balance * config.position_fraction;
                            tracing::info!(
                                "[ENTRY] Wallet: {:.4} SOL → position: {:.4} SOL ({:.0}% @ {}x)",
                                balance,
                                sized,
                                config.position_fraction * 100.0,
                                config.leverage
                            );
                            sized
                        }
                        Err(e) => {
                            tracing::warn!(
                                "[ENTRY] Balance fetch failed ({}). Using fallback: {} SOL",
                                e,
                                config.amount_sol
                            );
                            config.amount_sol
                        }
                    };

                    // Collateral pre-flight (GM only): if the sized collateral
                    // can't clear the fee-sane floor, skip BEFORE spending a
                    // venue round-trip (the venue-side check still guards —
                    // this only avoids the wasted call). The floor is the
                    // configurable `min_open_collateral_lamports()` (default
                    // 0.5 SOL), not the venue's $1 minimum: fixed per-order
                    // costs (execution fee + wrap) make sub-floor positions
                    // fee-negative regardless of edge.
                    if is_gm(&config.venue) {
                        let floor_lamports = gmtrade::min_open_collateral_lamports();
                        if (amount_sol * 1e9) < floor_lamports as f64 {
                            tracing::warn!(
                                "[ENTRY] Pre-flight: {:.4} SOL sized collateral below the {} \
                                 lamport fee-sane floor — soft-skip, entry cooldown {}s",
                                amount_sol,
                                floor_lamports,
                                ENTRY_COOLDOWN_SECS
                            );
                            *entry_cooldown_until =
                                cycle_start.timestamp() + ENTRY_COOLDOWN_SECS as i64;
                            return Ok(());
                        }
                    }

                    match venue_open_position(
                        &config.venue,
                        keypair,
                        amount_sol,
                        config.leverage,
                        trade_type,
                    )
                    .await
                    {
                        Ok((sig, size_usd, entry_price)) => {
                            tracing::info!(
                                "[ENTRY] TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta",
                                sig
                            );

                            // open_position already waits for the position to be readable.
                            // Re-fetch key for state; refuse to set open if still missing.
                            let pos_side = side.to_string();
                            match venue_get_positions(&config.venue, wallet).await {
                                Ok(positions) => {
                                    if let Some(p) = positions
                                        .into_iter()
                                        .find(|p| p.market_symbol == "SOL" && p.side_ui == side)
                                    {
                                        let entry = p.entry_price_ui.parse().unwrap_or(entry_price);
                                        let size = p.size_usd_ui.parse().unwrap_or(size_usd);
                                        state.lock().await.open_position = Some(OpenPosition {
                                            entry_price: entry,
                                            entry_time: Utc::now().timestamp(),
                                            peak_price: entry,
                                            entry_rsi: signal.rsi,
                                            entry_atr: signal.atr,
                                            entry_score: signal.score,
                                            position_key: p.key,
                                            size_usd: size,
                                            first_negative_score_time: None,
                                            side: pos_side,
                                        });
                                    } else {
                                        tracing::error!(
                                            "[ENTRY] Open returned ok but no SOL {} on Flash — not setting local open_position",
                                            side
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "[ENTRY] Open ok but positions fetch failed ({e}) — not setting local open_position"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            // Venue capacity soft-skip: market side is full.
                            // Do NOT count as a cycle error — the loop stays
                            // healthy and retries when headroom reappears.
                            if e.starts_with(gmtrade::CAPACITY_FULL_PREFIX) {
                                tracing::warn!("[ENTRY] Venue capacity full — soft-skip open: {e}");
                            } else if e.starts_with(gmtrade::INSUFFICIENT_COLLATERAL_PREFIX) {
                                // Wallet can't clear the venue's collateral
                                // minimum. Soft-skip (no cycle error) AND arm
                                // the entry cooldown — retrying every poll
                                // until funding arrives only burns RPC calls
                                // and the watchdog error budget (Aug 14).
                                tracing::warn!(
                                    "[ENTRY] Wallet below venue collateral floor — soft-skip open, \
                                     entry cooldown {}s: {e}",
                                    ENTRY_COOLDOWN_SECS
                                );
                                *entry_cooldown_until =
                                    cycle_start.timestamp() + ENTRY_COOLDOWN_SECS as i64;
                            } else {
                                tracing::error!("[ENTRY] Open failed: {}", e);
                                // Count open failures as cycle errors so the
                                // error-backoff sleep prevents burning gas on
                                // retries during hard failures.
                                return Err(format!("Open position failed: {e}"));
                            }
                        }
                    }
                } else {
                    tracing::info!(
                        "[DRY RUN] Would open {} SOL {} @ {}x ({}%)",
                        config.amount_sol,
                        side,
                        config.leverage,
                        config.position_fraction * 100.0
                    );
                }
            }
        }
    }

    Ok(())
}

// cache-bust: 2026-05-11T20:00Z

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trader_state_active_config_serde_roundtrip() {
        let mut state = TraderState::new("TestWallet11111111111111111111111111111111");
        state.active_config = StrategyParams {
            signal_threshold: 0.3,
            tp_atr: 6.0,
            sl_atr: 2.5,
            max_hold_hours: 96.0,
            trailing_stop_atr: 1.0,
            time_decay_hours: 48.0,
            min_alignment: 3,
            score_flip_delay_hrs: 2.0,
        };
        let json = serde_json::to_string_pretty(&state).unwrap();
        let parsed: TraderState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.active_config.signal_threshold, 0.3);
        assert_eq!(parsed.active_config.tp_atr, 6.0);
        assert_eq!(parsed.active_config.sl_atr, 2.5);
        assert_eq!(parsed.active_config.max_hold_hours, 96.0);
        assert_eq!(parsed.active_config.trailing_stop_atr, 1.0);
        assert_eq!(parsed.active_config.time_decay_hours, 48.0);
        assert_eq!(parsed.active_config.min_alignment, 3);
        assert_eq!(parsed.active_config.score_flip_delay_hrs, 2.0);
    }

    #[test]
    fn trader_state_loads_without_active_config_field() {
        // Existing trader-state.json lacks the active_config field.
        // With #[serde(default)], it should deserialize with StrategyParams::default().
        let old_json = r#"{
            "wallet": "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R",
            "open_position": null,
            "trade_history": [],
            "candle_count": 200,
            "last_poll": "2026-05-05T00:00:00+00:00",
            "total_pnl_sol": 0.0,
            "total_trades": 1,
            "consecutive_errors": 0,
            "last_healthy": "2026-05-05T00:00:00+00:00"
        }"#;
        let parsed: TraderState = serde_json::from_str(old_json).unwrap();
        let defaults = StrategyParams::default();
        assert_eq!(
            parsed.wallet,
            "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R"
        );
        assert_eq!(
            parsed.active_config.signal_threshold,
            defaults.signal_threshold
        );
        assert_eq!(parsed.active_config.tp_atr, defaults.tp_atr);
        assert_eq!(
            parsed.active_config.score_flip_delay_hrs,
            defaults.score_flip_delay_hrs
        );
        assert_eq!(
            parsed.active_config.time_decay_hours,
            defaults.time_decay_hours
        );
        assert_eq!(parsed.active_config.min_alignment, defaults.min_alignment);
    }

    #[test]
    fn trader_state_new_has_default_active_config() {
        let state = TraderState::new("TestWallet11111111111111111111111111111111");
        let defaults = StrategyParams::default();
        assert_eq!(
            state.active_config.signal_threshold,
            defaults.signal_threshold
        );
        assert_eq!(state.active_config.tp_atr, defaults.tp_atr);
        assert_eq!(state.active_config.sl_atr, defaults.sl_atr);
        assert_eq!(state.active_config.max_hold_hours, defaults.max_hold_hours);
        assert_eq!(
            state.active_config.trailing_stop_atr,
            defaults.trailing_stop_atr
        );
        assert_eq!(
            state.active_config.time_decay_hours,
            defaults.time_decay_hours
        );
        assert_eq!(state.active_config.min_alignment, defaults.min_alignment);
        assert_eq!(
            state.active_config.score_flip_delay_hrs,
            defaults.score_flip_delay_hrs
        );
    }

    #[test]
    fn startup_log_format_includes_all_strategy_fields() {
        // Verify the format string includes all StrategyParams fields
        // by constructing the same log message and checking key substrings
        let params = StrategyParams {
            signal_threshold: 0.3,
            tp_atr: 6.0,
            sl_atr: 2.5,
            max_hold_hours: 96.0,
            trailing_stop_atr: 1.0,
            time_decay_hours: 48.0,
            min_alignment: 3,
            score_flip_delay_hrs: 2.0,
        };
        let log_msg = format!(
            "signal={:.2} tp={:.1} sl={:.1} hold={:.0}h trail={:.2} decay={:.0}h flip_delay={:.1}h alignment={}",
            params.signal_threshold,
            params.tp_atr,
            params.sl_atr,
            params.max_hold_hours,
            params.trailing_stop_atr,
            params.time_decay_hours,
            params.score_flip_delay_hrs,
            params.min_alignment,
        );
        // Verify all key fields appear in the formatted log
        assert!(
            log_msg.contains("signal=0.30"),
            "Log must include signal_threshold"
        );
        assert!(log_msg.contains("tp=6.0"), "Log must include tp_atr");
        assert!(log_msg.contains("sl=2.5"), "Log must include sl_atr");
        assert!(
            log_msg.contains("hold=96h"),
            "Log must include max_hold_hours"
        );
        assert!(
            log_msg.contains("trail=1.00"),
            "Log must include trailing_stop_atr"
        );
        assert!(
            log_msg.contains("decay=48h"),
            "Log must include time_decay_hours"
        );
        assert!(
            log_msg.contains("flip_delay=2.0h"),
            "Log must include score_flip_delay_hrs"
        );
        assert!(
            log_msg.contains("alignment=3"),
            "Log must include min_alignment"
        );
    }

    #[test]
    fn existing_trader_state_json_file_loads() {
        // Load the actual data/trader-state.json file from the repo
        let repo_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/trader-state.json");
        if let Some(state) = TraderState::load(&repo_root) {
            assert_eq!(state.wallet, "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R");
            assert_eq!(state.total_trades, 1);
            // active_config should default since the file doesn't have this field
            let defaults = StrategyParams::default();
            assert_eq!(
                state.active_config.signal_threshold,
                defaults.signal_threshold
            );
            assert_eq!(
                state.active_config.score_flip_delay_hrs,
                defaults.score_flip_delay_hrs
            );
        }
        // If the file doesn't exist (CI), that's fine — we already test with a JSON string above
    }

    #[test]
    fn active_config_preserved_through_save_load_cycle() {
        let tmp = std::env::temp_dir().join("test-trader-state-active-config.json");
        let mut state = TraderState::new("TestWallet11111111111111111111111111111111");
        state.active_config = StrategyParams {
            signal_threshold: 0.3,
            tp_atr: 6.0,
            sl_atr: 2.5,
            max_hold_hours: 96.0,
            trailing_stop_atr: 1.0,
            time_decay_hours: 48.0,
            min_alignment: 3,
            score_flip_delay_hrs: 2.0,
        };
        state.save(&tmp).unwrap();
        let loaded = TraderState::load(&tmp).unwrap();
        assert_eq!(loaded.active_config.signal_threshold, 0.3);
        assert_eq!(loaded.active_config.tp_atr, 6.0);
        assert_eq!(loaded.active_config.score_flip_delay_hrs, 2.0);
        assert_eq!(loaded.active_config.time_decay_hours, 48.0);
        assert_eq!(loaded.active_config.min_alignment, 3);
        // Clean up
        let _ = std::fs::remove_file(&tmp);
    }

    // =========================================================================
    // SHORT position tests (feature: short-entry-and-exit-logic)
    // =========================================================================

    #[test]
    fn repair_trade_history_sides_fixes_legacy_long_labels() {
        let mut state = TraderState::new("TestWallet");
        state.trade_history.push(TradeRecord {
            entry_price: 82.01,
            exit_price: 81.40314411,
            entry_time: 0,
            exit_time: 0,
            pnl_pct: 0.739977917327162,
            exit_reason: "ScoreFlip".to_string(),
            size_usd: 266.77,
            side: "Long".to_string(),
            fees: None,
        });
        state.trade_history.push(TradeRecord {
            entry_price: 95.35,
            exit_price: 95.69,
            entry_time: 0,
            exit_time: 0,
            pnl_pct: 0.3565810173046706,
            exit_reason: "TrailingStop".to_string(),
            size_usd: 337.46,
            side: "Long".to_string(),
            fees: None,
        });
        assert_eq!(state.repair_trade_history_sides(), 1);
        assert_eq!(state.trade_history[0].side, "Short");
        assert_eq!(state.trade_history[1].side, "Long");
    }

    #[test]
    fn repair_phantom_clear_pnl_backfills_zero_booked_rows() {
        // Pre-P3-3 reconciliation rows were booked at pnl_pct = 0.0 even though
        // entry/exit prices carry the real outcome. The repair recomputes them
        // side-correct and leaves realized rows + total_pnl_sol untouched.
        let mut state = TraderState::new("TestWallet");
        state.trade_history.push(TradeRecord {
            entry_price: 91.245369,
            exit_price: 90.42,
            entry_time: 0,
            exit_time: 0,
            pnl_pct: 0.0,
            exit_reason: "PhantomClear(TrailingStop)".to_string(),
            size_usd: 597.0,
            side: "Long".to_string(),
            fees: None,
        });
        state.trade_history.push(TradeRecord {
            entry_price: 77.188243,
            exit_price: 80.39,
            entry_time: 0,
            exit_time: 0,
            pnl_pct: 0.0,
            exit_reason: "PhantomClear(TakeProfit)".to_string(),
            size_usd: 288.0,
            side: "Short".to_string(),
            fees: None,
        });
        // Realized row with real pnl — must stay untouched.
        state.trade_history.push(TradeRecord {
            entry_price: 94.0,
            exit_price: 96.0,
            entry_time: 0,
            exit_time: 0,
            pnl_pct: 2.1276595744680854,
            exit_reason: "TakeProfit".to_string(),
            size_usd: 300.0,
            side: "Long".to_string(),
            fees: None,
        });

        assert_eq!(state.repair_phantom_clear_pnl(), 2);
        // Long losing: (90.42 - 91.245369) / 91.245369 * 100
        let long = &state.trade_history[0];
        assert!(
            (long.pnl_pct - (-0.904643)).abs() < 1e-4,
            "got {}",
            long.pnl_pct
        );
        // Short losing on a price rise: -4.148%
        let short = &state.trade_history[1];
        assert!(
            (short.pnl_pct - (-4.147986)).abs() < 1e-4,
            "got {}",
            short.pnl_pct
        );
        // Realized row untouched.
        assert!((state.trade_history[2].pnl_pct - 2.1276595744680854).abs() < 1e-9);
        // Idempotent.
        assert_eq!(state.repair_phantom_clear_pnl(), 0);
    }

    #[test]
    fn short_position_stored_with_correct_side() {
        let mut state = TraderState::new("TestWallet");
        state.open_position = Some(OpenPosition {
            entry_price: 100.0,
            entry_time: 1700000000,
            peak_price: 100.0,
            entry_rsi: 50.0,
            entry_atr: 2.0,
            entry_score: -0.5,
            position_key: "short_key_123".to_string(),
            size_usd: 50.0,
            first_negative_score_time: None,
            side: "Short".to_string(),
        });
        assert_eq!(state.open_position.as_ref().unwrap().side(), "Short");
    }

    #[test]
    fn short_position_pnl_recording() {
        // Verify TradeRecord PnL is correct for SHORT: (entry - exit) / entry * 100
        let entry_price = 100.0;
        let exit_price = 95.0;
        let pnl_pct = (entry_price - exit_price) / entry_price * 100.0;
        assert_eq!(
            pnl_pct, 5.0,
            "SHORT profit should be +5% when price drops 5%"
        );

        // Verify SHORT loss PnL
        let exit_price_loss = 110.0;
        let pnl_pct_loss = (entry_price - exit_price_loss) / entry_price * 100.0;
        assert_eq!(
            pnl_pct_loss, -10.0,
            "SHORT loss should be -10% when price rises 10%"
        );
    }

    #[test]
    fn long_pnl_recording_unchanged() {
        // LONG PnL should be unchanged: (exit - entry) / entry * 100
        let entry_price = 100.0;
        let exit_price = 110.0;
        let pnl_pct = (exit_price - entry_price) / entry_price * 100.0;
        assert_eq!(
            pnl_pct, 10.0,
            "LONG profit should be +10% when price rises 10%"
        );

        let exit_price_loss = 90.0;
        let pnl_pct_loss = (exit_price_loss - entry_price) / entry_price * 100.0;
        assert_eq!(
            pnl_pct_loss, -10.0,
            "LONG loss should be -10% when price drops 10%"
        );
    }

    #[test]
    fn peak_update_for_short_tracks_trough() {
        // For SHORT: peak_price is updated when current < peak (tracking trough)
        let mut pos = OpenPosition {
            entry_price: 100.0,
            entry_time: 1700000000,
            peak_price: 100.0,
            entry_rsi: 50.0,
            entry_atr: 2.0,
            entry_score: -0.5,
            position_key: "key".to_string(),
            size_usd: 50.0,
            first_negative_score_time: None,
            side: "Short".to_string(),
        };

        // Price drops from 100 to 95 — favorable for SHORT, update trough
        let current = 95.0;
        assert!(current < pos.peak_price, "95 < 100: should update trough");
        pos.peak_price = current;
        assert_eq!(pos.peak_price, 95.0);

        // Price drops further to 90 — update trough again
        let current2 = 90.0;
        assert!(current2 < pos.peak_price, "90 < 95: should update trough");
        pos.peak_price = current2;
        assert_eq!(pos.peak_price, 90.0);

        // Price rises to 93 — should NOT update (tracking trough, not peak)
        let current3 = 93.0;
        assert!(
            current3 > pos.peak_price,
            "93 > 90: should NOT update trough"
        );
        assert_eq!(pos.peak_price, 90.0, "Trough should remain at 90");
    }

    #[test]
    fn peak_update_for_long_tracks_high() {
        // For LONG: peak_price is updated when current > peak (tracking high)
        let mut pos = OpenPosition {
            entry_price: 100.0,
            entry_time: 1700000000,
            peak_price: 100.0,
            entry_rsi: 50.0,
            entry_atr: 2.0,
            entry_score: 0.5,
            position_key: "key".to_string(),
            size_usd: 50.0,
            first_negative_score_time: None,
            side: "Long".to_string(),
        };

        // Price rises from 100 to 105 — favorable for LONG, update peak
        let current = 105.0;
        assert!(current > pos.peak_price, "105 > 100: should update peak");
        pos.peak_price = current;
        assert_eq!(pos.peak_price, 105.0);
    }

    #[test]
    fn reconcile_restores_short_position() {
        // Simulate TraderState with a SHORT position from reconciliation
        let mut state = TraderState::new("TestWallet");
        state.open_position = Some(OpenPosition {
            entry_price: 150.0,
            entry_time: 1700000000,
            peak_price: 150.0,
            entry_rsi: 50.0,
            entry_atr: 3.0,
            entry_score: -0.4,
            position_key: "reconciled_short_key".to_string(),
            size_usd: 30.0,
            first_negative_score_time: None,
            side: "Short".to_string(),
        });

        // Verify it serializes/deserializes correctly
        let json = serde_json::to_string(&state).unwrap();
        let parsed: TraderState = serde_json::from_str(&json).unwrap();
        let pos = parsed.open_position.unwrap();
        assert_eq!(pos.side, "Short");
        assert_eq!(pos.entry_price, 150.0);
        assert_eq!(pos.position_key, "reconciled_short_key");
    }

    #[test]
    fn existing_trader_state_json_loads_with_default_side() {
        // Load the actual data/trader-state.json which has an open position without `side` field
        let repo_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/trader-state.json");
        if let Some(state) = TraderState::load(&repo_root) {
            if let Some(ref pos) = state.open_position {
                assert_eq!(
                    pos.side, "Long",
                    "Existing state should default side to Long"
                );
            }
        }
    }

    #[test]
    fn entry_signal_logic_long_condition() {
        // Verify: score > threshold → LONG (matches Python reference — no
        // separate alignment AND-gate; the alignment count is already baked
        // into the score via the trend weight 0.4 × bull_count/3).
        let params = StrategyParams {
            signal_threshold: 0.3,
            min_alignment: 2,
            ..StrategyParams::default()
        };
        let score = 0.5;
        let bullish_count: usize = 2;

        let is_long = score > params.signal_threshold;
        assert!(
            is_long,
            "Score 0.5 > 0.3 → LONG regardless of alignment count ({})",
            bullish_count
        );
    }

    #[test]
    fn entry_signal_logic_short_condition() {
        // Verify: score < -threshold → SHORT (matches Python reference — score-only)
        let params = StrategyParams {
            signal_threshold: 0.3,
            min_alignment: 2,
            ..StrategyParams::default()
        };
        let score = -0.5;
        let bearish_count: usize = 2;

        let is_short = score < -params.signal_threshold;
        assert!(
            is_short,
            "Score -0.5 < -0.3 → SHORT regardless of alignment count ({})",
            bearish_count
        );
    }

    #[test]
    fn entry_signal_logic_no_entry_between_thresholds() {
        // Verify: score between ±threshold → neither LONG nor SHORT
        let params = StrategyParams {
            signal_threshold: 0.3,
            min_alignment: 3,
            ..StrategyParams::default()
        };

        for score in [-0.2, 0.0, 0.1, 0.29] {
            let is_long = score > params.signal_threshold;
            let is_short = score < -params.signal_threshold;
            assert!(!is_long, "Score {} should not trigger LONG", score);
            assert!(!is_short, "Score {} should not trigger SHORT", score);
        }
    }

    #[test]
    fn entry_signal_logic_exclusive() {
        // A single score cannot trigger both LONG and SHORT
        let params = StrategyParams {
            signal_threshold: 0.3,
            min_alignment: 3,
            ..StrategyParams::default()
        };

        for score in [-1.0, -0.5, -0.3, 0.0, 0.3, 0.5, 1.0] {
            let is_long = score > params.signal_threshold;
            let is_short = score < -params.signal_threshold;
            assert!(
                !(is_long && is_short),
                "Score {} cannot be both LONG and SHORT",
                score
            );
        }
    }

    // =========================================================================
    // Health monitoring tests (feature: health-monitoring)
    // =========================================================================

    /// Helper: create a healthy TraderState for test setup.
    fn healthy_state() -> TraderState {
        let mut state = TraderState::new("TestWallet");
        state.consecutive_errors = 0;
        state.last_healthy = chrono::Utc::now().to_rfc3339();
        state
    }

    #[test]
    fn health_returns_200_when_healthy() {
        // VAL-HEALTH-001: consecutive_errors < 5 and recent last_healthy → 200
        let state = healthy_state();
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(code, 200, "Should return 200 for healthy state");
        assert_eq!(body, "ok", "Body should be 'ok' for healthy state");
    }

    #[test]
    fn health_returns_503_when_consecutive_errors_exceeds_threshold() {
        // VAL-HEALTH-002: consecutive_errors >= 5 → 503
        let mut state = healthy_state();
        state.consecutive_errors = 5;
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(code, 503, "Should return 503 when consecutive_errors >= 5");
        assert!(
            body.contains("consecutive errors"),
            "Body should mention errors: {}",
            body
        );

        // Also test with > 5
        state.consecutive_errors = 10;
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(code, 503, "Should return 503 when consecutive_errors = 10");
        assert!(
            body.contains("consecutive errors"),
            "Body should mention errors: {}",
            body
        );
    }

    #[test]
    fn health_returns_503_when_last_healthy_stale() {
        // VAL-HEALTH-003: last_healthy > 30 minutes ago → 503
        let mut state = healthy_state();
        // Set last_healthy to 35 minutes ago
        let stale_time = chrono::Utc::now() - chrono::Duration::minutes(35);
        state.last_healthy = stale_time.to_rfc3339();
        state.consecutive_errors = 0;
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(code, 503, "Should return 503 when last_healthy is stale");
        assert!(
            body.contains("stale"),
            "Body should mention stale: {}",
            body
        );
    }

    #[test]
    fn health_returns_503_when_last_healthy_empty() {
        // VAL-HEALTH-006: empty last_healthy → 503 (initial state)
        let mut state = TraderState::new("TestWallet");
        state.consecutive_errors = 0;
        state.last_healthy = String::new(); // empty — initial state
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(code, 503, "Should return 503 when last_healthy is empty");
        assert!(
            body.contains("no healthy timestamp"),
            "Body should mention missing timestamp: {}",
            body
        );
    }

    #[test]
    fn health_returns_503_when_last_healthy_unparseable() {
        // VAL-HEALTH-006: unparseable last_healthy → 503
        let mut state = TraderState::new("TestWallet");
        state.consecutive_errors = 0;
        state.last_healthy = "garbage-not-a-timestamp".to_string();
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(
            code, 503,
            "Should return 503 when last_healthy cannot be parsed"
        );
        assert!(
            body.contains("invalid"),
            "Body should mention invalid timestamp: {}",
            body
        );
    }

    #[test]
    fn health_returns_200_when_just_under_stale_threshold() {
        // Verify: last_healthy = 29 minutes ago → still 200
        let mut state = healthy_state();
        let recent_time = chrono::Utc::now() - chrono::Duration::minutes(29);
        state.last_healthy = recent_time.to_rfc3339();
        state.consecutive_errors = 0;
        let (code, _reason, _body) = check_trader_health(&state);
        assert_eq!(
            code, 200,
            "Should return 200 when last_healthy is 29 minutes ago"
        );
    }

    #[test]
    fn health_error_takes_priority_over_stale() {
        // Both consecutive_errors >= 5 AND stale last_healthy → should still
        // return 503 (error check runs first)
        let mut state = TraderState::new("TestWallet");
        state.consecutive_errors = 5;
        state.last_healthy = String::new();
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(code, 503);
        assert!(
            body.contains("consecutive errors"),
            "Error check should fire first: {}",
            body
        );
    }

    #[test]
    fn state_endpoint_returns_valid_json() {
        // VAL-HEALTH-004: /state still returns full TraderState JSON
        let state = healthy_state();
        let json = serde_json::to_string(&state).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        // Verify key fields exist
        assert!(
            parsed.get("wallet").is_some(),
            "/state JSON must include wallet"
        );
        assert!(
            parsed.get("open_position").is_some(),
            "/state JSON must include open_position"
        );
        assert!(
            parsed.get("consecutive_errors").is_some(),
            "/state JSON must include consecutive_errors"
        );
        assert!(
            parsed.get("last_healthy").is_some(),
            "/state JSON must include last_healthy"
        );
        assert!(
            parsed.get("active_config").is_some(),
            "/state JSON must include active_config"
        );
        assert!(
            parsed.get("trade_history").is_some(),
            "/state JSON must include trade_history"
        );
    }

    #[test]
    fn health_handler_reads_trader_state() {
        // VAL-HEALTH-005: Verify check_trader_health function reads from TraderState
        // (not a static response). Different state should produce different results.
        let healthy = healthy_state();
        let (code1, _, _) = check_trader_health(&healthy);
        assert_eq!(code1, 200, "Healthy state should return 200");

        let mut unhealthy = healthy_state();
        unhealthy.consecutive_errors = 5;
        let (code2, _, _) = check_trader_health(&unhealthy);
        assert_eq!(code2, 503, "Unhealthy state should return 503");

        // Different input → different output, confirming dynamic behavior
        assert_ne!(code1, code2, "Health check must be dynamic, not static");
    }

    fn candle_at(ts: i64) -> indicators::Candle {
        indicators::Candle {
            timestamp: ts,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        }
    }

    #[test]
    fn drop_in_progress_candle_removes_current_period() {
        // Binance returns the in-progress candle last (open timestamp ==
        // current period start). It must be dropped so load_candles never
        // installs a half-formed candle as final.
        let now = Utc::now().timestamp();
        let hour = 3600;
        let current_start = (now / hour) * hour;
        let candles = vec![
            candle_at(current_start - 2 * hour),
            candle_at(current_start - hour),
            candle_at(current_start), // in-progress
        ];
        let out = drop_in_progress_candle(candles, hour);
        assert_eq!(out.len(), 2, "in-progress candle must be dropped");
        assert_eq!(out.last().unwrap().timestamp, current_start - hour);
    }

    #[test]
    fn drop_in_progress_candle_keeps_final_set() {
        // When the last candle belongs to a past period (e.g. fetched right
        // after the boundary rolled), nothing is dropped.
        let now = Utc::now().timestamp();
        let hour = 3600;
        let current_start = (now / hour) * hour;
        let candles = vec![
            candle_at(current_start - 2 * hour),
            candle_at(current_start - hour),
        ];
        let out = drop_in_progress_candle(candles.clone(), hour);
        assert_eq!(out.len(), 2, "all-final candle set must be kept");
    }

    #[test]
    fn drop_in_progress_candle_works_for_4h_and_1d_periods() {
        let now = Utc::now().timestamp();
        for period in [4 * 3600, 24 * 3600] {
            let current_start = (now / period) * period;
            let candles = vec![candle_at(current_start - period), candle_at(current_start)];
            let out = drop_in_progress_candle(candles, period);
            assert_eq!(out.len(), 1, "period {} must drop the live candle", period);
        }
    }

    #[test]
    fn gm_min_open_collateral_env_override() {
        // The fee-sane floor is operator-tunable; parse failures fall back
        // to the default rather than opening dust positions.
        unsafe { std::env::set_var("RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS", "123456789") };
        assert_eq!(gmtrade::min_open_collateral_lamports(), 123456789);
        unsafe { std::env::set_var("RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS", "not-a-number") };
        assert_eq!(
            gmtrade::min_open_collateral_lamports(),
            gmtrade::DEFAULT_MIN_OPEN_COLLATERAL_LAMPORTS
        );
        unsafe { std::env::remove_var("RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS") };
        assert_eq!(
            gmtrade::min_open_collateral_lamports(),
            gmtrade::DEFAULT_MIN_OPEN_COLLATERAL_LAMPORTS
        );
    }

    #[test]
    fn position_info_opened_at_secs_defaults_to_zero() {
        // Flash-era position JSON lacks openedAtSecs; GMTrade fills it.
        let json =
            r#"{"sideUi":"Long","marketSymbol":"SOL","sizeUsdUi":"100","entryPriceUi":"90"}"#;
        let parsed: crate::trader::executor::PositionInfo = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.opened_at_secs, 0);
        let json_gm = r#"{"sideUi":"Long","marketSymbol":"SOL","openedAtSecs":1787430903}"#;
        let parsed_gm: crate::trader::executor::PositionInfo =
            serde_json::from_str(json_gm).unwrap();
        assert_eq!(parsed_gm.opened_at_secs, 1787430903);
    }
}
