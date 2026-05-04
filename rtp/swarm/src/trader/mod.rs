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
            .unwrap_or_else(|_| "1.0".to_string())
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("Invalid RTP_TRADER_LEVERAGE: {}", e))?;
        let poll_secs: u64 = std::env::var("RTP_TRADER_POLL_SECS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .map_err(|e: std::num::ParseIntError| format!("Invalid RTP_TRADER_POLL_SECS: {}", e))?;
        let dry_run = std::env::var("RTP_TRADER_DRY_RUN").is_ok();

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

    // Load or create state
    let mut state = TraderState::load(&config.state_path).unwrap_or_else(|| TraderState::new(&wallet));
    let params = StrategyParams::default();
    let mut buffer = CandleBuffer::new(300); // 300 candles ≈ 12.5 days of 1h data

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
        state.last_poll = cycle_start.to_rfc3339();

        // 1. Fetch current SOL price from Flash Trade
        match executor::get_sol_price().await {
            Ok(price) => {
                buffer.append_tick(price, cycle_start.timestamp());
                tracing::info!(
                    "[POLL] SOL=${:.2} | candles={} | pos={}",
                    price,
                    buffer.len(),
                    if state.open_position.is_some() { "OPEN" } else { "FLAT" }
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
        if let Some(ref pos) = state.open_position {
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

                if let Some(reason) = exit {
                    tracing::info!("[EXIT] {:?} triggered!", reason);

                    if !config.dry_run {
                        // Fetch position key from Flash Trade
                        match executor::get_positions(&wallet).await {
                            Ok(positions) => {
                                if let Some(pos_info) = positions.iter().find(|p| {
                                    p.market_symbol == "SOL" && p.side_ui == "Long"
                                }) {
                                    match executor::close_position(
                                        &keypair,
                                        &pos_info.key,
                                        &pos_info.size_usd_ui,
                                    )
                                    .await
                                    {
                                        Ok((sig, pnl)) => {
                                            tracing::info!("[EXIT] TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", sig);
                                            tracing::info!("[EXIT] PnL: ${:.4}", pnl);

                                            // Record trade
                                            let trade = TradeRecord {
                                                entry_price: pos.entry_price,
                                                exit_price: closes.last().copied().unwrap_or(0.0),
                                                entry_time: pos.entry_time,
                                                exit_time: now_secs,
                                                pnl_pct: if pos.entry_price > 0.0 {
                                                    (closes.last().copied().unwrap_or(0.0) - pos.entry_price) / pos.entry_price * 100.0
                                                } else { 0.0 },
                                                exit_reason: format!("{:?}", reason),
                                                size_usd: pos.size_usd,
                                            };
                                            state.trade_history.push(trade);
                                            state.total_trades += 1;
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

                    state.open_position = None;
                } else {
                    // Update peak price for trailing stop
                    let current_price = closes.last().copied().unwrap_or(0.0);
                    if let Some(ref mut pos) = state.open_position {
                        if current_price > pos.peak_price {
                            pos.peak_price = current_price;
                        }
                    }
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

                                // Get position key
                                let pos_key = executor::get_positions(&wallet)
                                    .await
                                    .ok()
                                    .and_then(|p| p.into_iter().find(|p| p.market_symbol == "SOL" && p.side_ui == "Long"))
                                    .map(|p| p.key)
                                    .unwrap_or_default();

                                state.open_position = Some(OpenPosition {
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
        state.candle_count = buffer.len();
        if let Err(e) = state.save(&config.state_path) {
            tracing::warn!("[STATE] Save failed: {}", e);
        }

        // 5. Sleep
        let elapsed = (Utc::now() - cycle_start).num_seconds() as u64;
        let sleep_secs = config.poll_secs.saturating_sub(elapsed);
        if sleep_secs > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
        }
    }
}
