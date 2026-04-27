//! RTP Swarm — Devnet Loop Daemon (single-cycle, designed for 6h cron).
//! Loads config → runs one cycle → proposes mutations → writes output to data/devnet-cycles/.

use chrono::Utc;
use rtp_swarm::wings::evolve::{
    LlmProposerConfig, propose_strategy_mutation, validate_all_mutations,
};
use rtp_swarm::wings::trading::{StrategyConfig, apply_mutations};
use serde::{Deserialize, Serialize};

/// Cycle output written to data/devnet-cycles/{timestamp}/cycle.json.
#[derive(Debug, Serialize, Deserialize)]
struct CycleOutput {
    cycle_id: String,
    params_used: StrategyConfig,
    mutations_proposed: Vec<rtp_swarm::wings::evolve::StrategyMutation>,
    mutations_accepted: Vec<rtp_swarm::wings::evolve::StrategyMutation>,
    mutations_rejected: Vec<rtp_swarm::wings::evolve::StrategyMutation>,
    params_next: StrategyConfig,
    used_llm: bool,
    model_label: String,
    memory_files: Vec<String>,
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
                    println!("[DAEMON] loaded config from {}", path_display);
                    return config;
                }
                Err(e) => println!("[DAEMON] config parse error: {} — using defaults", e),
            },
            Err(e) => println!("[DAEMON] config read error: {} — using defaults", e),
        }
    } else {
        println!("[DAEMON] no prior config found — using SOL/USDT Survivor 2.69 defaults");
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

/// Check if the demo treasury is frozen by reading the on-chain account data.
/// Returns Ok(true) if frozen, Ok(false) if not, Err if RPC unreachable.
fn check_treasury_frozen() -> Result<bool, String> {
    let treasury_pda = "FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF";
    let rpc_url = "https://api.devnet.solana.com";

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

    let json: serde_json::Value = resp
        .json()
        .map_err(|e| format!("RPC parse error: {}", e))?;

    let data = json
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("data"))
        .and_then(|d| d.get(0))
        .and_then(|d| d.as_str())
        .ok_or("No account data returned")?;

    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    // Frozen field at byte offset 225 (8 discriminator + 32+32+1+8*6+32*3+8 = 225)
    let frozen_offset = 225;
    if bytes.len() > frozen_offset {
        Ok(bytes[frozen_offset] != 0)
    } else {
        Err("Account data too short to read frozen flag".to_string())
    }
}

#[tokio::main]
async fn main() {
    println!("┌─────────────────────────────────────────────────┐");
    println!("│  RTP — Devnet Loop Daemon                       │");
    println!(
        "│  Autonomous cycle: {}        │",
        Utc::now().format("%Y-%m-%d %H:%M UTC")
    );
    println!("└─────────────────────────────────────────────────┘");
    println!();

    // 0. Check if treasury is frozen before running cycle.
    // The on-chain program will reject operations, but checking early
    // avoids wasting a full cycle of work.
    match check_treasury_frozen() {
        Ok(true) => {
            println!("[DAEMON] Treasury is FROZEN — skipping cycle. Unfreeze required by authority.");
            println!("[DAEMON] exit 0 (frozen is not an error)");
            return;
        }
        Ok(false) => {
            println!("[DAEMON] Treasury status: ACTIVE");
        }
        Err(e) => {
            println!("[DAEMON] Could not check frozen status ({}). Continuing — on-chain CPI will gate.", e);
        }
    }

    // 1. Load config.
    let config = load_config();
    let params_used = config.clone();
    println!(
        "[DAEMON] params: signal_threshold={}, tp_atr={}, sl_atr={}, max_hold={}h, trailing_stop={}",
        config.signal_threshold,
        config.tp_atr,
        config.sl_atr,
        config.max_hold_hours,
        config.trailing_stop_atr
    );

    // 2. Run orchestrator cycle (reuses demo infrastructure).
    println!();
    println!("=== ORCHESTRATOR CYCLE ===");
    let demo_result = rtp_swarm::demo::run_two_cycle_demo().await;
    if demo_result.success {
        println!("[DAEMON] orchestrator cycle completed successfully");
    } else {
        println!("[DAEMON] orchestrator cycle completed with issues (non-fatal)");
    }

    // 3. Propose mutations.
    println!();
    println!("=== STRATEGY MUTATION PROPOSAL ===");
    let llm_config = LlmProposerConfig::from_env();
    let propose_result = propose_strategy_mutation(llm_config).await;

    println!(
        "[DAEMON] proposer: {} (model: {})",
        if propose_result.used_llm {
            "LLM"
        } else {
            "deterministic fallback"
        },
        propose_result.model_label
    );

    for m in &propose_result.mutations {
        println!(
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

    println!();
    println!("=== APPLY MUTATIONS ===");
    println!(
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

    if let Err(e) = std::fs::create_dir_all(&cycle_dir) {
        eprintln!("[DAEMON] failed to create {}: {}", cycle_dir, e);
        std::process::exit(1);
    }

    let output = CycleOutput {
        cycle_id: now.to_rfc3339(),
        params_used,
        mutations_proposed: all_proposed,
        mutations_accepted: accepted,
        mutations_rejected: rejected,
        params_next: next_config,
        used_llm: propose_result.used_llm,
        model_label: propose_result.model_label,
        memory_files: collect_memory_files(),
    };

    let cycle_json = serde_json::to_string_pretty(&output).expect("serialize cycle output");
    let cycle_path = format!("{}/cycle.json", cycle_dir);
    std::fs::write(&cycle_path, &cycle_json).expect("write cycle.json");
    println!("[DAEMON] wrote {}", cycle_path);

    // 6. Update latest directory (copy files — more portable than symlink for CI).
    let latest = root.join("data/devnet-cycles/latest");
    if latest.exists() || latest.is_symlink() {
        let _ = std::fs::remove_dir_all(&latest);
    }
    if let Err(e) = std::fs::create_dir_all(&latest) {
        println!("[DAEMON] ⚠️ could not create latest dir: {}", e);
    } else {
        let _ = std::fs::copy(&cycle_path, latest.join("cycle.json"));
        // Also copy config for next cycle.
        let config_json =
            serde_json::to_string_pretty(&output.params_next).expect("serialize config");
        let _ = std::fs::write(latest.join("config.json"), &config_json);
        println!("[DAEMON] updated data/devnet-cycles/latest/");
    }

    // 7. Summary.
    println!();
    println!("=== CYCLE COMPLETE ===");
    println!("[DAEMON] cycle_id: {}", output.cycle_id);
    println!("[DAEMON] used_llm: {}", output.used_llm);
    println!("[DAEMON] output: {}", cycle_path);
    println!("[DAEMON] exit 0");
}
