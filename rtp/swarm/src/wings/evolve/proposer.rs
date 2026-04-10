//! Proposer — architecture change proposals for the Evolve Wing.
//!
//! Reference: https://github.com/karpathy/autoresearch (Modify/Verify/Keep spec)
//!
//! Every change is a diff that goes through the Coordinator and must pass
//! the Audit Wing. The proposer follows the SPARC methodology:
//!   Specify -> Pseudocode -> Architect -> Refine -> Complete

use crate::types::{Message, Payload, WingId};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Status of a proposed change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Proposed, awaiting audit.
    Proposed,
    /// Approved by Audit Wing, awaiting execution.
    Approved,
    /// Executed successfully.
    Executed,
    /// Rejected by Audit Wing or soulguard.
    Rejected,
    /// Rolled back due to performance degradation.
    RolledBack,
}

/// An architecture change proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeProposal {
    /// Unique ID for this proposal.
    pub id: uuid::Uuid,
    /// The wing this proposal targets.
    pub target_wing: WingId,
    /// Description of the proposed change.
    pub description: String,
    /// The actual diff (code/config change).
    pub diff: String,
    /// Why this change is being proposed.
    pub rationale: String,
    /// Expected impact on performance.
    pub expected_impact: String,
    /// Current status of the proposal.
    pub status: ProposalStatus,
    /// When the proposal was created.
    pub created_at: DateTime<Utc>,
    /// When the proposal was last updated.
    pub updated_at: DateTime<Utc>,
    /// Assessment score before the change (baseline).
    pub baseline_score: Option<f64>,
    /// Assessment score after the change.
    pub post_score: Option<f64>,
    /// Rejection reason (if rejected).
    pub rejection_reason: Option<String>,
}

impl ChangeProposal {
    /// Create a new change proposal.
    pub fn new(
        target_wing: WingId,
        description: String,
        diff: String,
        rationale: String,
        expected_impact: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            target_wing,
            description,
            diff,
            rationale,
            expected_impact,
            status: ProposalStatus::Proposed,
            created_at: now,
            updated_at: now,
            baseline_score: None,
            post_score: None,
            rejection_reason: None,
        }
    }

    /// Transition the proposal to a new status.
    pub fn transition_to(&mut self, new_status: ProposalStatus) -> Result<(), String> {
        let valid = matches!(
            (&self.status, &new_status),
            (ProposalStatus::Proposed, ProposalStatus::Approved)
                | (ProposalStatus::Proposed, ProposalStatus::Rejected)
                | (ProposalStatus::Approved, ProposalStatus::Executed)
                | (ProposalStatus::Approved, ProposalStatus::Rejected)
                | (ProposalStatus::Executed, ProposalStatus::RolledBack)
        );

        if valid {
            self.status = new_status;
            self.updated_at = Utc::now();
            Ok(())
        } else {
            Err(format!(
                "Invalid transition: {:?} -> {:?}",
                self.status, new_status
            ))
        }
    }

    /// Convert to a swarm message for routing through the Coordinator.
    pub fn to_message(&self) -> Message {
        Message::new(
            WingId::Evolve,
            WingId::Coordinator,
            Payload::EvolveProposal {
                target_wing: self.target_wing,
                diff: self.diff.clone(),
                rationale: self.rationale.clone(),
                expected_impact: self.expected_impact.clone(),
            },
        )
    }
}

/// The Proposer — manages architecture change proposals.
pub struct Proposer {
    /// All proposals keyed by ID.
    proposals: Arc<DashMap<uuid::Uuid, ChangeProposal>>,
    /// Proposals per target wing.
    wing_proposals: Arc<DashMap<WingId, Vec<uuid::Uuid>>>,
}

impl Default for Proposer {
    fn default() -> Self {
        Self::new()
    }
}

impl Proposer {
    pub fn new() -> Self {
        Self {
            proposals: Arc::new(DashMap::new()),
            wing_proposals: Arc::new(DashMap::new()),
        }
    }

