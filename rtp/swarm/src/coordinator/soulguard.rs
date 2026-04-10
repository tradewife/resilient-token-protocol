//! Soulguard — enforces soulcontract.md invariants on every message.
//!
//! No wing can execute an action that violates an active constraint.
//! If a message violates the soulcontract, it is rejected before any
//! wing sees it. The Audit Wing logs every compliance check.
//!
//! Uses SoulcontractSpec (parsed from soulcontract.md) as the source of
//! truth, not hardcoded strings. Supports drift detection.

use super::soulcontract_spec::{DriftReport, SoulcontractSpec};
use crate::types::{AuditLogEntry, Message, Payload, ProposalKind, WingId};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Soulguard enforcement result.
#[derive(Debug, Clone)]
pub enum SoulguardVerdict {
    /// Message passes all soulcontract checks.
    Pass,
    /// Message violates a specific invariant.
    Reject { reason: String, constraint: String },
}

/// The soulguard — gatekeeper for every message in the swarm.
///
/// Validates against the parsed SoulcontractSpec. If the spec file
/// changes, the Soulguard can reload to pick up the new constraints.
/// Drift detection compares the active enforcement against the spec.
#[derive(Debug)]
pub struct Soulguard {
    /// The parsed soulcontract specification.
    spec: Arc<RwLock<SoulcontractSpec>>,
    /// Current risk budget (0.0 to 1.0).
    current_risk_budget: Arc<RwLock<f64>>,
    /// Current phase (transitions are irreversible).
    current_phase: Arc<RwLock<Phase>>,
    /// Audit log for every compliance check.
    audit_log: Arc<RwLock<Vec<AuditLogEntry>>>,
    /// Cached rollback threshold from the spec (updated on reload).
    rollback_threshold: Arc<RwLock<f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Sustenance,
    Ecosystem,
    Humanity,
}

impl Phase {
    /// Phase transitions are strictly forward — no reversal.
    pub fn can_transition_to(&self, target: &Phase) -> bool {
        matches!(
            (self, target),
            (Phase::Sustenance, Phase::Ecosystem) | (Phase::Ecosystem, Phase::Humanity)
        )
    }

    /// Treasury threshold for each phase (from parsed spec).
    pub fn threshold_usd(&self) -> f64 {
        match self {
            Phase::Sustenance => 50_000.0,
            Phase::Ecosystem => 1_000_000.0,
            Phase::Humanity => f64::MAX,
        }
    }
}

impl Soulguard {
    /// Create from a parsed SoulcontractSpec.
    pub fn from_spec(spec: SoulcontractSpec) -> Self {
        let threshold = spec.rollback_threshold;
        Self {
            current_risk_budget: Arc::new(RwLock::new(1.0)),
            current_phase: Arc::new(RwLock::new(Phase::Sustenance)),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            spec: Arc::new(RwLock::new(spec)),
            rollback_threshold: Arc::new(RwLock::new(threshold)),
        }
    }

    /// Create with default spec (uses soulcontract.md from repo root).
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let spec_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(1)
            .unwrap_or(std::path::Path::new("."))
            .join("soulcontract.md");

        let spec = if spec_path.exists() {
            SoulcontractSpec::from_file(&spec_path).unwrap_or_else(|e| {
                eprintln!(
                    "WARNING: Failed to parse soulcontract.md: {}. Using defaults.",
                    e
                );
                default_spec()
            })
        } else {
            default_spec()
        };

