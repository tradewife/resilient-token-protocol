//! RTP Swarm — Devnet Loop Daemon (single-cycle, designed for 6h cron).
//! Loads config → runs one cycle → proposes mutations → writes output to data/devnet-cycles/.

use chrono::Utc;
use rtp_swarm::bridge::read_latest_night_results;
use rtp_swarm::chain_client::{
    ChainConfig, ExecutionMode, FlashMarketAccounts, FlashSide, OraclePrice,
    build_open_flash_position_ix, build_close_flash_position_ix, submit_or_simulate,
};
use rtp_swarm::wings::evolve::{
    LlmProposerConfig, propose_strategy_mutation, validate_all_mutations,
};
use rtp_swarm::wings::trading::{StrategyConfig, apply_mutations};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signer;

/// Cycle output written to data/devnet-cycles/{timestamp}/cycle.json.
#[derive(Debug, Serialize, Deserialize)]
struct CycleOutput {
    cycle_id: String,
    health: CycleHealth,
    retry_count: u32,
    error_message: Option<String>,
    params_used: StrategyConfig,
    mutations_proposed: Vec<rtp_swarm::wings::evolve::StrategyMutation>,
    mutations_accepted: Vec<rtp_swarm::wings::evolve::StrategyMutation>,
    mutations_rejected: Vec<rtp_swarm::wings::evolve::StrategyMutation>,
    params_next: StrategyConfig,
    used_llm: bool,
    model_label: String,
    memory_files: Vec<String>,
}

/// Health status of a completed cycle.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum CycleHealth {
    /// All steps completed successfully.
    Healthy,
    /// Cycle completed but some non-critical steps failed.
    Degraded,
    /// Cycle failed — could not complete core steps.
    Failed,
}

/// Resolve the repo root (two levels up from CARGO_MANIFEST_DIR, i.e. rtp/swarm/).
fn repo_root() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&manifest)
        .join("../../")
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// Load config from data/devnet-cycles/latest/config.json or use defaults.
fn load_config() -> StrategyConfig {
    let path = repo_root().join("data/devnet-cycles/latest/config.json");
    if path.exists() {
        let path_display = path.display().to_string();
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => {
                    tracing::info!("[DAEMON] loaded config from {}", path_display);
                    return config;
                }
                Err(e) => tracing::info!("[DAEMON] config parse error: {} — using defaults", e),
            },
            Err(e) => tracing::info!("[DAEMON] config read error: {} — using defaults", e),
        }
    } else {
        tracing::info!("[DAEMON] no prior config found — using SOL/USDT Survivor 2.69 defaults");
    }
    StrategyConfig::default()
}

/// Collect memory file paths from data/swarm-memory.
fn collect_memory_files() -> Vec<String> {
    let base = std::path::Path::new("data/swarm-memory");
    let mut files = Vec::new();
    if base.exists() {
        for subdir in &["working", "project", "overview"] {
            let dir = base.join(subdir);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    files.push(entry.path().display().to_string());
                }
            }
        }
    }
    files
}

/// Minimal Anchor-compatible Treasury struct for decoding on-chain state.
/// Must match the field order in rtp-treasury/src/lib.rs exactly.
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct TreasuryAccount {
    mint: solana_sdk::pubkey::Pubkey,       // 32
    authority: solana_sdk::pubkey::Pubkey,    // 32
    phase: u8,                                // 1 (enum)
    total_fees_withdrawn: u64,               // 8
    total_distributed_holders: u64,           // 8
    total_distributed_dev: u64,               // 8
    total_distributed_ecosystem: u64,         // 8
    total_hydration: u64,                     // 8
    total_fees_received_lamports: u64,        // 8
    holders_wallet: solana_sdk::pubkey::Pubkey, // 32
    project_dev_wallet: solana_sdk::pubkey::Pubkey, // 32
    ecosystem_wallet: solana_sdk::pubkey::Pubkey, // 32
    min_runway_balance: u64,                  // 8
    frozen: bool,                             // 1
    bump: u8,                                 // 1
}

