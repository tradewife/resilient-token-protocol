//! Audit Wing — soulcontract enforcement and compliance.
//!
//! Implements the red-team-tribunal pattern: 3-agent adversarial review
//! (Skeptic + User Proxy + Optimizer) with configurable consensus mechanisms
//! (Majority, Weighted, Byzantine).
//!
//! Reference: red-team-tribunal skill — 3-agent adversarial review is the
//! Audit Wing's core pattern. Every wing proposal must pass tribunal review.

use crate::types::{Message, Payload, ProposalKind, RiskLevel, WingId};
use serde::{Deserialize, Serialize};

/// Consensus algorithm for tribunal decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusMode {
    /// Simple majority — most votes win.
    Majority,
    /// Queen (Audit Wing) vote counts 3x weight.
    Weighted,
    /// Requires 2/3 supermajority for approval.
    Byzantine,
}

impl Default for ConsensusMode {
    fn default() -> Self {
        ConsensusMode::Byzantine
    }
}

/// Vote from a tribunal agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Approve,
    Reject,
    Abstain,
}

impl Vote {
    pub fn approved(&self) -> bool {
        matches!(self, Vote::Approve)
    }
}

/// An individual agent's review within the tribunal.
#[derive(Debug, Clone)]
pub struct AgentReview {
    pub agent: TribunalAgent,
    pub vote: Vote,
    pub score: f64,       // 0.0-1.0 confidence in this vote
    pub findings: Vec<String>,
}

/// The three tribunal agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TribunalAgent {
    /// Looks for flaws, risks, and reasons to reject.
    Skeptic,
    /// Represents the end-user perspective — is this safe and useful?
    UserProxy,
    /// Looks for benefits and reasons to approve.
    Optimizer,
}

impl TribunalAgent {
    /// Weight multiplier for this agent in weighted consensus.
    pub fn weight(&self) -> u32 {
        match self {
            TribunalAgent::Skeptic => 2,
            TribunalAgent::UserProxy => 2,
            TribunalAgent::Optimizer => 1,
        }
    }

    fn all() -> [TribunalAgent; 3] {
        [TribunalAgent::Skeptic, TribunalAgent::UserProxy, TribunalAgent::Optimizer]
    }
}

/// Result of a tribunal review.
#[derive(Debug, Clone)]
pub struct TribunalResult {
    pub approved: bool,
    pub confidence: f64,      // 0.0-1.0
    pub risk_level: RiskLevel,
    pub reviews: Vec<AgentReview>,
    pub consensus_mode: ConsensusMode,
}

/// The Audit Wing — conducts adversarial tribunal reviews on proposals.
pub struct AuditWing {
    pub consensus_mode: ConsensusMode,
}

impl AuditWing {
    pub fn new() -> Self {
        Self {
            consensus_mode: ConsensusMode::Byzantine,
        }
    }

    pub fn with_consensus(mode: ConsensusMode) -> Self {
        Self { consensus_mode: mode }
    }

    /// Run a full 3-agent tribunal review on a proposal message.
    /// Returns the tribunal result and an AuditResult message for routing.
    pub fn review_proposal(&self, msg: &Message) -> (TribunalResult, Message) {
        let reviews = self.conduct_reviews(msg);
        let result = self.compute_consensus(&reviews);
        let response = Message::new(
            WingId::Audit,
            WingId::Coordinator,
            Payload::AuditResult {
                proposal_id: msg.id,
                approved: result.approved,
                risk_level: result.risk_level.clone(),
                findings: result.reviews.iter().flat_map(|r| r.findings.clone()).collect(),
            },
        );
        (result, response)
    }

    /// Quick stub review — auto-approves safe proposals, rejects dangerous ones.
    /// Used when full tribunal is overkill (e.g. heartbeats, knowledge queries).
    pub fn stub_review(msg: &Message) -> Option<Message> {
        match &msg.payload {
            Payload::Proposal { kind, confidence, .. } => {
                let (approved, risk) = match kind {
                    ProposalKind::SoulcontractAmendment => (false, RiskLevel::Critical),
                    ProposalKind::RiskThresholdChange => (false, RiskLevel::High),
                    ProposalKind::PhaseTransition => (false, RiskLevel::High),
                    _ => {
                        if *confidence >= 0.5 {
                            (true, RiskLevel::Low)
                        } else {
                            (false, RiskLevel::Medium)
                        }
                    }
                };
                Some(Message::new(
                    WingId::Audit,
                    WingId::Coordinator,
                    Payload::AuditResult {
                        proposal_id: msg.id,
                        approved,
                        risk_level: risk,
                        findings: if approved { vec![] } else { vec![format!("{:?} not auto-approved", kind)] },
                    },
                ))
            }
            Payload::EvolveProposal { .. } => Some(Message::new(
                WingId::Audit,
                WingId::Coordinator,
                Payload::AuditResult {
                    proposal_id: msg.id,
                    approved: true,
                    risk_level: RiskLevel::Medium,
                    findings: vec!["Evolve proposal — stub approved".to_string()],
                },
            )),
            _ => None,
        }
    }

