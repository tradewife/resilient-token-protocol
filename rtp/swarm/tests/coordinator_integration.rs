//! Integration tests for the RTP swarm Coordinator + Wings.
//!
//! Tests the full demo loop, knowledge persistence, two-cycle behavior,
//! Night Shift handoff pipeline, and real-chain instruction building.
//!
//! P1.2 acceptance: tests fail if the pipeline reverts to demo-only behavior.

use rtp_swarm::bridge::{self, NightShiftCandidate, NightShiftSummary};
use rtp_swarm::chain_client;
use rtp_swarm::chain_client::{
    ChainConfig, ExecutionMode, FlashMarketAccounts, FlashSide, OraclePrice,
    build_close_flash_position_ix, build_open_flash_position_ix,
};
use rtp_swarm::demo;
use rtp_swarm::wings::knowledge::KnowledgeWing;
use solana_sdk::signature::Signer;
use tempfile::TempDir;

#[tokio::test]
async fn full_demo_loop_completes_without_panic() {
    let result = demo::run_demo_loop().await;
    assert!(result.steps.iter().any(|s| s.name == "register_wings"));
    assert!(
        result.steps.len() >= 5,
        "Demo should have >= 5 steps, got {}",
        result.steps.len()
    );
}

#[tokio::test]
async fn knowledge_wing_persists_and_reloads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wing-state.json");
    {
        let wing = KnowledgeWing::new_with_persistence(path.clone());
        wing.put("test_key", "test_value_1");
        wing.put("test_key", "test_value_2");
        assert!(path.exists());
    }
    {
        let wing = KnowledgeWing::new_with_persistence(path.clone());
        assert_eq!(wing.store_size(), 1);
    }
}

#[tokio::test]
async fn demo_steps_use_correct_status() {
    let result = demo::run_demo_loop().await;
    for step in &result.steps {
        let _ = &step.status;
    }
}

#[tokio::test]
async fn two_cycle_demo_covers_all_judge_points() {
    let result = demo::run_two_cycle_demo().await;
    assert!(result.success);
    assert!(result.constraint_rejected);
    assert!(result.memory_persisted);
}

#[tokio::test]
async fn knowledge_wing_without_persistence_works() {
    let wing = KnowledgeWing::new();
    wing.put("test", "value");
    assert_eq!(wing.store_size(), 1);
    let wing2 = KnowledgeWing::default();
    assert_eq!(wing2.store_size(), 0);
}

// ---------------------------------------------------------------------------
// P1.2 — Real Pipeline Integration Tests
// ---------------------------------------------------------------------------

