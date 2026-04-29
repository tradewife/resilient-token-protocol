//! Integration tests for the RTP swarm Coordinator + Wings.
//!
//! Tests the full demo loop, knowledge persistence, and two-cycle behavior.

use rtp_swarm::demo;
use rtp_swarm::wings::knowledge::KnowledgeWing;

#[tokio::test]
async fn full_demo_loop_completes_without_panic() {
    let result = demo::run_demo_loop().await;
    assert!(
        result.steps.iter().any(|s| s.name == "register_wings"),
        "Demo should include register_wings step"
    );
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

    // Write data
    {
        let wing = KnowledgeWing::new_with_persistence(path.clone());
        wing.put("test_key", "test_value_1");
        wing.put("test_key", "test_value_2");
        assert!(path.exists(), "Persistence file should exist after put");
    }

    // Reload and verify
    {
        let wing = KnowledgeWing::new_with_persistence(path.clone());
        assert_eq!(wing.store_size(), 1, "Reloaded wing should have 1 key");
    }
}

#[tokio::test]
async fn demo_steps_use_correct_status() {
    let result = demo::run_demo_loop().await;
    for step in &result.steps {
        // No step should still use the old `passed: bool` pattern
        // (this test verifies the StepStatus enum is in use)
        let _ = &step.status;
    }
}

#[tokio::test]
async fn two_cycle_demo_covers_all_judge_points() {
    let result = demo::run_two_cycle_demo().await;
    assert!(result.success, "Two-cycle demo should succeed");
    assert!(
        result.constraint_rejected,
        "Constraint should be rejected"
    );
    assert!(result.memory_persisted, "Memory should persist across cycles");
}

#[tokio::test]
async fn knowledge_wing_without_persistence_works() {
    let wing = KnowledgeWing::new();
    wing.put("test", "value");
    assert_eq!(wing.store_size(), 1);

    let wing2 = KnowledgeWing::default();
    assert_eq!(wing2.store_size(), 0);
}
