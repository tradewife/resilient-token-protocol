//! Router — typed message routing between wings with topology and fault tolerance.
//! Hub topology (default): Central Coordinator routes all messages.
//!   - Hierarchical: Queen (Coordinator) delegates to sub-coordinators
//!
//! Fault tolerance:
//!   - Exponential backoff retry on delivery failure
//!   - Configurable max attempts and base delay

use crate::types::{Message, MessageId, Payload, WingId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, sleep};

/// Routing topology for the swarm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Topology {
    /// Central Coordinator routes all messages (RTP default).
    #[default]
    Hub,
    /// Wings can discover peers and request mediated peer messages.
    Mesh,
    /// Coordinator delegates to sub-coordinators for wing groups.
    Hierarchical,
}

/// Fault tolerance configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum delivery attempts before giving up.
    pub max_attempts: u32,
    /// Base delay for exponential backoff (milliseconds).
    pub base_delay_ms: u64,
    /// Maximum backoff delay (milliseconds).
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 50,
            max_delay_ms: 2000,
        }
    }
}

/// Channel for delivering messages to a wing.
type WingTx = mpsc::Sender<Message>;

/// Outcome of routing a single message.
#[derive(Debug, Clone)]
pub enum RoutingOutcome {
    Delivered { to: WingId, msg_id: MessageId },
    AwaitingAudit { msg_id: MessageId },
    Rejected { msg_id: MessageId, reason: String },
    Broadcast { msg_id: MessageId, count: usize },
}

/// A pending proposal awaiting audit approval.
#[derive(Debug, Clone)]
pub struct PendingProposal {
    pub message: Message,
    pub proposed_at: chrono::DateTime<chrono::Utc>,
}

/// The router — heart of the Coordinator's message bus.
pub struct Router {
    wing_channels: Arc<RwLock<HashMap<WingId, WingTx>>>,
    pending_proposals: Arc<RwLock<HashMap<MessageId, PendingProposal>>>,
    approved_proposals: Arc<RwLock<HashMap<MessageId, Message>>>,
    channel_capacity: usize,
    topology: Topology,
    retry_policy: RetryPolicy,
}

impl Router {
    pub fn new(channel_capacity: usize) -> Self {
        Self {
            wing_channels: Arc::new(RwLock::new(HashMap::new())),
            pending_proposals: Arc::new(RwLock::new(HashMap::new())),
            approved_proposals: Arc::new(RwLock::new(HashMap::new())),
            channel_capacity,
            topology: Topology::default(),
            retry_policy: RetryPolicy::default(),
        }
    }

    pub fn with_topology(mut self, topology: Topology) -> Self {
        self.topology = topology;
        self
    }

    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    pub fn topology(&self) -> Topology {
        self.topology
    }

    pub async fn register_wing(&self, wing_id: WingId) -> mpsc::Receiver<Message> {
        let (tx, rx) = mpsc::channel(self.channel_capacity);
        let mut channels = self.wing_channels.write().await;
        channels.insert(wing_id, tx);
        rx
    }

    pub async fn unregister_wing(&self, wing_id: &WingId) {
        let mut channels = self.wing_channels.write().await;
        channels.remove(wing_id);
    }

    pub async fn registered_wings(&self) -> Vec<WingId> {
        let channels = self.wing_channels.read().await;
        channels.keys().copied().collect()
    }