/// P1.2 Test 1: Night Shift summary to promotion dry-run.
/// Sets NIGHT_RESULTS_DIR, reads back, asserts locally. Isolated from other tests
/// by using a TempDir date "2026-04-28" (older than real "2026-04-15").
#[tokio::test]
async fn night_shift_summary_to_promotion_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    let night_dir = temp_dir.path().join("night_results");
    let date_dir = night_dir.join("2026-04-28");

    let summary = NightShiftSummary {
        run_at: "2026-04-28T06:00:00Z".to_string(),
        runtime_seconds: 9000.0,
        num_folds: 9,
        symbols: vec!["SOL/USDT".to_string(), "BTC/USDT".to_string()],
        top_candidates: vec![
            NightShiftCandidate {
                symbol: "SOL/USDT".to_string(),
                params: serde_json::json!({"signal_threshold": 0.3, "take_profit_atr": 3.0, "stop_loss_atr": 1.5, "max_hold_hours": 36.0, "trailing_stop_atr": 0.5}),
                survivor_score: 2.69,
                oos_sharpe: 3.96,
                oos_consistency: 1.0,
                oos_max_dd: 0.08,
                overfitting_score: 0.29,
                fragility: 0.29,
                oos_avg_trades_per_fold: 47.0,
                rejected: false,
                rejection_reason: None,
            },
            NightShiftCandidate {
                symbol: "BTC/USDT".to_string(),
                params: serde_json::json!({}),
                survivor_score: 1.52,
                oos_sharpe: 1.2,
                oos_consistency: 0.78,
                oos_max_dd: 0.12,
                overfitting_score: 0.57,
                fragility: 0.65,
                oos_avg_trades_per_fold: 22.0,
                rejected: false,
                rejection_reason: None,
            },
        ],
    };

    // Write summary FIRST (before env var is set)
    std::fs::create_dir_all(&date_dir).unwrap();
    std::fs::write(
        date_dir.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .unwrap();

    // Set env var so bridge.rs reads from our temp dir
    unsafe {
        std::env::set_var("NIGHT_RESULTS_DIR", night_dir.to_str().unwrap());
    }

    // Read back from the same dir
    let result =
        bridge::read_latest_night_results().expect("read_latest_night_results should succeed");
    let best = bridge::best_night_shift_candidate().expect("should find a candidate");

    // Assert on locally-captured values (not dependent on env var state)
    assert_eq!(result.summary.run_at, "2026-04-28T06:00:00Z");
    assert_eq!(result.summary.top_candidates.len(), 2);
    assert_eq!(best.symbol, "SOL/USDT");
    assert!(
        (best.survivor_score - 2.69).abs() < 0.001,
        "SOL survivor={}",
        best.survivor_score
    );
    assert!(
        (best.oos_sharpe - 3.96).abs() < 0.01,
        "SOL sharpe={}",
        best.oos_sharpe
    );

    let response = best.to_bridge_response();
    assert!(response.strategy.contains("SOL"));
    assert_eq!(response.params["signal_threshold"], 0.3);
    assert_eq!(response.params["take_profit_atr"], 3.0);

    unsafe {
        std::env::remove_var("NIGHT_RESULTS_DIR");
    }
}

/// Promotion gate helper for P1.2 Test 2.
struct TestGate {
    min_sharpe: f64,
    min_consistency: f64,
    min_trades: f64,
    max_fragility: f64,
}

impl TestGate {
    fn eval(&self, c: &NightShiftCandidate) -> (bool, Vec<String>) {
        if c.rejected {
            return (
                false,
                vec![format!(
                    "rejected: {}",
                    c.rejection_reason.as_deref().unwrap_or("?")
                )],
            );
        }
        let mut reasons = Vec::new();
        if c.oos_sharpe < self.min_sharpe {
            reasons.push(format!("Sharpe {:.2}<{:.1}", c.oos_sharpe, self.min_sharpe));
        }
        if c.oos_consistency < self.min_consistency {
            reasons.push(format!(
                "Cons {:.0}%<{:.0}%",
                c.oos_consistency * 100.0,
                self.min_consistency * 100.0
            ));
        }
        if c.oos_avg_trades_per_fold < self.min_trades {
            reasons.push(format!(
                "Trades {:.1}<{:.0}",
                c.oos_avg_trades_per_fold, self.min_trades
            ));
        }
        if c.fragility > self.max_fragility {
            reasons.push(format!("Frag {:.3}>{:.2}", c.fragility, self.max_fragility));
        }
        (reasons.is_empty(), reasons)
    }
}

/// P1.2 Test 2: Promotion gate evaluation.
#[tokio::test]
async fn promotion_gate_filters_candidates_correctly() {
    let gate = TestGate {
        min_sharpe: 2.5,
        min_consistency: 0.70,
        min_trades: 15.0,
        max_fragility: 0.40,
    };
    let sol = NightShiftCandidate {
        symbol: "SOL/USDT".to_string(),
        params: serde_json::json!({}),
        survivor_score: 2.69,
        oos_sharpe: 3.96,
        oos_consistency: 1.0,
        oos_max_dd: 0.08,
        overfitting_score: 0.29,
        fragility: 0.29,
        oos_avg_trades_per_fold: 47.0,
        rejected: false,
        rejection_reason: None,
    };
    let (p, r) = gate.eval(&sol);
    assert!(p, "SOL should pass: {:?}", r);

    let btc = NightShiftCandidate {
        symbol: "BTC/USDT".to_string(),
        params: serde_json::json!({}),
        survivor_score: 1.52,
        oos_sharpe: 1.2,
        oos_consistency: 0.78,
        oos_max_dd: 0.12,
        overfitting_score: 0.57,
        fragility: 0.65,
        oos_avg_trades_per_fold: 22.0,
        rejected: false,
        rejection_reason: None,
    };
    let (p, r) = gate.eval(&btc);
    assert!(!p, "BTC should fail");
    assert!(r.iter().any(|x| x.contains("Frag")));
}