    /// Create a new architecture change proposal.
    pub fn propose(
        &self,
        target_wing: WingId,
        description: String,
        diff: String,
        rationale: String,
        expected_impact: String,
    ) -> ChangeProposal {
        let proposal =
            ChangeProposal::new(target_wing, description, diff, rationale, expected_impact);
        let id = proposal.id;

        self.wing_proposals.entry(target_wing).or_default().push(id);

        self.proposals.insert(id, proposal.clone());
        proposal
    }

    /// Create a proposal with a specific ID (for internal use when
    /// baseline score needs to be set before returning).
    pub fn propose_with_id(
        &self,
        id: uuid::Uuid,
        target_wing: WingId,
        description: String,
        diff: String,
        rationale: String,
        expected_impact: String,
    ) -> ChangeProposal {
        let now = Utc::now();
        let proposal = ChangeProposal {
            id,
            target_wing,
            description,
            diff,
            rationale,
            expected_impact,
            status: ProposalStatus::Proposed,
            created_at: now,
            updated_at: now,
            baseline_score: None,
            post_score: None,
            rejection_reason: None,
        };

        self.wing_proposals.entry(target_wing).or_default().push(id);

        self.proposals.insert(id, proposal.clone());
        proposal
    }

    /// Get a proposal by ID.
    pub fn get(&self, id: &uuid::Uuid) -> Option<ChangeProposal> {
        self.proposals.get(id).map(|p| p.clone())
    }

    /// Approve a proposal (called when Audit Wing approves).
    pub fn approve(&self, id: &uuid::Uuid) -> Result<(), String> {
        let mut proposal = self.proposals.get_mut(id).ok_or("Proposal not found")?;
        proposal.transition_to(ProposalStatus::Approved)
    }

    /// Reject a proposal (called when Audit Wing or soulguard rejects).
    pub fn reject(&self, id: &uuid::Uuid, reason: String) -> Result<(), String> {
        let mut proposal = self.proposals.get_mut(id).ok_or("Proposal not found")?;
        proposal.rejection_reason = Some(reason);
        proposal.transition_to(ProposalStatus::Rejected)
    }

    /// Mark a proposal as executed.
    pub fn mark_executed(&self, id: &uuid::Uuid) -> Result<(), String> {
        let mut proposal = self.proposals.get_mut(id).ok_or("Proposal not found")?;
        proposal.transition_to(ProposalStatus::Executed)
    }

    /// Record the baseline score before applying a change.
    pub fn set_baseline_score(&self, id: &uuid::Uuid, score: f64) -> Result<(), String> {
        let mut proposal = self.proposals.get_mut(id).ok_or("Proposal not found")?;
        proposal.baseline_score = Some(score);
        Ok(())
    }

    /// Record the post-change score.
    pub fn set_post_score(&self, id: &uuid::Uuid, score: f64) -> Result<(), String> {
        let mut proposal = self.proposals.get_mut(id).ok_or("Proposal not found")?;
        proposal.post_score = Some(score);
        Ok(())
    }

