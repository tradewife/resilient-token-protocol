//! The Coordinator — message bus for the RTP swarm.
//!
//! Wings never talk to each other. Everything goes through the Coordinator.
//! Every message passes through a multi-stage quality gate before routing.
//!
//! Pipeline (compound-engineering pattern):
//!   Stage 1: Soulguard (soulcontract enforcement)
//!   Stage 2: Router (typed routing with fault tolerance)
//!   Stage 3: Audit Wing review (consensus tribunal for proposals)

pub mod lifecycle;
pub mod router;
pub mod soulcontract_spec;
pub mod soulguard;

use crate::types::{Message, MessageId, WingId};
use router::{Router, RoutingOutcome, Topology};
use soulguard::Soulguard;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Quality gate result for a single stage.
#[derive(Debug, Clone)]
pub enum StageResult {
    /// Stage passed.
    Pass,
    /// Stage blocked the message.
    Block { reason: String, constraint: String },
}

/// The Coordinator — central message bus of the swarm.
///
/// Implements a multi-stage quality gate:
///   1. Soulguard: enforces soulcontract.md on every message
///   2. Router: typed message routing with fault tolerance
///   3. Audit Wing: consensus tribunal for proposals
pub struct Coordinator {
    pub(crate) router: Arc<Router>,
    soulguard: Arc<Soulguard>,
    lifecycle: Arc<lifecycle::LifecycleManager>,
}

impl Coordinator {
    pub fn new(health_config: lifecycle::HealthConfig) -> Self {
        Self {
            router: Arc::new(Router::new(128)),
            soulguard: Arc::new(Soulguard::new()),
            lifecycle: Arc::new(lifecycle::LifecycleManager::new(health_config)),
        }
    }

    /// Create with custom topology.
    pub fn with_topology(health_config: lifecycle::HealthConfig, topology: Topology) -> Self {
        Self {
            router: Arc::new(Router::new(128).with_topology(topology)),
            soulguard: Arc::new(Soulguard::new()),
            lifecycle: Arc::new(lifecycle::LifecycleManager::new(health_config)),
        }
    }

    /// Register a wing with the Coordinator.
    pub async fn register_wing(&self, wing_id: WingId) -> mpsc::Receiver<Message> {
        let rx = self.router.register_wing(wing_id).await;
        let _ = self.lifecycle.spawn(wing_id);
        tracing::info!(wing = %wing_id, "Wing registered with Coordinator");
        rx
    }

    /// Process a message through the multi-stage quality gate.
    ///
    /// Stage 1: Soulguard check (soulcontract enforcement)
    /// Stage 2: Router (typed routing with fault tolerance)
    ///
    /// For proposals, the Router sends to Audit Wing for Stage 3 review
    /// (consensus tribunal) which then completes the pipeline.
    pub async fn process(&self, message: &Message) -> ProcessingResult {
        // Stage 1: Soulguard.
        match self.soulguard.check(message).await {
            soulguard::SoulguardVerdict::Pass => {}
            soulguard::SoulguardVerdict::Reject { reason, constraint } => {
                return ProcessingResult::Rejected {
                    message_id: message.id,
                    reason,
                    constraint,
                    stage: 1,
                };
            }
        }

        // Stage 2: Route with fault tolerance.
        let outcome = self.router.route(message.clone()).await;
        ProcessingResult::Routed {
            outcome,
            message_id: message.id,
            stage: 2,
        }
    }

    /// Send a message directly from the Coordinator to a wing.
    pub async fn send_to(&self, wing_id: WingId, message: Message) -> RoutingOutcome {
        let coordinator_msg = Message::new(WingId::Coordinator, wing_id, message.payload)
            .with_priority(message.priority);
        self.router.route(coordinator_msg).await
    }

    /// Run drift detection between the parsed soulcontract and active enforcement.
    pub async fn detect_spec_drift(&self) -> soulcontract_spec::DriftReport {
        self.soulguard.detect_drift().await
    }

    pub fn soulguard(&self) -> &Soulguard {
        &self.soulguard
    }

    pub fn router(&self) -> &Router {
        &self.router
    }

    pub fn lifecycle(&self) -> &lifecycle::LifecycleManager {
        &self.lifecycle
    }
}

/// Result of processing a message through the Coordinator.
#[derive(Debug, Clone)]
pub enum ProcessingResult {
    /// Message passed all quality gates and was routed.
    Routed {
        outcome: RoutingOutcome,
        message_id: MessageId,
        stage: u8,
    },
    /// Message was rejected by a quality gate.
    Rejected {
        message_id: MessageId,
        reason: String,
        constraint: String,
        stage: u8,
    },
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::types::{Payload, ProposalKind, RiskLevel};

