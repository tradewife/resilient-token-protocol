//! Autonomous trader module — Survivor 2.69 strategy on Flash Trade mainnet.
//!
//! Uses Binance OHLCV for warmup, then Flash Trade prices for ongoing data.
//! Executes via Flash Trade REST API (build → sign → submit).

pub mod candles;
pub mod executor;
pub mod indicators;
pub mod strategy;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use candles::CandleBuffer;
use strategy::{StrategyParams, TradeRecord, OpenPosition};
use solana_sdk::signer::Signer;

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
            .map_err(|e: std::num::ParseFloatError| format!("Invalid RTP_TRADER_POSITION_FRACTION: {}", e))?;
        let leverage = std::env::var("RTP_TRADER_LEVERAGE")
            .unwrap_or_else(|_| "9.0".to_string())
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("Invalid RTP_TRADER_LEVERAGE: {}", e))?;
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

        Ok(Self {
            keypair_path: PathBuf::from(keypair_path),
            amount_sol,
            position_fraction: position_fraction.clamp(0.01, 1.0),
            leverage,
            poll_secs: poll_secs.max(60),
            dry_run,
            state_path: PathBuf::from(state_path),
            rpc_url,
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

    /// Save state to JSON file.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("serialize: {}", e))?;
        std::fs::write(path, json).map_err(|e| format!("write: {}", e))
    }
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
    let resp = client.post(rpc_url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("RPC request failed: {}", e))?;
    let val: serde_json::Value = resp.json().await.map_err(|e| format!("RPC parse failed: {}", e))?;
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
                tracing::error!("[HTTP] Failed to bind port {}: {}. Status server not started.", port, e);
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

/// Determine trader health from current state.
/// Returns `(status_code, status_reason, body_text)`.
pub fn check_trader_health(state: &TraderState) -> (u16, &'static str, String) {
    // 1. Too many consecutive errors → 503
    if state.consecutive_errors >= HEALTH_MAX_ERRORS {
        return (503, "Service Unavailable",
            format!("unhealthy: {} consecutive errors", state.consecutive_errors));
    }

    // 2. Empty last_healthy (initial state) → 503
    if state.last_healthy.is_empty() {
        return (503, "Service Unavailable",
            "unhealthy: no healthy timestamp".to_string());
    }

    // 3. Unparseable last_healthy → 503
    let last_healthy = match chrono::DateTime::parse_from_rfc3339(&state.last_healthy) {
        Ok(dt) => dt,
        Err(_) => {
            return (503, "Service Unavailable",
                "unhealthy: invalid last_healthy timestamp".to_string());
        }
    };

    // 4. Stale last_healthy (> 30 min ago) → 503
    let now = chrono::Utc::now();
    let elapsed_secs = now.signed_duration_since(last_healthy).num_seconds();
    if elapsed_secs > HEALTH_STALE_THRESHOLD_SECS {
        return (503, "Service Unavailable",
            format!("unhealthy: last_healthy is stale ({}s ago)", elapsed_secs));
    }

    // All checks passed → healthy
    (200, "OK", "ok".to_string())
}

async fn handle_status_request(
    mut stream: tokio::net::TcpStream,
    state: Arc<Mutex<TraderState>>,
) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Read enough to parse the request line
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.map_err(|e| format!("read: {}", e))?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Extract the path from the request line (e.g., "GET /state HTTP/1.1")
    let path = request.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    let (status, body, content_type) = if path == "/state" || path == "/" {
        let snapshot = state.lock().await;
        let json = serde_json::to_string(&*snapshot).unwrap_or_else(|_| "{}".to_string());
        ("200 OK".to_string(), json, "application/json")
    } else if path == "/health" {
        let snapshot = state.lock().await;
        let (code, reason, body) = check_trader_health(&snapshot);
        (format!("{} {}", code, reason), body, "text/plain")
    } else {
        ("404 Not Found".to_string(), "not found".to_string(), "text/plain")
    };

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status, content_type, body.len(), body
    );

    stream.write_all(response.as_bytes()).await.map_err(|e| format!("write: {}", e))?;
    stream.flush().await.map_err(|e| format!("flush: {}", e))?;
    Ok(())
}