    /// Get all proposals for a specific wing.
    pub fn proposals_for_wing(&self, wing_id: WingId) -> Vec<ChangeProposal> {
        match self.wing_proposals.get(&wing_id) {
            Some(ids) => ids
                .iter()
                .filter_map(|id| self.proposals.get(id).map(|p| p.clone()))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get all proposals with a specific status.
    pub fn proposals_by_status(&self, status: ProposalStatus) -> Vec<ChangeProposal> {
        self.proposals
            .iter()
            .filter(|entry| entry.value().status == status)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get all proposals.
    pub fn all_proposals(&self) -> Vec<ChangeProposal> {
        self.proposals.iter().map(|e| e.value().clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proposal() -> ChangeProposal {
        ChangeProposal::new(
            WingId::Trading,
            "Optimize entry logic".to_string(),
            "diff --git a/entry.rs ...".to_string(),
            "Performance regression detected in backtests".to_string(),
            "+5% Sharpe ratio expected".to_string(),
        )
    }

    #[test]
    fn create_proposal() {
        let p = make_proposal();
        assert_eq!(p.status, ProposalStatus::Proposed);
        assert_eq!(p.target_wing, WingId::Trading);
        assert!(p.rejection_reason.is_none());
    }

    #[test]
    fn valid_transitions() {
        let mut p = make_proposal();
        assert!(p.transition_to(ProposalStatus::Approved).is_ok());
        assert_eq!(p.status, ProposalStatus::Approved);

        assert!(p.transition_to(ProposalStatus::Executed).is_ok());
        assert_eq!(p.status, ProposalStatus::Executed);

        assert!(p.transition_to(ProposalStatus::RolledBack).is_ok());
        assert_eq!(p.status, ProposalStatus::RolledBack);
    }

    #[test]
    fn invalid_transitions() {
        let mut p = make_proposal();
        // Cannot go directly from Proposed to Executed.
        assert!(p.transition_to(ProposalStatus::Executed).is_err());
        // Cannot go from Proposed to RolledBack.
        assert!(p.transition_to(ProposalStatus::RolledBack).is_err());
    }

    #[test]
    fn rejection_path() {
        let mut p = make_proposal();
        p.rejection_reason = Some("Too risky".to_string());
        assert!(p.transition_to(ProposalStatus::Rejected).is_ok());
    }

    #[test]
    fn proposer_lifecycle() {
        let proposer = Proposer::new();

        let proposal = proposer.propose(
            WingId::Trading,
            "Update RSI threshold".to_string(),
            "diff...".to_string(),
            "Better entry signals".to_string(),
            "+3% win rate".to_string(),
        );

        let id = proposal.id;
        assert_eq!(proposal.status, ProposalStatus::Proposed);

        // Approve.
        proposer.approve(&id).unwrap();
        let loaded = proposer.get(&id).unwrap();
        assert_eq!(loaded.status, ProposalStatus::Approved);

        // Execute.
        proposer.mark_executed(&id).unwrap();
        let loaded = proposer.get(&id).unwrap();
        assert_eq!(loaded.status, ProposalStatus::Executed);
    }

    #[test]
    fn reject_proposal() {
        let proposer = Proposer::new();
        let proposal = proposer.propose(
            WingId::Security,
            "Change scan interval".to_string(),
            "diff...".to_string(),
            "Faster detection".to_string(),
            "-2s latency".to_string(),
        );

        proposer
            .reject(&proposal.id, "Insufficient justification".to_string())
            .unwrap();
        let loaded = proposer.get(&proposal.id).unwrap();
        assert_eq!(loaded.status, ProposalStatus::Rejected);
        assert_eq!(
            loaded.rejection_reason,
            Some("Insufficient justification".to_string())
        );
    }

    #[test]
    fn proposals_for_wing() {
        let proposer = Proposer::new();
        proposer.propose(
            WingId::Trading,
            "Change 1".to_string(),
            "diff1".to_string(),
            "r1".to_string(),
            "i1".to_string(),
        );
        proposer.propose(
            WingId::Trading,
            "Change 2".to_string(),
            "diff2".to_string(),
            "r2".to_string(),
            "i2".to_string(),
        );
        proposer.propose(
            WingId::Security,
            "Change 3".to_string(),
            "diff3".to_string(),
            "r3".to_string(),
            "i3".to_string(),
        );

        let trading_proposals = proposer.proposals_for_wing(WingId::Trading);
        assert_eq!(trading_proposals.len(), 2);

        let security_proposals = proposer.proposals_for_wing(WingId::Security);
        assert_eq!(security_proposals.len(), 1);
    }

    #[test]
    fn score_tracking() {
        let proposer = Proposer::new();
        let proposal = proposer.propose(
            WingId::Trading,
            "Optimize".to_string(),
            "diff".to_string(),
            "r".to_string(),
            "i".to_string(),
        );

        proposer.set_baseline_score(&proposal.id, 0.08).unwrap();
        proposer.set_post_score(&proposal.id, 0.09).unwrap();

        let loaded = proposer.get(&proposal.id).unwrap();
        assert_eq!(loaded.baseline_score, Some(0.08));
        assert_eq!(loaded.post_score, Some(0.09));
    }

    #[test]
    fn proposal_to_message() {
        let p = make_proposal();
        let msg = p.to_message();
        assert_eq!(msg.from, WingId::Evolve);
        assert!(matches!(msg.payload, Payload::EvolveProposal { .. }));
    }
}