    /// Conduct the 3-agent adversarial review.
    fn conduct_reviews(&self, msg: &Message) -> Vec<AgentReview> {
        let base_risk = self.assess_base_risk(msg);
        TribunalAgent::all().iter().map(|agent| {
            let (vote, score, findings) = self.agent_review(*agent, msg, base_risk);
            AgentReview { agent: *agent, vote, score, findings }
        }).collect()
    }

    /// Base risk assessment before tribunal agents weigh in.
    fn assess_base_risk(&self, msg: &Message) -> RiskLevel {
        match &msg.payload {
            Payload::Proposal { kind, confidence, .. } => {
                if *confidence < 0.3 {
                    return RiskLevel::High;
                }
                match kind {
                    ProposalKind::SoulcontractAmendment => RiskLevel::Critical,
                    ProposalKind::RiskThresholdChange => RiskLevel::High,
                    ProposalKind::PhaseTransition => RiskLevel::High,
                    ProposalKind::ArchitectureChange => RiskLevel::Medium,
                    _ => RiskLevel::Low,
                }
            }
            Payload::EvolveProposal { .. } => RiskLevel::Medium,
            _ => RiskLevel::Low,
        }
    }

    /// Individual agent review logic.
    fn agent_review(
        &self,
        agent: TribunalAgent,
        msg: &Message,
        base_risk: RiskLevel,
    ) -> (Vote, f64, Vec<String>) {
        let confidence = msg.payload.payload_confidence();

        match agent {
            TribunalAgent::Skeptic => {
                // Skeptic is more cautious with low-confidence proposals.
                let findings = vec![format!(
                    "Base risk: {}. Proposal confidence: {:.2}.",
                    base_risk, confidence
                )];
                if base_risk == RiskLevel::Critical || confidence < 0.4 {
                    (Vote::Reject, 0.8, findings)
                } else if base_risk == RiskLevel::High {
                    (Vote::Reject, 0.7, findings)
                } else {
                    (Vote::Approve, 0.6, findings)
                }
            }
            TribunalAgent::UserProxy => {
                // User proxy focuses on safety and utility.
                let findings = vec![format!("User-facing risk: {}", base_risk)];
                if base_risk == RiskLevel::Critical {
                    (Vote::Reject, 0.9, findings)
                } else if confidence >= 0.5 {
                    (Vote::Approve, 0.7, findings)
                } else {
                    (Vote::Abstain, 0.5, findings)
                }
            }
            TribunalAgent::Optimizer => {
                // Optimizer is more permissive — looks for benefits.
                let findings = vec!["Proposal may improve system performance.".to_string()];
                if base_risk == RiskLevel::Critical {
                    (Vote::Reject, 0.6, findings)
                } else {
                    (Vote::Approve, 0.8, findings)
                }
            }
        }
    }

    /// Compute consensus from agent reviews.
    fn compute_consensus(&self, reviews: &[AgentReview]) -> TribunalResult {
        let (approved, confidence) = match self.consensus_mode {
            ConsensusMode::Majority => {
                let approve_count = reviews.iter().filter(|r| r.vote == Vote::Approve).count();
                let reject_count = reviews.iter().filter(|r| r.vote == Vote::Reject).count();
                let total = reviews.len();
                let approved = approve_count > reject_count;
                let confidence = if total > 0 { approve_count as f64 / total as f64 } else { 0.0 };
                (approved, confidence)
            }
            ConsensusMode::Weighted => {
                let mut approve_weight = 0u32;
                let mut total_weight = 0u32;
                for r in reviews {
                    let w = r.agent.weight();
                    total_weight += w;
                    match r.vote {
                        Vote::Approve => approve_weight += w,
                        Vote::Reject => {},
                        Vote::Abstain => total_weight -= w / 2,
                    }
                }
                let approved = total_weight > 0 && (approve_weight as f64 / total_weight as f64) > 0.5;
                let confidence = if total_weight > 0 { approve_weight as f64 / total_weight as f64 } else { 0.0 };
                (approved, confidence)
            }
            ConsensusMode::Byzantine => {
                // 2/3 supermajority required.
                let approve_count = reviews.iter().filter(|r| r.vote == Vote::Approve).count();
                let total = reviews.len();
                let approved = total > 0 && (approve_count as f64 / total as f64) >= (2.0 / 3.0);
                let confidence = if total > 0 { approve_count as f64 / total as f64 } else { 0.0 };
                (approved, confidence)
            }
        };

        // Determine risk level from reviews.
        let risk_level = if !approved {
            let has_critical = reviews.iter().any(|r| {
                r.vote == Vote::Reject && r.findings.iter().any(|f| f.contains("Critical"))
            });
            if has_critical { RiskLevel::Critical } else { RiskLevel::High }
        } else {
            let reject_count = reviews.iter().filter(|r| r.vote == Vote::Reject).count();
            if reject_count > 0 { RiskLevel::Medium } else { RiskLevel::Low }
        };

        TribunalResult {
            approved,
            confidence,
            risk_level,
            reviews: reviews.to_vec(),
            consensus_mode: self.consensus_mode,
        }
    }
}

