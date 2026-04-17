//! Core types for the RTP swarm runtime.
//!
//! Every wing communicates through typed, JSON-serializable messages.
//! Wings never talk to each other — all messages route through the Coordinator.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for any swarm message.
pub type MessageId = Uuid;

/// Identifies which wing sent or should receive a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WingId {
    Coordinator,
    Trading,
    Security,
    Evolve,
    Knowledge,
    Audit,
    Futureproof,
}

impl std::fmt::Display for WingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WingId::Coordinator => write!(f, "coordinator"),
            WingId::Trading => write!(f, "trading"),
            WingId::Security => write!(f, "security"),
            WingId::Evolve => write!(f, "evolve"),
            WingId::Knowledge => write!(f, "knowledge"),
            WingId::Audit => write!(f, "audit"),
            WingId::Futureproof => write!(f, "futureproof"),
        }
    }
}

/// Typed message envelope. Every inter-wing communication uses this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub from: WingId,
    pub to: WingId,
    pub payload: Payload,
    pub created_at: DateTime<Utc>,
    pub priority: Priority,
}

impl Message {
    pub fn new(from: WingId, to: WingId, payload: Payload) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to,
            payload,
            created_at: Utc::now(),
            priority: Priority::Normal,
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
}

/// Message priority — the Coordinator uses this for routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// All possible message payloads, typed and extensible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Payload {
    // Coordinator-level
    /// Coordinator acknowledges a message was received.
    Ack {
        in_reply_to: MessageId,
    },
    /// Coordinator reports an error routing or processing a message.
    Error {
        reason: String,
        in_reply_to: Option<MessageId>,
    },

    // Proposal lifecycle
    /// A wing submits a proposal for audit before execution.
    Proposal {
        kind: ProposalKind,
        description: String,
        changes: serde_json::Value,
        confidence: f64,
    },
    /// Audit Wing approves or rejects a proposal.
    AuditResult {
        proposal_id: MessageId,
        approved: bool,
        risk_level: RiskLevel,
        findings: Vec<String>,
    },
    /// Coordinator grants execution permission after audit approval.
    ExecutePermit {
        proposal_id: MessageId,
    },

    // Wing health
    /// Heartbeat response from a wing.
    Heartbeat {
        wing: WingId,
        status: HealthStatus,
        metrics: serde_json::Value,
    },
    /// Request a wing to shut down gracefully.
    Shutdown {
        reason: String,
    },

    // Evolve Wing payloads
    /// Evolve Wing proposes an architecture change.
    EvolveProposal {
        target_wing: WingId,
        diff: String,
        rationale: String,
        expected_impact: String,
    },
    /// Evolve Wing requests rollback of a prior change.
    RollbackRequest {
        change_id: MessageId,
        reason: String,
    },
    /// Performance assessment from Evolve Wing.
    Assessment {
        wing: WingId,
        score: f64,
        bottlenecks: Vec<String>,
        recommendations: Vec<String>,
    },

    // Trading Wing payloads
    TradingConfig {
        strategy: String,
        params: serde_json::Value,
    },
    /// Strategy assessment from the Trading Wing.
    ///
    /// `usdc_yield` is a **projected** annual yield from walk-forward analysis
    /// (OOS PnL), not a realized return. The bridge evaluates strategies on
    /// historical data and returns the out-of-sample performance estimate.
    /// `source` indicates the assessment origin (e.g. "wfa_backtest").
    YieldReport {
        usdc_yield: f64,
        sol_reserves: f64,
        drawdown: f64,
        #[serde(default)]
        source: Option<String>,
    },

    // Security Wing payloads
    SecurityAlert {
        severity: RiskLevel,
        threat: String,
    },

    // Knowledge Wing payloads
    KnowledgeQuery {
        query: String,
        context: Option<String>,
    },
    KnowledgeResult {
        results: Vec<String>,
    },

    // Generic typed JSON payload
    Raw(serde_json::Value),
}

/// What category of proposal is being submitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalKind {
    StrategyChange,
    ArchitectureChange,
    ConfigChange,
    RiskThresholdChange,
    NewModule,
    SoulcontractAmendment,
    PhaseTransition,
}

/// Risk level assigned by the Audit Wing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::None => write!(f, "NONE"),
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
            RiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Health status reported by wings in heartbeats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
    Offline,
}

/// A registered wing in the swarm.
#[derive(Debug, Clone)]
pub struct WingRegistration {
    pub id: WingId,
    pub registered_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub status: HealthStatus,
}

/// In-memory audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: MessageId,
    pub timestamp: DateTime<Utc>,
    pub from: WingId,
    pub to: WingId,
    pub payload_summary: String,
    pub soulguard_passed: bool,
    pub rejection_reason: Option<String>,
}

/// Represents a tracked change that can be rolled back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedChange {
    pub id: MessageId,
    pub target_wing: WingId,
    pub diff: String,
    pub baseline_score: f64,
    pub applied_at: DateTime<Utc>,
    pub rolled_back: bool,
    pub rollback_reason: Option<String>,
}