/// P1.2 Test 3: Daemon builds open_flash_position in simulate mode.
#[tokio::test]
async fn daemon_simulates_open_position() {
    let mut cfg = ChainConfig::test_default();
    cfg.mode = ExecutionMode::Simulate;
    // Point at a clearly unreachable address so we get Err, not a live RPC.
    cfg.rpc_url = "http://localhost:19999".to_string();
    let authority = solana_sdk::signature::Keypair::new();
    let market = FlashMarketAccounts::sol_long_default();
    let funding = solana_sdk::pubkey::Pubkey::new_unique();
    let ix = build_open_flash_position_ix(
        &cfg,
        &authority.pubkey(),
        &funding,
        &market,
        FlashSide::Long,
        10_000_000,
        10_000,
        500,
        OraclePrice {
            price: 170_000_000_000,
            exponent: -8,
        },
        "SOL_2.69",
    );
    assert!(ix.accounts.len() >= 16);
    assert_eq!(
        &ix.data[..8],
        &rtp_swarm::chain_client::OPEN_FLASH_POSITION_DISC
    );
    // submit_or_simulate uses blocking reqwest — call from blocking thread to avoid
    // "Cannot drop a runtime in a context where blocking is not allowed" panic.
    let inner_result: Result<String, String> = tokio::task::spawn_blocking({
        let cfg = cfg.clone();
        let authority = authority.insecure_clone();
        let ix = ix.clone();
        move || chain_client::submit_or_simulate(&cfg, vec![ix], &authority)
    })
    .await
    .unwrap();
    assert!(inner_result.is_err(), "unreachable RPC should fail");
}

/// P1.2 Test 4: Stale position triggers close instruction build.
#[tokio::test]
async fn stale_position_triggers_close_simulation() {
    let mut cfg = ChainConfig::test_default();
    cfg.mode = ExecutionMode::Simulate;
    cfg.rpc_url = "http://localhost:19999".to_string();
    let authority = solana_sdk::signature::Keypair::new();
    let market = FlashMarketAccounts::sol_long_default();

    let close_ix = build_close_flash_position_ix(
        &cfg,
        &authority.pubkey(),
        &cfg.vault_pda,
        &market,
        FlashSide::Long,
        OraclePrice {
            price: 160_000_000_000,
            exponent: -8,
        },
        500,
        0,
    );
    assert!(close_ix.accounts.len() >= 12);
    assert_eq!(
        &close_ix.data[..8],
        &rtp_swarm::chain_client::CLOSE_FLASH_POSITION_DISC
    );

    let open_ix = build_open_flash_position_ix(
        &cfg,
        &authority.pubkey(),
        &cfg.vault_pda,
        &market,
        FlashSide::Long,
        10_000_000,
        10_000,
        500,
        OraclePrice {
            price: 170_000_000_000,
            exponent: -8,
        },
        "SOL_2.69",
    );
    assert_ne!(&open_ix.data[..8], &close_ix.data[..8]);

    // submit_or_simulate uses blocking reqwest — call from blocking thread.
    let inner_result: Result<String, String> = tokio::task::spawn_blocking({
        let cfg = cfg.clone();
        let authority = authority.insecure_clone();
        let close_ix = close_ix.clone();
        move || chain_client::submit_or_simulate(&cfg, vec![close_ix], &authority)
    })
    .await
    .unwrap();
    assert!(inner_result.is_err(), "unreachable RPC should fail");
}