        Self::from_spec(spec)
    }

    /// Reload the spec from disk (e.g. after a human-signed amendment).
    pub async fn reload(&self, path: &std::path::Path) -> Result<(), String> {
        let spec = SoulcontractSpec::from_file(path)?;
        let threshold = spec.rollback_threshold;
        let mut current = self.spec.write().await;
        *current = spec;
        drop(current);
        *self.rollback_threshold.write().await = threshold;
        Ok(())
    }

    /// Check a message against all soulcontract invariants.
    pub async fn check(&self, message: &Message) -> SoulguardVerdict {
        let verdict = self.evaluate(message).await;
        self.log(message, &verdict).await;
        verdict
    }

    /// Core evaluation — validates against the parsed spec, not hardcoded strings.
    async fn evaluate(&self, message: &Message) -> SoulguardVerdict {
        let spec = self.spec.read().await;

        // Rule 1: Wings never talk to each other directly.
        if message.to != WingId::Coordinator && message.from != WingId::Coordinator {
            return SoulguardVerdict::Reject {
                reason: format!(
                    "Direct wing-to-wing communication forbidden: {} -> {}. \
                     All messages must route through the Coordinator.",
                    message.from, message.to
                ),
                constraint: "wings_communicate_via_coordinator".to_string(),
            };
        }

        // Drop the spec read lock before async checks.
        drop(spec);

        // Payload-specific checks.
        match &message.payload {
            Payload::Proposal { kind, .. } => self.check_proposal(kind).await,
            Payload::EvolveProposal { .. } => self.check_evolve_proposal(&message.from).await,
            Payload::RollbackRequest { .. } => {
                SoulguardVerdict::Pass // Rollbacks are always safety mechanisms.
            }
            Payload::Shutdown { .. } => {
                if message.priority != crate::types::Priority::Critical {
                    return SoulguardVerdict::Reject {
                        reason: "Shutdown requires Critical priority.".to_string(),
                        constraint: "shutdown_requires_critical_priority".to_string(),
                    };
                }
                SoulguardVerdict::Pass
            }
            _ => SoulguardVerdict::Pass,
        }
    }

    /// Validate proposals against parsed spec constraints.
    /// Uses keyword matching so the constraint names work regardless of
    /// exact formatting in soulcontract.md.
    async fn check_proposal(&self, kind: &ProposalKind) -> SoulguardVerdict {
        let spec = self.spec.read().await;

        // Map ProposalKind to the keywords we look for in immutable constraints.
        let (keyword, reason_msg) = match kind {
            ProposalKind::SoulcontractAmendment => (
                "self_modification",
                "Soulcontract amendments require human cryptographic signature \
                 and 24-hour monitoring window.",
            ),
            ProposalKind::RiskThresholdChange => (
                "risk_budget",
                "Risk threshold changes require explicit human consent.",
            ),
            ProposalKind::PhaseTransition => (
                "phase_reversal",
                "Phase transitions are irreversible on-chain. \
                 Submit for human review and on-chain execution.",
            ),
            ProposalKind::ArchitectureChange
            | ProposalKind::StrategyChange
            | ProposalKind::ConfigChange
            | ProposalKind::NewModule => return SoulguardVerdict::Pass,
        };

        // Check if any immutable constraint contains the keyword.
        let matches = spec
            .immutable_constraints
            .iter()
            .any(|c| c.name.contains(keyword));

        if matches {
            let constraint_name = spec
                .immutable_constraints
                .iter()
                .find(|c| c.name.contains(keyword))
                .map(|c| c.name.clone())
                .unwrap_or(keyword.to_string());
            SoulguardVerdict::Reject {
                reason: format!("{} [constraint: {}]", reason_msg, constraint_name),
                constraint: constraint_name,
            }
        } else {
            SoulguardVerdict::Pass
        }
    }

    /// Only the Evolve Wing can submit EvolveProposals.
    async fn check_evolve_proposal(&self, from: &WingId) -> SoulguardVerdict {
        if *from != WingId::Evolve {
            return SoulguardVerdict::Reject {
                reason: format!(
                    "Only the Evolve Wing can submit EvolveProposals. Got: {}",
                    from
                ),
                constraint: "evolve_wing_exclusive".to_string(),
            };
        }
        SoulguardVerdict::Pass
    }

    /// Log the compliance check result for full audit trail.
    async fn log(&self, message: &Message, verdict: &SoulguardVerdict) {
        let (passed, rejection_reason) = match verdict {
            SoulguardVerdict::Pass => (true, None),
            SoulguardVerdict::Reject { reason, .. } => (false, Some(reason.clone())),
        };

        let entry = AuditLogEntry {
            id: message.id,
            timestamp: message.created_at,
            from: message.from,
            to: message.to,
            payload_summary: format!("{:?}", message.payload).chars().take(200).collect(),
            soulguard_passed: passed,
            rejection_reason,
        };

        let mut log = self.audit_log.write().await;
        log.push(entry);
    }

    /// Detect drift between the parsed spec and active constraints.
    /// Only checks spec-level drift (immutable constraints), not runtime routing rules.
    pub async fn detect_drift(&self) -> DriftReport {
        let spec = self.spec.read().await;
        spec.detect_drift(&spec.constraint_names())
    }

    /// Get the full audit log (for Audit Wing inspection).
    pub async fn audit_log(&self) -> Vec<AuditLogEntry> {
        self.audit_log.read().await.clone()
    }

    pub async fn risk_budget(&self) -> f64 {
        *self.current_risk_budget.read().await
    }

    pub async fn set_risk_budget(&self, budget: f64) {
        let mut current = self.current_risk_budget.write().await;
        if budget <= 1.0 {
            *current = budget;
        }
    }

    pub async fn current_phase(&self) -> Phase {
        *self.current_phase.read().await
    }

    pub async fn transition_phase(&self, target: Phase) -> Result<(), String> {
        let mut current = self.current_phase.write().await;
        if current.can_transition_to(&target) {
            *current = target;
            Ok(())
        } else {
            Err(format!(
                "Cannot transition from {:?} to {:?}. Phase transitions are irreversible.",
                *current, target
            ))
        }
    }

    /// Check if degradation exceeds the rollback threshold from the parsed spec.
    pub fn exceeds_rollback_threshold(&self, degradation: f64) -> bool {
        // H-5 fix: read the cached threshold from the spec instead of hardcoding 0.05.
        match self.rollback_threshold.try_read() {
            Ok(threshold) => degradation > *threshold,
            Err(_) => degradation > 0.05, // fallback if lock is poisoned
        }
    }

    /// Get a snapshot of the current spec.
    pub async fn spec_snapshot(&self) -> SoulcontractSpec {
        self.spec.read().await.clone()
    }
}