    /// Route a message with fault tolerance (exponential backoff retry).
    pub async fn route(&self, message: Message) -> RoutingOutcome {
        match &message.payload {
            Payload::Proposal { .. } => {
                let proposal_id = message.id;
                let pending = PendingProposal {
                    message: message.clone(),
                    proposed_at: chrono::Utc::now(),
                };
                self.pending_proposals
                    .write()
                    .await
                    .insert(proposal_id, pending);

                match self
                    .deliver_with_retry(WingId::Audit, message.clone())
                    .await
                {
                    true => RoutingOutcome::AwaitingAudit {
                        msg_id: proposal_id,
                    },
                    false => RoutingOutcome::Rejected {
                        msg_id: proposal_id,
                        reason: "Audit Wing not registered after retries".to_string(),
                    },
                }
            }

            Payload::AuditResult {
                proposal_id,
                approved,
                findings,
                ..
            } => {
                if *approved {
                    if let Some(pending) = self.pending_proposals.write().await.remove(proposal_id)
                    {
                        let proposer = pending.message.from;
                        let execute_permit = Message::new(
                            WingId::Coordinator,
                            proposer,
                            Payload::ExecutePermit {
                                proposal_id: *proposal_id,
                            },
                        )
                        .with_priority(crate::types::Priority::High);

                        self.approved_proposals
                            .write()
                            .await
                            .insert(*proposal_id, pending.message);

                        match self.deliver_with_retry(proposer, execute_permit).await {
                            true => RoutingOutcome::Delivered {
                                to: proposer,
                                msg_id: *proposal_id,
                            },
                            false => RoutingOutcome::Rejected {
                                msg_id: *proposal_id,
                                reason: format!(
                                    "Cannot deliver execution permit to {}. Wing not registered after retries. Findings: {:?}",
                                    proposer, findings
                                ),
                            },
                        }
                    } else {
                        RoutingOutcome::Rejected {
                            msg_id: *proposal_id,
                            reason: "No pending proposal found for this audit result.".to_string(),
                        }
                    }
                } else {
                    self.pending_proposals.write().await.remove(proposal_id);
                    RoutingOutcome::Rejected {
                        msg_id: *proposal_id,
                        reason: format!("Audit rejected: {:?}", findings),
                    }
                }
            }

            Payload::EvolveProposal { .. } => {
                let proposal_id = message.id;
                let pending = PendingProposal {
                    message: message.clone(),
                    proposed_at: chrono::Utc::now(),
                };
                self.pending_proposals
                    .write()
                    .await
                    .insert(proposal_id, pending);

                match self
                    .deliver_with_retry(WingId::Audit, message.clone())
                    .await
                {
                    true => RoutingOutcome::AwaitingAudit {
                        msg_id: proposal_id,
                    },
                    false => RoutingOutcome::Rejected {
                        msg_id: proposal_id,
                        reason: "Audit Wing not registered for evolve proposal review".to_string(),
                    },
                }
            }

            Payload::RollbackRequest { .. } => {
                match self
                    .deliver_with_retry(WingId::Evolve, message.clone())
                    .await
                {
                    true => RoutingOutcome::Delivered {
                        to: WingId::Evolve,
                        msg_id: message.id,
                    },
                    false => RoutingOutcome::Rejected {
                        msg_id: message.id,
                        reason: "Evolve Wing not registered after retries".to_string(),
                    },
                }
            }

            Payload::Shutdown { .. } => {
                let count = self.broadcast(message.clone()).await;
                RoutingOutcome::Broadcast {
                    msg_id: message.id,
                    count,
                }
            }

            _ => {
                let target = message.to;
                match self.deliver_with_retry(target, message.clone()).await {
                    true => RoutingOutcome::Delivered {
                        to: target,
                        msg_id: message.id,
                    },
                    false => RoutingOutcome::Rejected {
                        msg_id: message.id,
                        reason: format!("Wing {} not registered after retries", target),
                    },
                }
            }
        }
    }

    /// Deliver with exponential backoff retry.
    async fn deliver_with_retry(&self, wing_id: WingId, message: Message) -> bool {
        let mut delay = self.retry_policy.base_delay_ms;
        for attempt in 0..self.retry_policy.max_attempts {
            if self.deliver_to_wing(wing_id, &message).await {
                return true;
            }
            if attempt < self.retry_policy.max_attempts - 1 {
                sleep(Duration::from_millis(delay)).await;
                delay = (delay * 2).min(self.retry_policy.max_delay_ms);
            }
        }
        false
    }

    /// Single fire-and-forget delivery attempt.
    async fn deliver_to_wing(&self, wing_id: WingId, message: &Message) -> bool {
        let channels = self.wing_channels.read().await;
        if let Some(tx) = channels.get(&wing_id) {
            tx.send(message.clone()).await.is_ok()
        } else {
            false
        }
    }

    async fn broadcast(&self, message: Message) -> usize {
        let channels = self.wing_channels.read().await;
        let mut delivered = 0;
        for (_, tx) in channels.iter() {
            if tx.send(message.clone()).await.is_ok() {
                delivered += 1;
            }
        }
        delivered
    }

    pub async fn pending_count(&self) -> usize {
        self.pending_proposals.read().await.len()
    }

    pub async fn approved_count(&self) -> usize {
        self.approved_proposals.read().await.len()
    }

    pub async fn is_pending(&self, proposal_id: &MessageId) -> bool {
        self.pending_proposals
            .read()
            .await
            .contains_key(proposal_id)
    }