    fn health_config() -> lifecycle::HealthConfig {
        lifecycle::HealthConfig {
            check_interval: std::time::Duration::from_secs(30),
            degraded_after: std::time::Duration::from_secs(60),
            unhealthy_after: std::time::Duration::from_secs(120),
            retire_after: std::time::Duration::from_secs(300),
        }
    }

    #[tokio::test]
    async fn full_pipeline_proposal_to_execution() {
        let coord = Coordinator::new(health_config());
        let mut audit_rx = coord.register_wing(WingId::Audit).await;
        let mut trading_rx = coord.register_wing(WingId::Trading).await;

        let proposal = Message::new(
            WingId::Trading,
            WingId::Coordinator,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "Update RSI".to_string(),
                changes: serde_json::json!({"rsi_entry": 28}),
                confidence: 0.92,
            },
        );

        // Stage 1+2: Soulguard pass → route to Audit.
        let result = coord.process(&proposal).await;
        assert!(matches!(result, ProcessingResult::Routed { stage: 2, .. }));

        // Audit Wing receives proposal.
        let audit_msg = audit_rx.recv().await.unwrap();
        let proposal_id = audit_msg.id;

        // Audit approves → Stage 1+2 again for the audit response.
        let audit_response = Message::new(
            WingId::Audit,
            WingId::Coordinator,
            Payload::AuditResult {
                proposal_id,
                approved: true,
                risk_level: RiskLevel::Low,
                findings: vec![],
            },
        );
        let result = coord.process(&audit_response).await;
        assert!(matches!(result, ProcessingResult::Routed { stage: 2, .. }));