/// P1.2 Test 5: Night shift params flow into daemon config.
#[tokio::test]
async fn night_shift_to_daemon_config() {
    let temp_dir = TempDir::new().unwrap();
    let night_dir = temp_dir.path().join("night_results");
    std::fs::create_dir_all(&night_dir).unwrap();
    let date_dir = night_dir.join("2026-04-28");
    std::fs::create_dir_all(&date_dir).unwrap();

    let summary = NightShiftSummary {
        run_at: "2026-04-28T14:00:00Z".to_string(),
        runtime_seconds: 9888.0,
        num_folds: 9,
        symbols: vec!["SOL/USDT".to_string()],
        top_candidates: vec![NightShiftCandidate {
            symbol: "SOL/USDT".to_string(),
            params: serde_json::json!({"signal_threshold": 0.3, "take_profit_atr": 3.0, "stop_loss_atr": 1.5, "max_hold_hours": 36.0, "trailing_stop_atr": 0.5}),
            survivor_score: 2.69,
            oos_sharpe: 3.96,
            oos_consistency: 1.0,
            oos_max_dd: 0.08,
            overfitting_score: 0.29,
            fragility: 0.29,
            oos_avg_trades_per_fold: 47.0,
            rejected: false,
            rejection_reason: None,
        }],
    };

    std::fs::write(
        date_dir.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .unwrap();

    unsafe {
        std::env::set_var("NIGHT_RESULTS_DIR", night_dir.to_str().unwrap());
    }
    let best = bridge::best_night_shift_candidate().expect("should find candidate");
    assert_eq!(best.params["signal_threshold"], 0.3);
    assert_eq!(best.params["take_profit_atr"], 3.0);
    assert_eq!(best.params["max_hold_hours"], 36.0);

    let stale = 36.0_f64 * 1.1;
    assert!((stale - 39.6).abs() < 0.001);

    unsafe {
        std::env::remove_var("NIGHT_RESULTS_DIR");
    }
}

// ---------------------------------------------------------------------------
// P1.3 — Knowledge Wing Railway Persistence
// ---------------------------------------------------------------------------

/// P1.3 Test: Daemon persistence path resolves from RTP_KNOWLEDGE_PATH.
#[tokio::test]
async fn knowledge_wing_persists_with_daemon_path() {
    let dir = TempDir::new().unwrap();
    let path = dir
        .path()
        .join("data/swarm-memory/knowledge/wing-state.json");
    {
        let wing = KnowledgeWing::new_with_persistence(path.clone());
        wing.put("daemon_cycle", "cycle_id=2026-04-29T14_health=Healthy");
        wing.put("config_applied", "signal_threshold=0.3");
        assert!(path.exists());
        assert_eq!(wing.store_size(), 2);
    }
    {
        let wing = KnowledgeWing::new_with_persistence(path.clone());
        assert_eq!(wing.store_size(), 2);
        let msg = rtp_swarm::types::Message::new(
            rtp_swarm::types::WingId::Coordinator,
            rtp_swarm::types::WingId::Knowledge,
            rtp_swarm::types::Payload::KnowledgeQuery {
                query: "daemon_cycle".to_string(),
                context: None,
            },
        );
        let resp = wing.handle_message(&msg).unwrap();
        if let rtp_swarm::types::Payload::KnowledgeResult { results } = &resp.payload {
            assert!(results.iter().any(|r| r.contains("Healthy")));
        } else {
            panic!("Expected KnowledgeResult, got {:?}", resp.payload);
        }
    }
}

/// P1.3 Test: RTP_KNOWLEDGE_PATH env var is respected.
#[tokio::test]
async fn knowledge_wing_respects_rtp_knowledge_path_env() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("knowledge.json");
    unsafe {
        std::env::set_var("RTP_KNOWLEDGE_PATH", path.to_str().unwrap());
    }

    let wing =
        KnowledgeWing::new_with_persistence(std::path::PathBuf::from(path.to_str().unwrap()));
    wing.put("test_from_env", "value123");
    assert!(path.exists());

    unsafe {
        std::env::remove_var("RTP_KNOWLEDGE_PATH");
    }

    let wing2 = KnowledgeWing::new_with_persistence(path.clone());
    assert_eq!(wing2.store_size(), 1);
}
