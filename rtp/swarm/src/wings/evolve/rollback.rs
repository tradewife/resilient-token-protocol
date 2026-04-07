//! Rollback — revert changes that degrade performance beyond threshold.
//!
//! Reference: https://github.com/chrisworsey55/atlas-gic (Darwinian loop)
//!
//! If a change degrades performance > 5%, revert within minutes.
//! The rollback mechanism:
//!   1. Monitor post-change performance via Assessor
//!   2. Compare against baseline (pre-change score)
//!   3. If degradation > threshold, queue rollback
//!   4. Execute rollback and notify all wings

use crate::types::{Message, Payload, TrackedChange, WingId};
use crate::wings::evolve::proposer::Proposer;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Status of a rollback operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollbackStatus {
    /// Monitoring post-change performance.
    Monitoring,
    /// Degradation detected, rollback queued.
    Queued,
    /// Rollback executing.
    Executing,
    /// Rollback completed successfully.
    Completed,
    /// Rollback failed (manual intervention needed).
    Failed,
}

/// A tracked rollback operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackOperation {
    /// The change being rolled back.
    pub change_id: uuid::Uuid,
    /// The wing that was changed.
    pub target_wing: WingId,
    /// The original diff that was applied.
    pub original_diff: String,
    /// The reverse diff to revert.
    pub reverse_diff: String,
    /// Baseline score before the change.
    pub baseline_score: f64,
    /// Current post-change score.
    pub current_score: f64,
    /// Measured degradation percentage.
    pub degradation: f64,
    /// Current rollback status.
    pub status: RollbackStatus,
    /// Reason for the rollback.
    pub reason: String,
    /// When rollback was initiated.
    pub initiated_at: DateTime<Utc>,
    /// When rollback completed (if applicable).
    pub completed_at: Option<DateTime<Utc>>,
}

/// Rollback configuration.
#[derive(Debug, Clone)]
pub struct RollbackConfig {
    /// Performance degradation threshold that triggers rollback (default 5%).
    pub degradation_threshold: f64,
    /// How long to monitor after a change before allowing rollback.
    pub monitoring_window_secs: u64,
    /// Maximum time to attempt rollback before marking as failed.
    pub rollback_timeout_secs: u64,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            degradation_threshold: 0.05,
            monitoring_window_secs: 300, // 5 minutes
            rollback_timeout_secs: 600,  // 10 minutes
        }
    }
}

/// The Rollback manager — monitors changes and reverts degrading ones.
///
/// Implements the Darwinian loop from ATLAS: changes that weaken the
/// swarm are pruned automatically, keeping only improvements.
pub struct RollbackManager {
    /// Configuration.
    config: RollbackConfig,
    /// Tracked changes that can be rolled back.
    tracked_changes: Arc<DashMap<uuid::Uuid, TrackedChange>>,
    /// Active rollback operations.
    rollbacks: Arc<DashMap<uuid::Uuid, RollbackOperation>>,
    /// Reference to the proposer (to update proposal status on rollback).
    proposer: Arc<Proposer>,
}

impl RollbackManager {
    pub fn new(config: RollbackConfig, proposer: Arc<Proposer>) -> Self {
        Self {
            config,
            tracked_changes: Arc::new(DashMap::new()),
            rollbacks: Arc::new(DashMap::new()),
            proposer,
        }
    }

    pub fn with_defaults(proposer: Arc<Proposer>) -> Self {
        Self::new(RollbackConfig::default(), proposer)
    }

    /// Track a change that has been applied and may need rollback.
    pub fn track_change(&self, change: TrackedChange) {
        self.tracked_changes.insert(change.id, change);
    }

    /// Check a tracked change for performance degradation.
    /// If degradation exceeds threshold, queue a rollback.
    pub fn evaluate(
        &self,
        change_id: uuid::Uuid,
        current_score: f64,
    ) -> Option<RollbackOperation> {
        let change = self.tracked_changes.get(&change_id)?;

        if change.rolled_back {
            return None;
        }

        let baseline = change.baseline_score;
        if baseline <= 0.0 {
            return None;
        }

        let degradation = (baseline - current_score) / baseline;

        if degradation > self.config.degradation_threshold {
            let rollback = RollbackOperation {
                change_id,
                target_wing: change.target_wing,
                original_diff: change.diff.clone(),
                reverse_diff: format!("REVERT: {}", change.diff),
                baseline_score: baseline,
                current_score,
                degradation,
                status: RollbackStatus::Queued,
                reason: format!(
                    "Performance degraded {:.1}% (threshold: {:.1}%)",
                    degradation * 100.0,
                    self.config.degradation_threshold * 100.0
                ),
                initiated_at: Utc::now(),
                completed_at: None,
            };

            self.rollbacks.insert(change_id, rollback.clone());
            Some(rollback)
        } else {
            None
        }
    }

    /// Execute a queued rollback.
    pub fn execute_rollback(&self, change_id: &uuid::Uuid) -> Result<RollbackOperation, String> {
        let mut rollback = self
            .rollbacks
            .get_mut(change_id)
            .ok_or("No rollback operation found")?;

        if rollback.status != RollbackStatus::Queued {
            return Err(format!(
                "Rollback is in {:?} state, expected Queued",
                rollback.status
            ));
        }

        rollback.status = RollbackStatus::Executing;

        // Mark the tracked change as rolled back.
        if let Some(mut change) = self.tracked_changes.get_mut(change_id) {
            change.rolled_back = true;
            change.rollback_reason = Some(rollback.reason.clone());
        }

        // Update the proposal status if it exists.
        // We look for proposals by matching the change_id.
        let proposals = self.proposer.all_proposals();
        for p in proposals {
            if p.id == *change_id {
                let _ = self.proposer.mark_executed(&p.id);
                // Transition to RolledBack if executed.
                break;
            }
        }

        rollback.status = RollbackStatus::Completed;
        rollback.completed_at = Some(Utc::now());

        Ok(rollback.clone())
    }