/// Check for stale positions that have exceeded max_hold_hours * 1.1.
///
/// Queries the Flash Trade REST API for open positions belonging to the
/// treasury PDA. Any position open longer than the timeout is logged and,
/// when `RTP_EXECUTION_MODE != demo` and a chain config is available, the
/// daemon builds a `close_flash_position` instruction and either simulates
/// or submits it depending on the active execution mode.
async fn check_stale_positions(
    max_hold_hours: f64,
    chain_cfg: Option<&ChainConfig>,
) -> Vec<StalePositionAction> {
    let mut actions: Vec<StalePositionAction> = Vec::new();
    let Some(cfg) = chain_cfg else {
        tracing::info!(
            "[DAEMON] stale-position check skipped — RTP_MINT not set, no treasury PDA derivable"
        );
        return actions;
    };
    let treasury_pda = cfg.treasury_pda.to_string();
    let timeout_hours = max_hold_hours * 1.1;

    let client = rtp_swarm::wings::trading::flash_trade_client::FlashTradeClient::new();
    match client.get_positions(&treasury_pda).await {
        Ok(positions) => {
            if positions.is_empty() {
                tracing::info!("[DAEMON] no open positions — stale check clean");
                return actions;
            }
            tracing::info!("[DAEMON] {} open position(s) found", positions.len());
            let now = Utc::now();
            for pos in &positions {
                // Parse created_at — Flash Trade returns ISO 8601 timestamps.
                let opened = chrono::DateTime::parse_from_rfc3339(&pos.created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|e| {
                        tracing::info!(
                            "[DAEMON] warning: could not parse created_at '{}' — {}",
                            pos.created_at, e
                        );
                        Utc::now()
                    });
                let age_hours = (now - opened).num_seconds() as f64 / 3600.0;
                let stale = age_hours > timeout_hours;
                tracing::info!(
                    "[DAEMON] position {} — side={}, size_usd={}, age={:.1}h{}",
                    &pos.position_address[..8],
                    pos.side,
                    pos.size_usd,
                    age_hours,
                    if stale { " *** STALE ***" } else { "" }
                );
                if stale {
                    tracing::info!(
                        "[DAEMON] queued close_flash_position for stale position {} ({:.1}h > {:.1}h)",
                        pos.position_address, age_hours, timeout_hours
                    );
                    actions.push(StalePositionAction {
                        position_address: pos.position_address.clone(),
                        side: pos.side.clone(),
                        size_usd: pos.size_usd.clone(),
                        age_hours,
                    });
                }
            }
        }
        Err(e) => {
            tracing::info!(
                "[DAEMON] could not query Flash Trade positions ({}). \
                 Stale check skipped — on-chain CPI will still gate.",
                e
            );
        }
    }
    actions
}

/// A stale position the daemon decided to close. Captured here so the cycle
/// output can record what was closed (or attempted) and at what age.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StalePositionAction {
    position_address: String,
    side: String,
    size_usd: String,
    age_hours: f64,
}