        // Trading Wing receives execution permit.
        let permit = trading_rx.recv().await.unwrap();
        assert!(matches!(permit.payload, Payload::ExecutePermit { .. }));
    }

    #[tokio::test]
    async fn soulguard_blocks_at_stage_1() {
        let coord = Coordinator::new(health_config());
        coord.register_wing(WingId::Trading).await;
        coord.register_wing(WingId::Security).await;

        let msg = Message::new(
            WingId::Trading,
            WingId::Security,
            Payload::Raw(serde_json::json!({})),
        );
        let result = coord.process(&msg).await;
        match result {
            ProcessingResult::Rejected { stage, .. } => assert_eq!(stage, 1),
            _ => panic!("Expected rejection at stage 1"),
        }
    }

    #[tokio::test]
    async fn soulguard_blocks_amendment() {
        let coord = Coordinator::new(health_config());
        coord.register_wing(WingId::Evolve).await;

        let msg = Message::new(
            WingId::Evolve,
            WingId::Coordinator,
            Payload::Proposal {
                kind: ProposalKind::SoulcontractAmendment,
                description: "Remove PDA".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.95,
            },
        );
        let result = coord.process(&msg).await;
        match result {
            ProcessingResult::Rejected { reason, stage, .. } => {
                assert_eq!(stage, 1);
                assert!(reason.contains("human"));
            }
            _ => panic!("Expected rejection"),
        }
    }

    #[tokio::test]
    async fn shutdown_broadcasts() {
        let coord = Coordinator::new(health_config());
        let mut t_rx = coord.register_wing(WingId::Trading).await;
        let mut e_rx = coord.register_wing(WingId::Evolve).await;
        let mut a_rx = coord.register_wing(WingId::Audit).await;

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Shutdown {
                reason: "test".to_string(),
            },
        )
        .with_priority(crate::types::Priority::Critical);

        let result = coord.process(&msg).await;
        assert!(matches!(result, ProcessingResult::Routed { .. }));
        assert!(t_rx.recv().await.is_some());
        assert!(e_rx.recv().await.is_some());
        assert!(a_rx.recv().await.is_some());
    }

    #[tokio::test]
    async fn drift_detection_works() {
        let coord = Coordinator::new(health_config());
        let report = coord.detect_spec_drift().await;
        assert!(report.in_sync);
    }

    // ── Week 3: New wing integration tests ─────────────────────────────

    #[tokio::test]
    async fn all_six_wings_registered() {
        let coord = Coordinator::new(health_config());
        coord.register_wing(WingId::Trading).await;
        coord.register_wing(WingId::Security).await;
        coord.register_wing(WingId::Evolve).await;
        coord.register_wing(WingId::Knowledge).await;
        coord.register_wing(WingId::Audit).await;
        coord.register_wing(WingId::Futureproof).await;
        assert_eq!(coord.lifecycle().active_count(), 6);
    }

    #[tokio::test]
    async fn security_wing_receives_routed_alert() {
        let coord = Coordinator::new(health_config());
        let mut security_rx = coord.register_wing(WingId::Security).await;

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Security,
            Payload::SecurityAlert {
                severity: RiskLevel::Medium,
                threat: "Anomaly detected".to_string(),
            },
        );
        let result = coord.process(&msg).await;
        assert!(matches!(result, ProcessingResult::Routed { stage: 2, .. }));

        let received = security_rx.recv().await.unwrap();
        assert!(matches!(received.payload, Payload::SecurityAlert { .. }));
    }

    #[tokio::test]
    async fn knowledge_wing_receives_routed_query() {
        let coord = Coordinator::new(health_config());
        let mut knowledge_rx = coord.register_wing(WingId::Knowledge).await;

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::KnowledgeQuery {
                query: "yield SOL".to_string(),
                context: None,
            },
        );
        let result = coord.process(&msg).await;
        assert!(matches!(result, ProcessingResult::Routed { stage: 2, .. }));

        let received = knowledge_rx.recv().await.unwrap();
        assert!(matches!(received.payload, Payload::KnowledgeQuery { .. }));
    }

    #[tokio::test]
    async fn knowledge_store_and_query_loop() {
        use crate::wings::knowledge::KnowledgeWing;

        let knowledge = KnowledgeWing::new();

        // Store a yield report via the wing handler.
        let yield_msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::YieldReport {
                usdc_yield: 5000.0,
                sol_reserves: 50000.0,
                drawdown: 0.03,
            },
        );
        let resp = knowledge.handle_message(&yield_msg);
        assert!(matches!(resp.unwrap().payload, Payload::Ack { .. }));

        // Query for it.
        let query_msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::KnowledgeQuery {
                query: "yield".to_string(),
                context: None,
            },
        );
        let resp = knowledge.handle_message(&query_msg).unwrap();
        match resp.payload {
            Payload::KnowledgeResult { results } => {
                assert!(results.iter().any(|r| r.contains("yield=5000")));
            }
            _ => panic!("Expected KnowledgeResult with yield data"),
        }
    }

    #[tokio::test]
    async fn security_flags_suspicious_proposals() {
        use crate::wings::security::SecurityWing;

        let security = SecurityWing::new();

        // SoulcontractAmendment should trigger a critical alert.
        let amendment = Message::new(
            WingId::Evolve,
            WingId::Security,
            Payload::Proposal {
                kind: ProposalKind::SoulcontractAmendment,
                description: "Remove PDA ownership".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.99,
            },
        );
        let resp = security.handle_message(&amendment).unwrap();
        match resp.payload {
            Payload::SecurityAlert { severity, threat } => {
                assert_eq!(severity, RiskLevel::Critical);
                assert!(threat.contains("SoulcontractAmendment"));
            }
            _ => panic!("Expected SecurityAlert for amendment"),
        }

        // Safe strategy change should just get an Ack.
        let safe = Message::new(
            WingId::Trading,
            WingId::Security,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "Update RSI params".to_string(),
                changes: serde_json::json!({"rsi_entry": 28}),
                confidence: 0.9,
            },
        );
        let resp = security.handle_message(&safe).unwrap();
        assert!(matches!(resp.payload, Payload::Ack { .. }));
    }

    #[tokio::test]
    async fn trading_wing_receives_execute_permit() {
        let coord = Coordinator::new(health_config());
        let mut trading_rx = coord.register_wing(WingId::Trading).await;
        let mut audit_rx = coord.register_wing(WingId::Audit).await;

        // Submit proposal → routed to Audit.
        let proposal = Message::new(
            WingId::Trading,
            WingId::Coordinator,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "New strategy".to_string(),
                changes: serde_json::json!({"symbol": "SOL/USDT"}),
                confidence: 0.92,
            },
        );
        coord.process(&proposal).await;
        let audit_msg = audit_rx.recv().await.unwrap();
        let proposal_id = audit_msg.id;

        // Audit approves → ExecutePermit sent to Trading.
        let audit_response = Message::new(
            WingId::Audit,
            WingId::Coordinator,
            Payload::AuditResult {
                proposal_id,
                approved: true,
                risk_level: RiskLevel::Low,
                findings: vec![],
            },
        );
        coord.process(&audit_response).await;
        let permit = trading_rx.recv().await.unwrap();
        assert!(matches!(permit.payload, Payload::ExecutePermit { .. }));
    }
}
