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
    pub leverage: f64,
    pub poll_secs: u64,
    pub dry_run: bool,
    pub state_path: PathBuf,
}

impl TraderConfig {
    pub fn from_env() -> Result<Self, String> {
        let keypair_path = std::env::var("RTP_TRADER_KEYPAIR")
            .map_err(|_| "RTP_TRADER_KEYPAIR not set".to_string())?;
        let amount_sol = std::env::var("RTP_TRADER_AMOUNT")
            .unwrap_or_else(|_| "0.20".to_string())
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("Invalid RTP_TRADER_AMOUNT: {}", e))?;
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

        Ok(Self {
            keypair_path: PathBuf::from(keypair_path),
            amount_sol,
            leverage,
            poll_secs: poll_secs.max(60),
            dry_run,
            state_path: PathBuf::from(state_path),
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
        }
    }

    /// Load state from JSON file.
    pub fn load(path: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
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
        ("200 OK", json, "application/json")
    } else if path == "/health" {
        ("200 OK", "ok".to_string(), "text/plain")
    } else {
        ("404 Not Found", "not found".to_string(), "text/plain")
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
    tracing::info!("Amount:     {} SOL", config.amount_sol);
    tracing::info!("Leverage:   {}x", config.leverage);
    tracing::info!("Poll:       {}s", config.poll_secs);
    tracing::info!("Dry run:    {}", config.dry_run);
    tracing::info!("State:      {}", config.state_path.display());
    tracing::info!("");

    // Load or create state — shared with HTTP status server via Arc<Mutex>
    let initial = TraderState::load(&config.state_path).unwrap_or_else(|| TraderState::new(&wallet));
    let state = Arc::new(Mutex::new(initial));
    let params = StrategyParams::default();
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

    // Main loop
    tracing::info!("[LOOP] Starting autonomous trading loop...");
    loop {
        let cycle_start = Utc::now();
        {
            let mut s = state.lock().await;
            s.last_poll = cycle_start.to_rfc3339();
        }

        // 1. Fetch current SOL price from Flash Trade
        match executor::get_sol_price().await {
            Ok(price) => {
                buffer.append_tick(price, cycle_start.timestamp());
                let has_pos = state.lock().await.open_position.is_some();
                tracing::info!(
                    "[POLL] SOL=${:.2} | candles={} | pos={}",
                    price,
                    buffer.len(),
                    if has_pos { "OPEN" } else { "FLAT" }
                );
            }
            Err(e) => {
                tracing::warn!("[POLL] Price fetch failed: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(config.poll_secs)).await;
                continue;
            }
        }

        let closes = buffer.closes();
        let volumes = buffer.volumes();

        // 2. Check exit on existing position
        let exit_info = {
            let s = state.lock().await;
            if let Some(ref pos) = s.open_position {
                if let Some(signal) = strategy::compute_signal(&closes, &volumes) {
                    let now_secs = Utc::now().timestamp();
                    let current_price = closes.last().copied().unwrap_or(0.0);
                    let exit = strategy::check_exit(
                        &params,
                        pos.entry_price,
                        pos.entry_time,
                        pos.peak_price,
                        pos.entry_rsi,
                        current_price,
                        signal.score,
                        signal.rsi,
                        signal.atr,
                        now_secs,
                    );
                    Some((exit, pos.clone(), signal.score))
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some((Some(reason), pos_info, _)) = exit_info {
            tracing::info!("[EXIT] {:?} triggered!", reason);

            if !config.dry_run {
                match executor::get_positions(&wallet).await {
                    Ok(positions) => {
                        if let Some(pos_api) = positions.iter().find(|p| {
                            p.market_symbol == "SOL" && p.side_ui == "Long"
                        }) {
                            match executor::close_position(
                                &keypair,
                                &pos_api.key,
                                &pos_api.size_usd_ui,
                            )
                            .await
                            {
                                Ok((sig, pnl)) => {
                                    tracing::info!("[EXIT] TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", sig);
                                    tracing::info!("[EXIT] PnL: ${:.4}", pnl);

                                    let exit_price = closes.last().copied().unwrap_or(0.0);
                                    let trade = TradeRecord {
                                        entry_price: pos_info.entry_price,
                                        exit_price,
                                        entry_time: pos_info.entry_time,
                                        exit_time: Utc::now().timestamp(),
                                        pnl_pct: if pos_info.entry_price > 0.0 {
                                            (exit_price - pos_info.entry_price) / pos_info.entry_price * 100.0
                                        } else { 0.0 },
                                        exit_reason: format!("{:?}", reason),
                                        size_usd: pos_info.size_usd,
                                    };
                                    let mut s = state.lock().await;
                                    s.trade_history.push(trade);
                                    s.total_trades += 1;
                                }
                                Err(e) => {
                                    tracing::error!("[EXIT] Close failed: {}", e);
                                }
                            }
                        } else {
                            tracing::warn!("[EXIT] No SOL Long position found on Flash Trade");
                        }
                    }
                    Err(e) => {
                        tracing::error!("[EXIT] Positions fetch failed: {}", e);
                    }
                }
            } else {
                tracing::info!("[DRY RUN] Would close position: {:?}", reason);
            }

            state.lock().await.open_position = None;
        } else if let Some((None, pos_info, _)) = exit_info {
            // No exit triggered — update peak price for trailing stop
            let current_price = closes.last().copied().unwrap_or(0.0);
            if current_price > pos_info.peak_price {
                if let Some(ref mut pos) = state.lock().await.open_position {
                    pos.peak_price = current_price;
                }
            }
        } else {
            // 3. Check entry signal (only if flat)
            if let Some(signal) = strategy::compute_signal(&closes, &volumes) {
                tracing::debug!(
                    "[SIGNAL] score={:.3} rsi={:.1} bull={} atr={:.2} reasons={:?}",
                    signal.score, signal.rsi, signal.bullish_count, signal.atr, signal.reasons
                );
                if signal.score > params.signal_threshold && signal.bullish_count >= params.min_alignment {
                    tracing::info!(
                        "[ENTRY] Signal: score={:.3} rsi={:.1} bull={} reasons={:?}",
                        signal.score,
                        signal.rsi,
                        signal.bullish_count,
                        signal.reasons
                    );

                    if !config.dry_run {
                        match executor::open_position(&keypair, config.amount_sol, config.leverage).await {
                            Ok((sig, size_usd, entry_price)) => {
                                tracing::info!("[ENTRY] TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", sig);

                                let pos_key = executor::get_positions(&wallet)
                                    .await
                                    .ok()
                                    .and_then(|p| p.into_iter().find(|p| p.market_symbol == "SOL" && p.side_ui == "Long"))
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
                                });
                            }
                            Err(e) => {
                                tracing::error!("[ENTRY] Open failed: {}", e);
                            }
                        }
                    } else {
                        tracing::info!("[DRY RUN] Would open {} SOL LONG @ {}x", config.amount_sol, config.leverage);
                    }
                }
            }
        }

        // 4. Save state
        {
            let mut s = state.lock().await;
            s.candle_count = buffer.len();
            if let Err(e) = s.save(&config.state_path) {
                tracing::warn!("[STATE] Save failed: {}", e);
            }
        }

        // 5. Sleep
        let elapsed = (Utc::now() - cycle_start).num_seconds() as u64;
        let sleep_secs = config.poll_secs.saturating_sub(elapsed);
        if sleep_secs > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
        }
    }
}