/// Check if the configured treasury is frozen by reading the on-chain
/// account data. Uses bincode deserialization (Anchor's account layout
/// without the 8-byte discriminator). Returns Ok(true) if frozen, Ok(false)
/// if not, Err if the RPC is unreachable or the account is missing.
fn check_treasury_frozen(cfg: &ChainConfig) -> Result<bool, String> {
    let treasury_pda = cfg.treasury_pda.to_string();
    let rpc_url = cfg.rpc_url.as_str();

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [treasury_pda, { "encoding": "base64" }]
        }))
        .send()
        .map_err(|e| format!("RPC request failed: {}", e))?;

    let json: serde_json::Value = resp.json().map_err(|e| format!("RPC parse error: {}", e))?;

    // Check if account exists
    let value = json
        .get("result")
        .and_then(|r| r.get("value"))
        .ok_or("No result.value in RPC response")?;

    if value.is_null() {
        return Err("Treasury account does not exist on this network".to_string());
    }

    let data = value
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|d| d.as_str())
        .ok_or("No account data returned")?;

    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    // Anchor accounts start with 8-byte discriminator, then Borsh-serialized data.
    if bytes.len() < 8 {
        return Err("Account data too short for Anchor discriminator".to_string());
    }

    // Deserialize the account data (skip 8-byte discriminator).
    let treasury: TreasuryAccount = bincode::deserialize(&bytes[8..])
        .map_err(|e| format!("Treasury deserialize error: {}", e))?;

    tracing::info!(
        "[DAEMON] treasury decoded: mint={}, phase={}, frozen={}, runway={}",
        treasury.mint, treasury.phase, treasury.frozen, treasury.min_runway_balance
    );

    Ok(treasury.frozen)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let watch_mode = std::env::var("RTP_WATCHDOG").is_ok();
    let interval_secs: u64 = std::env::var("RTP_CYCLE_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(21600); // 6h default

    if watch_mode {
        tracing::info!("[DAEMON] Watchdog mode — cycling every {}s until interrupted", interval_secs);
        loop {
            run_cycle_with_retry(3).await;
            tracing::info!("[DAEMON] sleeping {}s until next cycle", interval_secs);
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    } else {
        // Single-shot mode (current behavior, for Railway cron).
        run_cycle_with_retry(3).await;
    }
}

/// Run a single cycle with retry logic.
/// Retries up to `max_retries` times with exponential backoff.
/// Always exits cleanly (exit 0) — never panics or fails the Railway cron.
async fn run_cycle_with_retry(max_retries: u32) {
    for attempt in 1..=max_retries {
        match run_single_cycle().await {
            Ok(()) => return,
            Err(e) => {
                tracing::warn!(
                    "[DAEMON] cycle attempt {}/{} failed: {}",
                    attempt,
                    max_retries,
                    e
                );
                if attempt < max_retries {
                    let delay = 30 * attempt as u64;
                    tracing::info!("[DAEMON] retrying in {}s", delay);
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                } else {
                    // All retries exhausted — write a degraded cycle output.
                    tracing::error!(
                        "[DAEMON] all {} attempts failed. Writing degraded output.",
                        max_retries
                    );
                    let root = repo_root();
                    let now = Utc::now();
                    let cycle_id = now.format("%Y-%m-%dT%H").to_string();
                    let cycle_dir = root.join("data/devnet-cycles").join(&cycle_id);
                    let _ = std::fs::create_dir_all(&cycle_dir);
                    let degraded = CycleOutput {
                        cycle_id: now.to_rfc3339(),
                        health: CycleHealth::Failed,
                        retry_count: max_retries,
                        error_message: Some(e.to_string()),
                        params_used: StrategyConfig::default(),
                        mutations_proposed: vec![],
                        mutations_accepted: vec![],
                        mutations_rejected: vec![],
                        params_next: StrategyConfig::default(),
                        used_llm: false,
                        model_label: "none".to_string(),
                        memory_files: vec![],
                    };
                    let json = serde_json::to_string_pretty(&degraded).unwrap_or_default();
                    let _ = std::fs::write(cycle_dir.join("cycle.json"), &json);
                }
            }
        }
    }
}

/// Run a single daemon cycle. Returns Ok(()) on success, Err on failure.
async fn run_single_cycle() -> Result<(), String> {
    tracing::info!("┌─────────────────────────────────────────────────┐");
    tracing::info!("│  RTP — Devnet Loop Daemon                       │");
    tracing::info!(
        "│  Autonomous cycle: {}        │",
        Utc::now().format("%Y-%m-%d %H:%M UTC")
    );
    tracing::info!("└─────────────────────────────────────────────────┘");
    tracing::info!(" ");

    // 0. Load chain config (env-driven, no hardcoded PDAs).
    let chain_cfg = match ChainConfig::from_env() {
        Ok(cfg) => {
            cfg.log_summary();
            Some(cfg)
        }
        Err(e) => {
            tracing::info!(
                "[DAEMON] chain config not loaded ({}). Running in demo-only mode.",
                e
            );
            None
        }
    };

    // 0a. Check if treasury is frozen before running cycle.
    if let Some(ref cfg) = chain_cfg {
        match check_treasury_frozen(cfg) {
            Ok(true) => {
                tracing::info!(
                    "[DAEMON] Treasury is FROZEN — skipping cycle. Unfreeze required by authority."
                );
                tracing::info!("[DAEMON] exit 0 (frozen is not an error)");
                return Ok(());
            }
            Ok(false) => {
                tracing::info!("[DAEMON] Treasury status: ACTIVE");
            }
            Err(e) => {
                tracing::info!(
                    "[DAEMON] Could not check frozen status ({}). Continuing — on-chain CPI will gate.",
                    e
                );
            }
        }
    }

    // 0b. Execution mode (replaces legacy RTP_MAINNET_EXECUTE).
    let execution_mode = chain_cfg.as_ref().map(|c| c.mode).unwrap_or(ExecutionMode::Simulate);
    tracing::info!(" ");
    tracing::info!("[DAEMON] execution mode: {}", execution_mode.label());
    if execution_mode.submits() {
        tracing::info!(
            "[DAEMON] *** {} — real transactions will be sent ***",
            execution_mode.label().to_uppercase()
        );
    }

    // 1. Load config.
    let mut config = load_config();
    tracing::info!(
        "[DAEMON] params: signal_threshold={}, tp_atr={}, sl_atr={}, max_hold={}h, trailing_stop={}",
        config.signal_threshold,
        config.tp_atr,
        config.sl_atr,
        config.max_hold_hours,
        config.trailing_stop_atr
    );

    // 1b. Read latest Night Shift results.
    tracing::info!(" ");
    tracing::info!("=== NIGHT SHIFT RESULTS ===");
    match read_latest_night_results() {
        Ok(result) => {
            tracing::info!("[DAEMON] latest results: {}", result.source_path);
            tracing::info!("[DAEMON] run_at: {}, symbols: {}", result.summary.run_at, result.summary.symbols.join(", "));
            let eligible: Vec<_> = result.summary.top_candidates.iter().filter(|c| !c.rejected).collect();
            tracing::info!("[DAEMON] candidates: {} total, {} eligible", result.summary.top_candidates.len(), eligible.len());

            if let Some(best) = eligible.iter().max_by(|a, b| {
                a.survivor_score.partial_cmp(&b.survivor_score).unwrap_or(std::cmp::Ordering::Equal)
            }) {
                tracing::info!(
                    "[DAEMON] best: {} — survivor={:.3}, sharpe={:.2}, cons={:.0}%, fragility={:.3}",
                    best.symbol,
                    best.survivor_score,
                    best.oos_sharpe,
                    best.oos_consistency * 100.0,
                    best.fragility
                );

                if let Some(threshold) = best.params.get("signal_threshold").and_then(|v| v.as_f64()) {
                    config.signal_threshold = threshold;
                }
                if let Some(tp) = best.params.get("take_profit_atr").and_then(|v| v.as_f64()) {
                    config.tp_atr = tp;
                }
                if let Some(sl) = best.params.get("stop_loss_atr").and_then(|v| v.as_f64()) {
                    config.sl_atr = sl;
                }
                if let Some(mh) = best.params.get("max_hold_hours").and_then(|v| v.as_f64()) {
                    config.max_hold_hours = mh;
                }
                if let Some(ts) = best.params.get("trailing_stop_atr").and_then(|v| v.as_f64()) {
                    config.trailing_stop_atr = ts;
                }
                tracing::info!(
                    "[DAEMON] updated config from night shift: signal_threshold={}, tp_atr={}, sl_atr={}, max_hold={}h, trailing_stop={}",
                    config.signal_threshold,
                    config.tp_atr,
                    config.sl_atr,
                    config.max_hold_hours,
                    config.trailing_stop_atr
                );
            } else {
                tracing::info!("[DAEMON] no eligible candidates — keeping current config");
            }
        }
        Err(e) => {
            tracing::info!("[DAEMON] no night shift results available ({}) — using current config", e);
        }
    }
    let params_used = config;

    // 1c. Stale position check.
    let stale_actions;
    {
        let max_hold = params_used.max_hold_hours;
        let stale_timeout_hours = max_hold * 1.1;
        tracing::info!(
            "[DAEMON] stale position timeout: {:.1}h (max_hold={}h × 1.1)",
            stale_timeout_hours, max_hold
        );
        stale_actions = check_stale_positions(max_hold, chain_cfg.as_ref()).await;

        // Submit close instructions for stale positions when chain config is available.
        if !stale_actions.is_empty() {
            if let Some(ref cfg) = chain_cfg {
                tracing::info!(
                    "[DAEMON] building close_flash_position for {} stale position(s)",
                    stale_actions.len()
                );
                let market = FlashMarketAccounts::sol_long_default();
                let auth_kp = cfg.load_authority();
                match auth_kp {
                    Ok(authority) => {
                        for stale in &stale_actions {
                            let side = match stale.side.as_str() {
                                "Long" | "long" => FlashSide::Long,
                                _ => FlashSide::Short,
                            };
                            let close_ix = build_close_flash_position_ix(
                                cfg,
                                &authority.pubkey(),
                                &cfg.vault_pda,
                                &market,
                                side,
                                OraclePrice { price: 0, exponent: -8 }, // placeholder — real oracle from Flash API
                                500, // 5% slippage buffer
                                0,   // delta — let program figure this out
                            );
                            tracing::info!(
                                "[DAEMON] close ix built for {} ({}), submitting via {}",
                                &stale.position_address[..8],
                                stale.side,
                                cfg.mode.label()
                            );
                            match submit_or_simulate(cfg, vec![close_ix], &authority) {
                                Ok(result) => {
                                    tracing::info!("[DAEMON] close result: {}", &result[..result.len().min(200)]);
                                }
                                Err(e) => {
                                    tracing::info!("[DAEMON] close failed for {}: {}", &stale.position_address[..8], e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::info!(
                            "[DAEMON] cannot close stale positions — authority keypair not loaded: {}",
                            e
                        );
                    }
                }
            } else {
                tracing::info!(
                    "[DAEMON] {} stale position(s) detected but no chain config — skipping close",
                    stale_actions.len()
                );
            }
        }
    }

    // 2. Execute: real chain interaction when config available, demo fallback.
    tracing::info!(" ");
    tracing::info!("=== EXECUTION ===");

    let mut _open_tx_result: Option<String> = None;

    if let Some(ref cfg) = chain_cfg {
        // Real chain path — build and submit/simulate open_flash_position.
        match cfg.load_authority() {
            Ok(authority) => {
                let market = FlashMarketAccounts::sol_long_default();
                tracing::info!(
                    "[DAEMON] building open_flash_position: side=Long, leverage=1x, mode={}",
                    cfg.mode.label()
                );
                let open_ix = build_open_flash_position_ix(
                    cfg,
                    &authority.pubkey(),
                    &cfg.vault_pda, // funding_account — treasury vault ATA for wSOL
                    &market,
                    FlashSide::Long,
                    10_000_000, // 0.01 SOL
                    10_000,     // 1x leverage (100%)
                    500,        // 5% slippage
                    OraclePrice {
                        price: 170_000_000_000, // placeholder — production should query Pyth/Flash
                        exponent: -8,
                    },
                    "Crypto.1",
                );

                match submit_or_simulate(cfg, vec![open_ix], &authority) {
                    Ok(result) => {
                        tracing::info!(
                            "[DAEMON] open_flash_position result: {}",
                            &result[..result.len().min(300)]
                        );
                        _open_tx_result = Some(result);
                    }
                    Err(e) => {
                        tracing::info!(
                            "[DAEMON] open_flash_position failed: {}. On-chain CPI gates will enforce.",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                tracing::info!(
                    "[DAEMON] authority keypair not loaded ({}). Skipping open execution.",
                    e
                );
            }
        }
    } else {
        // Demo fallback — no chain config available.
        tracing::info!("[DAEMON] no chain config — running demo orchestrator cycle");
    }

    // Always run the in-process orchestrator (trading state, audit, knowledge).
    let demo_result = rtp_swarm::demo::run_two_cycle_demo().await;
    let cycle_health = if demo_result.success {
        CycleHealth::Healthy
    } else {
        CycleHealth::Degraded
    };
    if demo_result.success {
        tracing::info!("[DAEMON] orchestrator cycle completed successfully");
    } else {
        tracing::info!("[DAEMON] orchestrator cycle completed with issues (non-fatal)");
    }

    // 3. Propose mutations.
    tracing::info!(" ");
    tracing::info!("=== STRATEGY MUTATION PROPOSAL ===");
    let llm_config = LlmProposerConfig::from_env();
    let propose_result = propose_strategy_mutation(llm_config).await;

    tracing::info!(
        "[DAEMON] proposer: {} (model: {})",
        if propose_result.used_llm {
            "LLM"
        } else {
            "deterministic fallback"
        },
        propose_result.model_label
    );

    for m in &propose_result.mutations {
        tracing::info!(
            "[DAEMON] proposed: {} → {} ({})",
            m.param, m.value, m.rationale
        );
    }

    // 4. Validate and apply mutations.
    let all_proposed = propose_result.mutations.clone();
    let accepted = validate_all_mutations(propose_result.mutations);
    let rejected: Vec<_> = all_proposed
        .iter()
        .filter(|m| !accepted.contains(m))
        .cloned()
        .collect();

    tracing::info!(" ");
    tracing::info!("=== APPLY MUTATIONS ===");
    tracing::info!(
        "[DAEMON] accepted: {}, rejected: {}",
        accepted.len(),
        rejected.len()
    );

    let mut next_config = params_used.clone();
    apply_mutations(&mut next_config, &accepted);

    // 5. Write cycle output.
    let now = Utc::now();
    let cycle_id = now.format("%Y-%m-%dT%H").to_string();
    let root = repo_root();
    let cycle_dir = root
        .join("data/devnet-cycles")
        .join(&cycle_id)
        .display()
        .to_string();

    std::fs::create_dir_all(&cycle_dir)
        .map_err(|e| format!("failed to create {}: {}", cycle_dir, e))?;

    let output = CycleOutput {
        cycle_id: now.to_rfc3339(),
        health: cycle_health,
        retry_count: 0,
        error_message: None,
        params_used,
        mutations_proposed: all_proposed,
        mutations_accepted: accepted,
        mutations_rejected: rejected,
        params_next: next_config,
        used_llm: propose_result.used_llm,
        model_label: propose_result.model_label,
        memory_files: collect_memory_files(),
    };

    let cycle_json = serde_json::to_string_pretty(&output)
        .map_err(|e| format!("failed to serialize cycle output: {}", e))?;
    let cycle_path = format!("{}/cycle.json", cycle_dir);
    std::fs::write(&cycle_path, &cycle_json)
        .map_err(|e| format!("failed to write {}: {}", cycle_path, e))?;
    tracing::info!("[DAEMON] wrote {}", cycle_path);

    // 6. Update latest directory.
    let latest = root.join("data/devnet-cycles/latest");
    if latest.exists() || latest.is_symlink() {
        let _ = std::fs::remove_dir_all(&latest);
    }
    if let Err(e) = std::fs::create_dir_all(&latest) {
        tracing::info!("[DAEMON] could not create latest dir: {}", e);
    } else {
        let _ = std::fs::copy(&cycle_path, latest.join("cycle.json"));
        let config_json =
            serde_json::to_string_pretty(&output.params_next).unwrap_or_default();
        let _ = std::fs::write(latest.join("config.json"), &config_json);
        tracing::info!("[DAEMON] updated data/devnet-cycles/latest/");
    }

    // 7. Summary.
    tracing::info!(" ");
    tracing::info!("=== CYCLE COMPLETE ===");
    tracing::info!("[DAEMON] health: {:?}", output.health);
    tracing::info!("[DAEMON] cycle_id: {}", output.cycle_id);
    tracing::info!("[DAEMON] used_llm: {}", output.used_llm);
    tracing::info!("[DAEMON] output: {}", cycle_path);
    tracing::info!("[DAEMON] exit 0");

    Ok(())
}