    pub async fn take_approved(&self, proposal_id: &MessageId) -> Option<Message> {
        self.approved_proposals.write().await.remove(proposal_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ProposalKind, RiskLevel};

    #[tokio::test]
    async fn deliver_to_registered_wing() {
        let router = Router::new(32);
        let mut rx = router.register_wing(WingId::Trading).await;

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Ack {
                in_reply_to: uuid::Uuid::new_v4(),
            },
        );
        let outcome = router.route(msg.clone()).await;
        assert!(matches!(outcome, RoutingOutcome::Delivered { .. }));
        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, msg.id);
    }

    #[tokio::test]
    async fn unregistered_wing_rejected() {
        let router = Router::new(32);
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Ack {
                in_reply_to: uuid::Uuid::new_v4(),
            },
        );
        let outcome = router.route(msg).await;
        assert!(matches!(outcome, RoutingOutcome::Rejected { .. }));
    }

    #[tokio::test]
    async fn proposal_to_audit() {
        let router = Router::new(32);
        let mut audit_rx = router.register_wing(WingId::Audit).await;
        router.register_wing(WingId::Trading).await;

        let msg = Message::new(
            WingId::Trading,
            WingId::Coordinator,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "test".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.85,
            },
        );
        let outcome = router.route(msg.clone()).await;
        assert!(matches!(outcome, RoutingOutcome::AwaitingAudit { .. }));
        assert!(router.is_pending(&msg.id).await);

        let received = audit_rx.recv().await.unwrap();
        assert!(matches!(received.payload, Payload::Proposal { .. }));
    }

    #[tokio::test]
    async fn audit_approval_to_execution() {
        let router = Router::new(32);
        let mut audit_rx = router.register_wing(WingId::Audit).await;
        let mut trading_rx = router.register_wing(WingId::Trading).await;

        let proposal = Message::new(
            WingId::Trading,
            WingId::Coordinator,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "test".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.85,
            },
        );
        router.route(proposal.clone()).await;
        let _ = audit_rx.recv().await;

        let audit_response = Message::new(
            WingId::Audit,
            WingId::Coordinator,
            Payload::AuditResult {
                proposal_id: proposal.id,
                approved: true,
                risk_level: RiskLevel::Low,
                findings: vec![],
            },
        );
        let outcome = router.route(audit_response).await;
        assert!(matches!(outcome, RoutingOutcome::Delivered { .. }));

        let permit = trading_rx.recv().await.unwrap();
        assert!(matches!(permit.payload, Payload::ExecutePermit { .. }));
    }

    #[tokio::test]
    async fn audit_rejection_clears_pending() {
        let router = Router::new(32);
        router.register_wing(WingId::Audit).await;

        let proposal = Message::new(
            WingId::Trading,
            WingId::Coordinator,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "test".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.3,
            },
        );
        router.route(proposal.clone()).await;

        let audit_response = Message::new(
            WingId::Audit,
            WingId::Coordinator,
            Payload::AuditResult {
                proposal_id: proposal.id,
                approved: false,
                risk_level: RiskLevel::High,
                findings: vec!["Bad".to_string()],
            },
        );
        let outcome = router.route(audit_response).await;
        assert!(matches!(outcome, RoutingOutcome::Rejected { .. }));
        assert!(!router.is_pending(&proposal.id).await);
    }

    #[tokio::test]
    async fn shutdown_broadcasts() {
        let router = Router::new(32);
        let mut t_rx = router.register_wing(WingId::Trading).await;
        let mut s_rx = router.register_wing(WingId::Security).await;
        let mut e_rx = router.register_wing(WingId::Evolve).await;

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Shutdown {
                reason: "test".to_string(),
            },
        )
        .with_priority(crate::types::Priority::Critical);

        let outcome = router.route(msg).await;
        if let RoutingOutcome::Broadcast { count, .. } = outcome {
            assert_eq!(count, 3);
        } else {
            panic!("Expected broadcast");
        }

        assert!(t_rx.recv().await.is_some());
        assert!(s_rx.recv().await.is_some());
        assert!(e_rx.recv().await.is_some());
    }

    #[tokio::test]
    async fn evolve_proposal_to_audit() {
        let router = Router::new(32);
        let mut audit_rx = router.register_wing(WingId::Audit).await;

        let msg = Message::new(
            WingId::Evolve,
            WingId::Coordinator,
            Payload::EvolveProposal {
                target_wing: WingId::Trading,
                diff: "test".to_string(),
                rationale: "test".to_string(),
                expected_impact: "test".to_string(),
            },
        );
        let outcome = router.route(msg).await;
        assert!(matches!(outcome, RoutingOutcome::AwaitingAudit { .. }));

        let received = audit_rx.recv().await.unwrap();
        assert!(matches!(received.payload, Payload::EvolveProposal { .. }));
    }

    #[tokio::test]
    async fn rollback_to_evolve() {
        let router = Router::new(32);
        let mut rx = router.register_wing(WingId::Evolve).await;

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Evolve,
            Payload::RollbackRequest {
                change_id: uuid::Uuid::new_v4(),
                reason: "test".to_string(),
            },
        );
        let outcome = router.route(msg.clone()).await;
        assert!(matches!(outcome, RoutingOutcome::Delivered { .. }));

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, msg.id);
    }

    #[test]
    fn topology_default_is_hub() {
        let router = Router::new(32);
        assert_eq!(router.topology(), Topology::Hub);
    }

    #[test]
    fn topology_configurable() {
        let router = Router::new(32).with_topology(Topology::Mesh);
        assert_eq!(router.topology(), Topology::Mesh);
    }
}