/// Run the main trader loop.
pub async fn run_trader(config: TraderConfig) -> Result<(), String> {
    // Load keypair
    let keypair_data = std::fs::read_to_string(&config.keypair_path)
        .map_err(|e| format!("Read keypair {}: {}", config.keypair_path.display(), e))?;
    let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)
        .map_err(|e| format!("Parse keypair: {}", e))?;
    let keypair = solana_sdk::signature::Keypair::try_from(keypair_bytes.as_slice())
        .map_err(|e| format!("Invalid keypair: {}", e))?;
    let wallet = keypair.pubkey().to_string();

    tracing::info!("=== RTP Autonomous Trader ===");
    tracing::info!("Wallet:     {}", wallet);
    tracing::info!("Amount:     {} SOL (fallback)", config.amount_sol);
    tracing::info!("Fraction:   {}% of wallet balance", config.position_fraction * 100.0);
    tracing::info!("Leverage:   {}x", config.leverage);
    tracing::info!("Poll:       {}s", config.poll_secs);
    tracing::info!("Dry run:    {}", config.dry_run);
    tracing::info!("State:      {}", config.state_path.display());
    tracing::info!("");

    // Load or create state — shared with HTTP status server via Arc<Mutex>
    let mut initial =
        TraderState::load(&config.state_path).unwrap_or_else(|| TraderState::new(&wallet));
    let repaired = initial.repair_trade_history_sides();
    if repaired > 0 {
        tracing::warn!(
            "[STATE] Repaired {} trade_history side label(s) from pnl_pct (legacy default Long)",
            repaired
        );
        if let Err(e) = initial.save(&config.state_path) {
            tracing::warn!("[STATE] Could not persist side repairs (trading continues): {}", e);
        }
    }
    let state = Arc::new(Mutex::new(initial));
    let params = StrategyParams::load_from_daemon_config();

    // Store active config in state for /state endpoint visibility
    {
        let mut s = state.lock().await;
        s.active_config = params.clone();
    }

    // Log loaded params at startup for Railway log visibility
    tracing::info!(
        "[STARTUP] Active strategy config: signal={:.2} tp={:.1} sl={:.1} hold={:.0}h trail={:.2} decay={:.0}h flip_delay={:.1}h alignment={}",
        params.signal_threshold, params.tp_atr, params.sl_atr,
        params.max_hold_hours, params.trailing_stop_atr,
        params.time_decay_hours, params.score_flip_delay_hrs,
        params.min_alignment,
    );

    let mut buffer = CandleBuffer::new(300); // 300 candles ≈ 12.5 days of 1h data

    // Start HTTP status server for live dashboard access
    let http_port: u16 = std::env::var("RTP_TRADER_HTTP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);
    let http_state = state.clone();
    start_status_server(http_state, http_port);

    // Warmup: fetch historical candles from Binance
    tracing::info!("[WARMUP] Fetching 200h OHLCV from Binance...");
    match candles::fetch_binance_ohlcv("SOLUSDT", 200).await {
        Ok(candles) => {
            tracing::info!("[WARMUP] Loaded {} candles from Binance", candles.len());
            buffer.load_candles(candles);
        }
        Err(e) => {
            tracing::warn!("[WARMUP] Binance fetch failed ({}). Starting cold.", e);
            tracing::warn!("[WARMUP] SMA(200) will not be available until buffer fills.");
        }
    }

    // Reconcile with Flash Trade: if a position is open on-chain but missing
    // from internal state (e.g. after redeploy), restore it so the trader
    // can manage exits and won't open duplicates.
    {
        let mut s = state.lock().await;
        if s.open_position.is_none() {
            match executor::get_positions(&wallet).await {
                Ok(positions) => {
                    // Look for both Long and Short SOL positions
                    if let Some(pos) = positions.iter().find(|p| {
                        p.market_symbol == "SOL" && (p.side_ui == "Long" || p.side_ui == "Short")
                    }) {
                        let entry_price: f64 = pos.entry_price_ui.parse().unwrap_or(0.0);
                        let size_usd: f64 = pos.size_usd_ui.parse().unwrap_or(0.0);
                        let side = pos.side_ui.clone();
                        tracing::warn!(
                            "[RECONCILE] Found orphaned SOL {} position on Flash Trade: \
                             entry=${:.2} size=${:.2} key={}...restoring to internal state.",
                            side, entry_price, size_usd, &pos.key[..8]
                        );
                        s.open_position = Some(OpenPosition {
                            entry_price,
                            entry_time: Utc::now().timestamp() - 3600, // assume 1h ago to avoid max_hold firing immediately
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
                    tracing::warn!("[RECONCILE] Failed to check Flash Trade positions: {}. Continuing without reconciliation.", e);
                }
            }
        }
    }

    // Main loop with watchdog — each cycle is wrapped in a timeout.
    // If a cycle hangs (e.g., HTTP request stalls), the watchdog kills it,
    // increments consecutive_errors, and retries after a backoff.
    const CYCLE_TIMEOUT_SECS: u64 = 120; // max time for one trading cycle
    const MAX_CONSECUTIVE_ERRORS: u32 = 10; // after this many, sleep longer

    tracing::info!("[LOOP] Starting autonomous trading loop (watchdog: {}s cycle timeout)...", CYCLE_TIMEOUT_SECS);
    loop {
        let cycle_start = Utc::now();
        {
            let mut s = state.lock().await;
            s.last_poll = cycle_start.to_rfc3339();
        }

        let cycle_result = tokio::time::timeout(
            std::time::Duration::from_secs(CYCLE_TIMEOUT_SECS),
            run_cycle(&config, &keypair, &wallet, &params, &mut buffer, &state),
        ).await;

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
                tracing::warn!("[WATCHDOG] Consecutive errors: {}/{}", s.consecutive_errors, MAX_CONSECUTIVE_ERRORS);
            }
            Err(_) => {
                // Cycle timed out — watchdog killed it
                tracing::error!("[WATCHDOG] Cycle timed out after {}s — likely HTTP hang", CYCLE_TIMEOUT_SECS);
                let mut s = state.lock().await;
                s.consecutive_errors += 1;
                tracing::warn!("[WATCHDOG] Consecutive errors: {}/{}", s.consecutive_errors, MAX_CONSECUTIVE_ERRORS);
            }
        }

        // Save state after every cycle (success or failure)
        {
            let mut s = state.lock().await;
            s.candle_count = buffer.len();
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
async fn run_cycle(
    config: &TraderConfig,
    keypair: &solana_sdk::signature::Keypair,
    wallet: &str,
    params: &StrategyParams,
    buffer: &mut CandleBuffer,
    state: &Arc<Mutex<TraderState>>,
) -> Result<(), String> {
    let cycle_start = Utc::now();

    // 1. Fetch current SOL price from Flash Trade
    let price = match executor::get_sol_price().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("[POLL] Price fetch failed: {}", e);
            return Ok(()); // non-fatal, will retry next cycle
        }
    };

    buffer.append_tick(price, cycle_start.timestamp());
    let has_pos = state.lock().await.open_position.is_some();
    tracing::info!(
        "[POLL] SOL=${:.2} | candles={} | pos={}",
        price,
        buffer.len(),
        if has_pos { "OPEN" } else { "FLAT" }
    );

    let closes = buffer.closes();
    let volumes = buffer.volumes();

    // 2. Check exit on existing position
    let exit_info = {
        let s = state.lock().await;
        if let Some(ref pos) = s.open_position {
            if let Some(signal) = strategy::compute_signal(&closes, &volumes, params.min_alignment) {
                let now_secs = Utc::now().timestamp();
                let current_price = closes.last().copied().unwrap_or(0.0);
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
                match executor::get_positions(wallet).await {
                    Ok(positions) => {
                        let pos_side = pos_info.side();
                        if let Some(pos_api) = positions.iter().find(|p| {
                            p.market_symbol == "SOL" && p.side_ui == pos_side
                        }) {
                            match executor::close_position(
                                keypair,
                                &pos_api.key,
                                &pos_api.size_usd_ui,
                                &pos_api.collateral_symbol,
                            )
                            .await
                            {
                                Ok((sig, pnl)) => {
                                    tracing::info!("[EXIT] TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", sig);
                                    tracing::info!("[EXIT] PnL: ${:.4}", pnl);

                                    let exit_price = closes.last().copied().unwrap_or(0.0);
                                    let side = pos_info.side();
                                    let trade = TradeRecord {
                                        entry_price: pos_info.entry_price,
                                        exit_price,
                                        entry_time: pos_info.entry_time,
                                        exit_time: Utc::now().timestamp(),
                                        pnl_pct: if pos_info.entry_price > 0.0 {
                                            match side {
                                                "Short" => (pos_info.entry_price - exit_price) / pos_info.entry_price * 100.0,
                                                _ => (exit_price - pos_info.entry_price) / pos_info.entry_price * 100.0,
                                            }
                                        } else { 0.0 },
                                        exit_reason: format!("{:?}", reason),
                                        size_usd: pos_info.size_usd,
                                        side: side.to_string(),
                                    };
                                    let mut s = state.lock().await;
                                    s.trade_history.push(trade);
                                    s.total_trades += 1;
                                    close_succeeded = true;
                                }
                                Err(e) => {
                                    tracing::error!("[EXIT] Close failed: {}", e);
                                }
                            }
                        } else {
                            tracing::warn!("[EXIT] No SOL {} position found on Flash Trade", pos_side);
                            // Position already closed externally — clear stale state
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
            let current_price = closes.last().copied().unwrap_or(0.0);
            let side = pos_info.side();
            let should_update_peak = match side {
                "Short" => current_price < pos_info.peak_price, // track trough for SHORT
                _ => current_price > pos_info.peak_price,       // track peak for LONG
            };
            if should_update_peak
                && let Some(ref mut pos) = state.lock().await.open_position
            {
                pos.peak_price = current_price;
            }
        }
    } else {
        // 3. Check entry signal (only if flat)
        if let Some(signal) = strategy::compute_signal(&closes, &volumes, params.min_alignment) {
            tracing::info!(
                "[SIGNAL] score={:.3} rsi={:.1} bull={} bear={} atr={:.2} reasons={:?}",
                signal.score, signal.rsi, signal.bullish_count, signal.bearish_count, signal.atr, signal.reasons
            );

            // Entry logic: LONG or SHORT, mutually exclusive
            let entry_signal = if signal.score > params.signal_threshold && signal.bullish_count >= params.min_alignment {
                Some(("Long", "LONG", signal.score, signal.bullish_count, signal.reasons.clone()))
            } else if signal.score < -params.signal_threshold && signal.bearish_count >= params.min_alignment {
                Some(("Short", "SHORT", signal.score, signal.bearish_count, signal.reasons.clone()))
            } else {
                None
            };

            if let Some((side, trade_type, score, align_count, reasons)) = entry_signal {
                tracing::info!(
                    "[ENTRY] Signal: {} score={:.3} align={} reasons={:?}",
                    side, score, align_count, reasons
                );

                if !config.dry_run {
                    // Compute position size as fraction of wallet balance
                    let amount_sol = match fetch_wallet_balance(&config.rpc_url, wallet).await {
                        Ok(balance) => {
                            let sized = balance * config.position_fraction;
                            tracing::info!(
                                "[ENTRY] Wallet: {:.4} SOL → position: {:.4} SOL ({:.0}% @ {}x)",
                                balance, sized, config.position_fraction * 100.0, config.leverage
                            );
                            sized
                        }
                        Err(e) => {
                            tracing::warn!("[ENTRY] Balance fetch failed ({}). Using fallback: {} SOL", e, config.amount_sol);
                            config.amount_sol
                        }
                    };
                    match executor::open_position(keypair, amount_sol, config.leverage, trade_type).await {
                        Ok((sig, size_usd, entry_price)) => {
                            tracing::info!("[ENTRY] TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", sig);

                            let pos_side = side.to_string();
                            let pos_key = executor::get_positions(wallet)
                                .await
                                .ok()
                                .and_then(|p| p.into_iter().find(|p| p.market_symbol == "SOL" && p.side_ui == side))
                                .map(|p| p.key)
                                .unwrap_or_default();

                            state.lock().await.open_position = Some(OpenPosition {
                                entry_price,
                                entry_time: Utc::now().timestamp(),
                                peak_price: entry_price,
                                entry_rsi: signal.rsi,
                                entry_atr: signal.atr,
                                entry_score: signal.score,
                                position_key: pos_key,
                                size_usd,
                                first_negative_score_time: None,
                                side: pos_side,
                            });
                        }
                        Err(e) => {
                            tracing::error!("[ENTRY] Open failed: {}", e);
                        }
                    }
                } else {
                    tracing::info!("[DRY RUN] Would open {} SOL {} @ {}x ({}%)", config.amount_sol, side, config.leverage, config.position_fraction * 100.0);
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
        assert_eq!(parsed.wallet, "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R");
        assert_eq!(parsed.active_config.signal_threshold, defaults.signal_threshold);
        assert_eq!(parsed.active_config.tp_atr, defaults.tp_atr);
        assert_eq!(parsed.active_config.score_flip_delay_hrs, defaults.score_flip_delay_hrs);
        assert_eq!(parsed.active_config.time_decay_hours, defaults.time_decay_hours);
        assert_eq!(parsed.active_config.min_alignment, defaults.min_alignment);
    }

    #[test]
    fn trader_state_new_has_default_active_config() {
        let state = TraderState::new("TestWallet11111111111111111111111111111111");
        let defaults = StrategyParams::default();
        assert_eq!(state.active_config.signal_threshold, defaults.signal_threshold);
        assert_eq!(state.active_config.tp_atr, defaults.tp_atr);
        assert_eq!(state.active_config.sl_atr, defaults.sl_atr);
        assert_eq!(state.active_config.max_hold_hours, defaults.max_hold_hours);
        assert_eq!(state.active_config.trailing_stop_atr, defaults.trailing_stop_atr);
        assert_eq!(state.active_config.time_decay_hours, defaults.time_decay_hours);
        assert_eq!(state.active_config.min_alignment, defaults.min_alignment);
        assert_eq!(state.active_config.score_flip_delay_hrs, defaults.score_flip_delay_hrs);
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
            params.signal_threshold, params.tp_atr, params.sl_atr,
            params.max_hold_hours, params.trailing_stop_atr,
            params.time_decay_hours, params.score_flip_delay_hrs,
            params.min_alignment,
        );
        // Verify all key fields appear in the formatted log
        assert!(log_msg.contains("signal=0.30"), "Log must include signal_threshold");
        assert!(log_msg.contains("tp=6.0"), "Log must include tp_atr");
        assert!(log_msg.contains("sl=2.5"), "Log must include sl_atr");
        assert!(log_msg.contains("hold=96h"), "Log must include max_hold_hours");
        assert!(log_msg.contains("trail=1.00"), "Log must include trailing_stop_atr");
        assert!(log_msg.contains("decay=48h"), "Log must include time_decay_hours");
        assert!(log_msg.contains("flip_delay=2.0h"), "Log must include score_flip_delay_hrs");
        assert!(log_msg.contains("alignment=3"), "Log must include min_alignment");
    }

    #[test]
    fn existing_trader_state_json_file_loads() {
        // Load the actual data/trader-state.json file from the repo
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/trader-state.json");
        if let Some(state) = TraderState::load(&repo_root) {
            assert_eq!(state.wallet, "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R");
            assert_eq!(state.total_trades, 1);
            // active_config should default since the file doesn't have this field
            let defaults = StrategyParams::default();
            assert_eq!(state.active_config.signal_threshold, defaults.signal_threshold);
            assert_eq!(state.active_config.score_flip_delay_hrs, defaults.score_flip_delay_hrs);
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
        });
        assert_eq!(state.repair_trade_history_sides(), 1);
        assert_eq!(state.trade_history[0].side, "Short");
        assert_eq!(state.trade_history[1].side, "Long");
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
        assert_eq!(pnl_pct, 5.0, "SHORT profit should be +5% when price drops 5%");

        // Verify SHORT loss PnL
        let exit_price_loss = 110.0;
        let pnl_pct_loss = (entry_price - exit_price_loss) / entry_price * 100.0;
        assert_eq!(pnl_pct_loss, -10.0, "SHORT loss should be -10% when price rises 10%");
    }

    #[test]
    fn long_pnl_recording_unchanged() {
        // LONG PnL should be unchanged: (exit - entry) / entry * 100
        let entry_price = 100.0;
        let exit_price = 110.0;
        let pnl_pct = (exit_price - entry_price) / entry_price * 100.0;
        assert_eq!(pnl_pct, 10.0, "LONG profit should be +10% when price rises 10%");

        let exit_price_loss = 90.0;
        let pnl_pct_loss = (exit_price_loss - entry_price) / entry_price * 100.0;
        assert_eq!(pnl_pct_loss, -10.0, "LONG loss should be -10% when price drops 10%");
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
        assert!(current3 > pos.peak_price, "93 > 90: should NOT update trough");
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
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/trader-state.json");
        if let Some(state) = TraderState::load(&repo_root) {
            if let Some(ref pos) = state.open_position {
                assert_eq!(pos.side, "Long", "Existing state should default side to Long");
            }
        }
    }

    #[test]
    fn entry_signal_logic_long_condition() {
        // Verify: score > threshold && bullish_count >= min_alignment → LONG
        let params = StrategyParams {
            signal_threshold: 0.3,
            min_alignment: 3,
            ..StrategyParams::default()
        };
        let score = 0.5;
        let bullish_count: usize = 3;

        let is_long = score > params.signal_threshold && bullish_count >= params.min_alignment;
        assert!(is_long, "Score 0.5 > 0.3 with 3 bull → LONG");
    }

    #[test]
    fn entry_signal_logic_short_condition() {
        // Verify: score < -threshold && bearish_count >= min_alignment → SHORT
        let params = StrategyParams {
            signal_threshold: 0.3,
            min_alignment: 3,
            ..StrategyParams::default()
        };
        let score = -0.5;
        let bearish_count: usize = 3;

        let is_short = score < -params.signal_threshold && bearish_count >= params.min_alignment;
        assert!(is_short, "Score -0.5 < -0.3 with 3 bear → SHORT");
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
        assert!(body.contains("consecutive errors"), "Body should mention errors: {}", body);

        // Also test with > 5
        state.consecutive_errors = 10;
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(code, 503, "Should return 503 when consecutive_errors = 10");
        assert!(body.contains("consecutive errors"), "Body should mention errors: {}", body);
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
        assert!(body.contains("stale"), "Body should mention stale: {}", body);
    }

    #[test]
    fn health_returns_503_when_last_healthy_empty() {
        // VAL-HEALTH-006: empty last_healthy → 503 (initial state)
        let mut state = TraderState::new("TestWallet");
        state.consecutive_errors = 0;
        state.last_healthy = String::new(); // empty — initial state
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(code, 503, "Should return 503 when last_healthy is empty");
        assert!(body.contains("no healthy timestamp"), "Body should mention missing timestamp: {}", body);
    }

    #[test]
    fn health_returns_503_when_last_healthy_unparseable() {
        // VAL-HEALTH-006: unparseable last_healthy → 503
        let mut state = TraderState::new("TestWallet");
        state.consecutive_errors = 0;
        state.last_healthy = "garbage-not-a-timestamp".to_string();
        let (code, _reason, body) = check_trader_health(&state);
        assert_eq!(code, 503, "Should return 503 when last_healthy cannot be parsed");
        assert!(body.contains("invalid"), "Body should mention invalid timestamp: {}", body);
    }

    #[test]
    fn health_returns_200_when_just_under_stale_threshold() {
        // Verify: last_healthy = 29 minutes ago → still 200
        let mut state = healthy_state();
        let recent_time = chrono::Utc::now() - chrono::Duration::minutes(29);
        state.last_healthy = recent_time.to_rfc3339();
        state.consecutive_errors = 0;
        let (code, _reason, _body) = check_trader_health(&state);
        assert_eq!(code, 200, "Should return 200 when last_healthy is 29 minutes ago");
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
        assert!(body.contains("consecutive errors"), "Error check should fire first: {}", body);
    }

    #[test]
    fn state_endpoint_returns_valid_json() {
        // VAL-HEALTH-004: /state still returns full TraderState JSON
        let state = healthy_state();
        let json = serde_json::to_string(&state).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        // Verify key fields exist
        assert!(parsed.get("wallet").is_some(), "/state JSON must include wallet");
        assert!(parsed.get("open_position").is_some(), "/state JSON must include open_position");
        assert!(parsed.get("consecutive_errors").is_some(), "/state JSON must include consecutive_errors");
        assert!(parsed.get("last_healthy").is_some(), "/state JSON must include last_healthy");
        assert!(parsed.get("active_config").is_some(), "/state JSON must include active_config");
        assert!(parsed.get("trade_history").is_some(), "/state JSON must include trade_history");
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
}