impl Default for AuditWing {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension to extract confidence from any payload.
trait PayloadConfidence {
    fn payload_confidence(&self) -> f64;
}

impl PayloadConfidence for Payload {
    fn payload_confidence(&self) -> f64 {
        match self {
            Payload::Proposal { confidence, .. } => *confidence,
            Payload::EvolveProposal { .. } => 0.7, // Evolve proposals get moderate confidence.
            _ => 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strategy_proposal(confidence: f64) -> Message {
        Message::new(
            WingId::Coordinator,
            WingId::Audit,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "test".to_string(),
                changes: serde_json::json!({}),
                confidence,
            },
        )
    }

    fn amendment_proposal() -> Message {
        Message::new(
            WingId::Coordinator,
            WingId::Audit,
            Payload::Proposal {
                kind: ProposalKind::SoulcontractAmendment,
                description: "test".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.99,
            },
        )
    }

    #[test]
    fn byzantine_approves_safe_strategy() {
        let audit = AuditWing::with_consensus(ConsensusMode::Byzantine);
        let msg = strategy_proposal(0.9);
        let (result, response) = audit.review_proposal(&msg);
        assert!(result.approved);
        assert!(result.reviews.len() == 3);
        assert_eq!(result.consensus_mode, ConsensusMode::Byzantine);
        match response.payload {
            Payload::AuditResult { approved, .. } => assert!(approved),
            _ => panic!("Expected AuditResult"),
        }
    }

    #[test]
    fn byzantine_rejects_amendment() {
        let audit = AuditWing::with_consensus(ConsensusMode::Byzantine);
        let msg = amendment_proposal();
        let (result, _) = audit.review_proposal(&msg);
        assert!(!result.approved);
        assert!(result.reviews.iter().any(|r| r.vote == Vote::Reject));
    }

    #[test]
    fn byzantine_rejects_low_confidence() {
        let audit = AuditWing::with_consensus(ConsensusMode::Byzantine);
        let msg = strategy_proposal(0.2);
        let (result, _) = audit.review_proposal(&msg);
        // Skeptic rejects (low confidence), UserProxy abstains, Optimizer approves.
        // 1/3 approve < 2/3 threshold → rejected.
        assert!(!result.approved);
    }

    #[test]
    fn majority_mode() {
        let audit = AuditWing::with_consensus(ConsensusMode::Majority);
        let msg = strategy_proposal(0.9);
        let (result, _) = audit.review_proposal(&msg);
        // All 3 approve → majority passes.
        assert!(result.approved);
    }

    #[test]
    fn weighted_mode() {
        let audit = AuditWing::with_consensus(ConsensusMode::Weighted);
        let msg = strategy_proposal(0.9);
        let (result, _) = audit.review_proposal(&msg);
        assert!(result.approved);
    }

    #[test]
    fn risk_level_from_reviews() {
        let audit = AuditWing::new();
        let msg = strategy_proposal(0.9);
        let (result, _) = audit.review_proposal(&msg);
        assert_eq!(result.risk_level, RiskLevel::Low);
    }

    #[test]
    fn confidence_score() {
        let audit = AuditWing::new();
        let msg = strategy_proposal(0.9);
        let (result, _) = audit.review_proposal(&msg);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn tribunal_has_three_agents() {
        let audit = AuditWing::new();
        let msg = strategy_proposal(0.9);
        let (result, _) = audit.review_proposal(&msg);
        assert_eq!(result.reviews.len(), 3);
        let agents: Vec<_> = result.reviews.iter().map(|r| r.agent).collect();
        assert!(agents.contains(&TribunalAgent::Skeptic));
        assert!(agents.contains(&TribunalAgent::UserProxy));
        assert!(agents.contains(&TribunalAgent::Optimizer));
    }

    #[test]
    fn skeptic_rejects_critical() {
        let audit = AuditWing::new();
        let msg = amendment_proposal();
        let reviews = audit.conduct_reviews(&msg);
        let skeptic = reviews.iter().find(|r| r.agent == TribunalAgent::Skeptic).unwrap();
        assert_eq!(skeptic.vote, Vote::Reject);
    }

    #[test]
    fn stub_review_approves_strategy() {
        let msg = strategy_proposal(0.9);
        let response = AuditWing::stub_review(&msg).unwrap();
        match response.payload {
            Payload::AuditResult { approved, .. } => assert!(approved),
            _ => panic!("Expected AuditResult"),
        }
    }

    #[test]
    fn stub_review_rejects_amendment() {
        let msg = amendment_proposal();
        let response = AuditWing::stub_review(&msg).unwrap();
        match response.payload {
            Payload::AuditResult { approved, .. } => assert!(!approved),
            _ => panic!("Expected AuditResult"),
        }
    }

    #[test]
    fn agent_weights() {
        assert_eq!(TribunalAgent::Skeptic.weight(), 2);
        assert_eq!(TribunalAgent::UserProxy.weight(), 2);
        assert_eq!(TribunalAgent::Optimizer.weight(), 1);
    }
}
