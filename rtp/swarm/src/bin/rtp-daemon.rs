//! RTP Swarm — Devnet Loop Daemon
//!
//! Run with: cargo run --bin rtp-daemon
//!
//! Single-cycle autonomous daemon. Designed to be called on a schedule
//! (GitHub Actions cron every 6h). Each invocation:
//!   1. Loads last cycle config (or defaults)
//!   2. Runs one orchestrator cycle
//!   3. Proposes strategy mutations (LLM or fallback)
//!   4. Applies accepted mutations to config
//!   5. Writes cycle output to data/devnet-cycles/{timestamp}/
//!   6. Exits 0

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

/// Collect memory file paths from /tmp/rtp-demo-memory.
fn collect_memory_files() -> Vec<String> {
    let base = std::path::Path::new("/tmp/rtp-demo-memory");
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