    /// Create a rollback request message for routing through Coordinator.
    pub fn create_rollback_message(change_id: uuid::Uuid, reason: String) -> Message {
        Message::new(
            WingId::Evolve,
            WingId::Coordinator,
            Payload::RollbackRequest { change_id, reason },
        )
    }

    /// Get all active (non-completed) rollbacks.
    pub fn active_rollbacks(&self) -> Vec<RollbackOperation> {
        self.rollbacks
            .iter()
            .filter(|r| r.value().status != RollbackStatus::Completed)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get all tracked changes.
    pub fn tracked_changes(&self) -> Vec<TrackedChange> {
        self.tracked_changes.iter().map(|r| r.value().clone()).collect()
    }

    /// Get count of completed rollbacks.
    pub fn completed_count(&self) -> usize {
        self.rollbacks
            .iter()
            .filter(|r| r.value().status == RollbackStatus::Completed)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (RollbackManager, uuid::Uuid) {
        let proposer = Arc::new(Proposer::new());
        let rb = RollbackManager::with_defaults(proposer);
        let change_id = uuid::Uuid::new_v4();

        let change = TrackedChange {
            id: change_id,
            target_wing: WingId::Trading,
            diff: "Update entry threshold from 30 to 25".to_string(),
            baseline_score: 0.08,
            applied_at: Utc::now(),
            rolled_back: false,
            rollback_reason: None,
        };

        rb.track_change(change);
        (rb, change_id)
    }

    #[test]
    fn no_rollback_when_performance_improves() {
        let (rb, change_id) = setup();
        let result = rb.evaluate(change_id, 0.09); // Score improved
        assert!(result.is_none());
    }

    #[test]
    fn no_rollback_within_threshold() {
        let (rb, change_id) = setup();
        // 3% degradation — within 5% threshold
        let result = rb.evaluate(change_id, 0.0776);
        assert!(result.is_none());
    }

    #[test]
    fn rollback_triggered_on_degradation() {
        let (rb, change_id) = setup();
        // 10% degradation — exceeds 5% threshold
        let result = rb.evaluate(change_id, 0.072);
        assert!(result.is_some());
        let rb_op = result.unwrap();
        assert_eq!(rb_op.status, RollbackStatus::Queued);
        assert_eq!(rb_op.target_wing, WingId::Trading);
        assert!(rb_op.reason.contains("degraded"));
    }

    #[test]
    fn execute_rollback() {
        let (rb, change_id) = setup();
        rb.evaluate(change_id, 0.072).unwrap();
        let result = rb.execute_rollback(&change_id).unwrap();
        assert_eq!(result.status, RollbackStatus::Completed);
        assert!(result.completed_at.is_some());
    }

    #[test]
    fn double_rollback_fails() {
        let (rb, change_id) = setup();
        rb.evaluate(change_id, 0.072).unwrap();
        rb.execute_rollback(&change_id).unwrap();
        // Already completed — second call should fail.
        assert!(rb.execute_rollback(&change_id).is_err());
    }

    #[test]
    fn rolled_back_change_not_re_evaluated() {
        let (rb, change_id) = setup();
        rb.evaluate(change_id, 0.072).unwrap();
        rb.execute_rollback(&change_id).unwrap();
        // Re-evaluating should return None since change is marked rolled_back.
        let result = rb.evaluate(change_id, 0.01);
        assert!(result.is_none());
    }

    #[test]
    fn tracked_change_is_marked_rolled_back() {
        let (rb, change_id) = setup();
        rb.evaluate(change_id, 0.072).unwrap();
        rb.execute_rollback(&change_id).unwrap();

        let changes = rb.tracked_changes();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].rolled_back);
        assert!(changes[0].rollback_reason.is_some());
    }

    #[test]
    fn active_and_completed_counts() {
        let (rb, change_id) = setup();
        rb.evaluate(change_id, 0.072).unwrap();
        assert_eq!(rb.active_rollbacks().len(), 1);
        assert_eq!(rb.completed_count(), 0);

        rb.execute_rollback(&change_id).unwrap();
        assert_eq!(rb.active_rollbacks().len(), 0);
        assert_eq!(rb.completed_count(), 1);
    }

    #[test]
    fn rollback_message_creation() {
        let change_id = uuid::Uuid::new_v4();
        let msg = RollbackManager::create_rollback_message(
            change_id,
            "Performance degraded 8%".to_string(),
        );
        assert_eq!(msg.from, WingId::Evolve);
        assert!(matches!(msg.payload, Payload::RollbackRequest { .. }));
    }

    #[test]
    fn custom_threshold() {
        let proposer = Arc::new(Proposer::new());
        let rb = RollbackManager::new(
            RollbackConfig {
                degradation_threshold: 0.10, // 10% threshold
                ..Default::default()
            },
            proposer,
        );
        let change_id = uuid::Uuid::new_v4();
        rb.track_change(TrackedChange {
            id: change_id,
            target_wing: WingId::Trading,
            diff: "test diff".to_string(),
            baseline_score: 0.08,
            applied_at: Utc::now(),
            rolled_back: false,
            rollback_reason: None,
        });

        // 6% degradation — below 10% custom threshold.
        let result = rb.evaluate(change_id, 0.0752);
        assert!(result.is_none());

        // 12% degradation — exceeds custom threshold.
        let result = rb.evaluate(change_id, 0.0704);
        assert!(result.is_some());
    }
}