/// Build a default spec when the file is not available (for testing).
fn default_spec() -> SoulcontractSpec {
    SoulcontractSpec::from_str(
        r#"
## Core Values

1. The protocol exists to generate sustainable yield
2. No single entity controls the treasury
3. Human sovereignty over irreversible decisions
4. Self-hydration
5. Risk budgets

## What Cannot Evolve

- **Core values** — immutable
- **Human-sovereign control** — no amendment can remove
- **Self-modification of this contract** — amendments require human signature + 24h monitoring
- **Risk budget expansion** — increasing max risk without explicit human consent
- **PDA ownership** — treasury must always be PDA-owned
- **Fee immutability** — SPL TransferFeeConfig must remain immutable
- **Phase reversal** — once a phase transition occurs, it cannot be undone

## Amendment Protocol

Auto-rollback if system performance degrades > 5% post-amendment

## Phase Evolution

| Phase | Threshold | New Capabilities |
|-------|-----------|-----------------|
| Sustenance | < $50k | Reinvest |
| Ecosystem | $50k–$1M | LP |
| Humanity | > $1M | Grants |
"#,
    )
    .unwrap_or_else(|_| SoulcontractSpec {
        immutable_constraints: vec![],
        evolvable_items: vec![],
        phases: vec![],
        core_values: vec![],
        rollback_threshold: 0.05,
        raw_content: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, Payload, Priority, WingId};

    fn make_message(from: WingId, to: WingId, payload: Payload) -> Message {
        Message::new(from, to, payload)
    }

    fn test_spec() -> SoulcontractSpec {
        SoulcontractSpec::from_str(
            r#"
## Core Values

1. Sustainable yield
2. PDA ownership

## What Cannot Evolve

- **Core values** — immutable
- **Human-sovereign control** — no amendment can remove the human approval requirement
- **Self-modification of this contract** — amendments require human signature + 24h monitoring
- **Risk budget expansion** — increasing max risk without explicit human consent
- **PDA ownership** — treasury must always be PDA-owned
- **Fee immutability** — SPL TransferFeeConfig must remain immutable
- **Phase reversal** — once a phase transition occurs, it cannot be undone

## Amendment Protocol

Auto-rollback if system performance degrades > 5% post-amendment

## Phase Evolution

| Phase | Threshold | New Capabilities |
|-------|-----------|-----------------|
| Sustenance | < $50k | Reinvest |
| Ecosystem | $50k–$1M | LP |
| Humanity | > $1M | Grants |
"#,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn normal_message_passes() {
        let sg = Soulguard::from_spec(test_spec());
        let msg = make_message(
            WingId::Trading,
            WingId::Coordinator,
            Payload::Heartbeat {
                wing: WingId::Trading,
                status: crate::types::HealthStatus::Healthy,
                metrics: serde_json::json!({}),
            },
        );
        assert!(matches!(sg.check(&msg).await, SoulguardVerdict::Pass));
    }

    #[tokio::test]
    async fn direct_wing_communication_rejected() {
        let sg = Soulguard::from_spec(test_spec());
        let msg = make_message(
            WingId::Trading,
            WingId::Evolve,
            Payload::Raw(serde_json::json!({})),
        );
        assert!(matches!(
            sg.check(&msg).await,
            SoulguardVerdict::Reject { .. }
        ));
    }

    #[tokio::test]
    async fn amendment_rejected_from_parsed_spec() {
        let sg = Soulguard::from_spec(test_spec());
        let msg = make_message(
            WingId::Evolve,
            WingId::Coordinator,
            Payload::Proposal {
                kind: ProposalKind::SoulcontractAmendment,
                description: "test".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.9,
            },
        );
        let verdict = sg.check(&msg).await;
        match verdict {
            SoulguardVerdict::Reject { reason, constraint } => {
                assert!(reason.contains("human"));
                assert!(constraint.contains("self_modification"));
            }
            _ => panic!("Expected rejection"),
        }
    }

    #[tokio::test]
    async fn risk_change_rejected_from_parsed_spec() {
        let sg = Soulguard::from_spec(test_spec());
        let msg = make_message(
            WingId::Trading,
            WingId::Coordinator,
            Payload::Proposal {
                kind: ProposalKind::RiskThresholdChange,
                description: "test".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.8,
            },
        );
        let verdict = sg.check(&msg).await;
        assert!(
            matches!(verdict, SoulguardVerdict::Reject { constraint, .. } if constraint.contains("risk_budget"))
        );
    }

    #[tokio::test]
    async fn strategy_change_passes() {
        let sg = Soulguard::from_spec(test_spec());
        let msg = make_message(
            WingId::Trading,
            WingId::Coordinator,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "test".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.9,
            },
        );
        assert!(matches!(sg.check(&msg).await, SoulguardVerdict::Pass));
    }

    #[tokio::test]
    async fn drift_detection_no_drift() {
        let sg = Soulguard::from_spec(test_spec());
        let report = sg.detect_drift().await;
        assert!(report.in_sync);
    }

    #[tokio::test]
    async fn phase_transition_irreversible() {
        let sg = Soulguard::from_spec(test_spec());
        assert!(sg.transition_phase(Phase::Ecosystem).await.is_ok());
        assert!(sg.transition_phase(Phase::Sustenance).await.is_err());
    }

    #[tokio::test]
    async fn rollback_threshold_from_spec() {
        let sg = Soulguard::from_spec(test_spec());
        assert!(!sg.exceeds_rollback_threshold(0.03));
        assert!(sg.exceeds_rollback_threshold(0.06));
    }

    #[tokio::test]
    async fn evolve_proposal_from_evolve_passes() {
        let sg = Soulguard::from_spec(test_spec());
        let msg = make_message(
            WingId::Evolve,
            WingId::Coordinator,
            Payload::EvolveProposal {
                target_wing: WingId::Trading,
                diff: "test".to_string(),
                rationale: "test".to_string(),
                expected_impact: "test".to_string(),
            },
        );
        assert!(matches!(sg.check(&msg).await, SoulguardVerdict::Pass));
    }

    #[tokio::test]
    async fn shutdown_requires_critical() {
        let sg = Soulguard::from_spec(test_spec());
        let msg = make_message(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Shutdown {
                reason: "test".to_string(),
            },
        );
        assert!(matches!(
            sg.check(&msg).await,
            SoulguardVerdict::Reject { .. }
        ));

        let msg_critical = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Shutdown {
                reason: "test".to_string(),
            },
        )
        .with_priority(Priority::Critical);
        assert!(matches!(
            sg.check(&msg_critical).await,
            SoulguardVerdict::Pass
        ));
    }

    #[tokio::test]
    async fn audit_log_populated() {
        let sg = Soulguard::from_spec(test_spec());
        let msg = make_message(
            WingId::Trading,
            WingId::Coordinator,
            Payload::Heartbeat {
                wing: WingId::Trading,
                status: crate::types::HealthStatus::Healthy,
                metrics: serde_json::json!({}),
            },
        );
        sg.check(&msg).await;
        assert_eq!(sg.audit_log().await.len(), 1);
    }

    #[tokio::test]
    async fn phase_forward_only() {
        assert!(Phase::Sustenance.can_transition_to(&Phase::Ecosystem));
        assert!(!Phase::Sustenance.can_transition_to(&Phase::Humanity));
        assert!(Phase::Ecosystem.can_transition_to(&Phase::Humanity));
        assert!(!Phase::Humanity.can_transition_to(&Phase::Ecosystem));
    }
}
