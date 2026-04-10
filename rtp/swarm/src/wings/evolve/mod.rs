//! Evolve Wing — the only wing that can modify how other wings work.
//!
//! Every change is a diff that goes through the Coordinator and must pass
//! the Audit Wing. The Evolve Wing follows the Darwinian loop:
//!   Propose -> Assess baseline -> Apply -> Monitor -> Keep or Rollback
//!
//! Reference: https://github.com/chrisworsey55/atlas-gic (Darwinian loop)

pub mod assessor;
pub mod proposer;
pub mod rollback;

use crate::types::WingId;
use assessor::{Assessor, PerformanceMetrics};
use proposer::Proposer;
use rollback::RollbackManager;
use std::sync::Arc;

/// The Evolve Wing — self-modification and adaptation.
///
/// Combines:
/// - **Assessor**: benchmark wing performance, detect regressions
/// - **Proposer**: architecture change proposals (SPARC methodology)
/// - **Rollback**: revert changes that degrade performance >5%
pub struct EvolveWing {
    pub assessor: Arc<Assessor>,
    pub proposer: Arc<Proposer>,
    pub rollback: Arc<RollbackManager>,
}

impl EvolveWing {
    pub fn new() -> Self {
        let proposer = Arc::new(Proposer::new());
        let assessor = Arc::new(Assessor::new());
        let rollback = Arc::new(RollbackManager::with_defaults(proposer.clone()));

        Self {
            assessor,
            proposer,
            rollback,
        }
    }

    /// Record performance metrics for a wing and assess it.
    pub fn record_and_assess(&self, metrics: PerformanceMetrics) -> assessor::Assessment {
        let wing = metrics.wing;
        self.assessor.record_metrics(metrics);
        self.assessor.assess(wing)
    }

    /// Propose a change, assess baseline, and submit for audit.
    pub fn propose_change(
        &self,
        target_wing: WingId,
        description: String,
        diff: String,
        rationale: String,
        expected_impact: String,
    ) -> proposer::ChangeProposal {
        // Assess baseline before proposing.
        let baseline = self.assessor.assess(target_wing);
        let proposal_id = uuid::Uuid::new_v4();
        self.proposer.propose_with_id(
            proposal_id,
            target_wing,
            description,
            diff,
            rationale,
            expected_impact,
        );

        // Record baseline score on the proposal.
        let _ = self
            .proposer
            .set_baseline_score(&proposal_id, baseline.score);

        // Return the proposal with baseline set.
        self.proposer.get(&proposal_id).unwrap()
    }

    /// Check if any applied changes need rollback.
    pub fn check_rollbacks(&self) -> Vec<rollback::RollbackOperation> {
        let changes = self.rollback.tracked_changes();
        let mut triggered = Vec::new();

        for change in changes {
            if change.rolled_back {
                continue;
            }
            let current = self.assessor.assess(change.target_wing);
            if let Some(op) = self.rollback.evaluate(change.id, current.score) {
                triggered.push(op);
            }
        }

        triggered
    }

    /// Execute a rollback for a specific change.
    pub fn execute_rollback(
        &self,
        change_id: uuid::Uuid,
    ) -> Result<rollback::RollbackOperation, String> {
        self.rollback.execute_rollback(&change_id)
    }
}

impl Default for EvolveWing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WingId;

    fn trading_metrics(usdc_yield: f64, consistency: f64) -> PerformanceMetrics {
        PerformanceMetrics {
            wing: WingId::Trading,
            usdc_yield,
            sol_reserves: 50000.0,
            max_drawdown: 0.05,
            consistency,
            captured_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn evolve_wing_full_lifecycle() {
        let evolve = EvolveWing::new();

        // Step 1: Record baseline metrics.
        evolve.record_and_assess(trading_metrics(5000.0, 0.85));

        // Step 2: Propose a change.
        let proposal = evolve.propose_change(
            WingId::Trading,
            "Optimize entry logic".to_string(),
            "diff --git a/entry.rs".to_string(),
            "Better signals needed".to_string(),
            "+5% Sharpe".to_string(),
        );

        assert_eq!(proposal.status, proposer::ProposalStatus::Proposed);
        assert!(proposal.baseline_score.is_some());

        // Step 3: Track the change.
        let baseline = proposal.baseline_score.unwrap();
        evolve.rollback.track_change(crate::types::TrackedChange {
            id: proposal.id,
            target_wing: WingId::Trading,
            diff: "entry logic change".to_string(),
            baseline_score: baseline,
            applied_at: chrono::Utc::now(),
            rolled_back: false,
            rollback_reason: None,
        });

        // Step 4: Simulate improvement — no rollback.
        evolve.record_and_assess(trading_metrics(6000.0, 0.90));
        let rollbacks = evolve.check_rollbacks();
        assert!(rollbacks.is_empty());

        // Step 5: Simulate degradation — rollback triggered.
        evolve.record_and_assess(trading_metrics(500.0, 0.20));
        let rollbacks = evolve.check_rollbacks();
        assert_eq!(rollbacks.len(), 1);
        assert_eq!(rollbacks[0].status, rollback::RollbackStatus::Queued);

        // Step 6: Execute rollback.
        let result = evolve.execute_rollback(proposal.id).unwrap();
        assert_eq!(result.status, rollback::RollbackStatus::Completed);
    }

    #[test]
    fn propose_change_records_baseline() {
        let evolve = EvolveWing::new();
        evolve.record_and_assess(trading_metrics(5000.0, 0.85));

        let proposal = evolve.propose_change(
            WingId::Trading,
            "Test change".to_string(),
            "diff".to_string(),
            "reason".to_string(),
            "impact".to_string(),
        );

        assert!(proposal.baseline_score.unwrap() > 0.0);
    }

    #[test]
    fn evolve_wing_id() {
        let proposal = proposer::ChangeProposal::new(
            WingId::Security,
            "test".to_string(),
            "diff".to_string(),
            "r".to_string(),
            "i".to_string(),
        );
        let msg = proposal.to_message();
        assert_eq!(msg.from, WingId::Evolve);
    }
}
