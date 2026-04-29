//! # RTP Swarm Runtime
//!
//! Solana-native self-funding treasury governed by a modular Rust swarm.
//! Six wings: Trading, Security, Evolve, Knowledge, Audit, Futureproof.
//! All cross-wing communication via Coordinator through soulguard.

pub mod bridge;
pub mod chain_client;
pub mod config;
pub mod coordinator;
pub mod demo;
pub mod evaluator;
pub mod heartbeat;
pub mod memory_promotion;
pub mod orchestrator;
pub mod types;
pub mod wings;

// Re-exports for convenience.
pub use coordinator::lifecycle::HealthConfig as LifecycleHealthConfig;
pub use coordinator::soulcontract_spec::{DriftReport, SoulcontractSpec};
pub use coordinator::{Coordinator, ProcessingResult};
pub use evaluator::{BridgeMetrics, Evaluation, Evaluator, HealthCheck, OnChainState, compute_tsi};
pub use heartbeat::{
    HeartbeatConfig, HeartbeatEngine, HeartbeatSignal, HeartbeatType, RecommendedAction,
};
pub use memory_promotion::{
    CoreMemory, MemoryConfig, MemoryPromotion, OverviewMemory, ProjectMemory, PromotionResult,
    RedirectEvent, RedirectTrigger, WorkingMemory,
};
pub use orchestrator::{
    BridgeFetcher, CycleResult, Hooks, MockBridgeFetcher, MockTreasuryFetcher, Orchestrator,
    OrchestratorConfig, OrchestratorStatus, TreasuryFetcher,
};
pub use types::{
    AuditLogEntry, HealthStatus, Message, MessageId, Payload, Priority, ProposalKind, RiskLevel,
    TrackedChange, WingId,
};
pub use wings::evolve::EvolveWing;
pub use wings::evolve::assessor::{Assessment, PerformanceMetrics};
pub use wings::evolve::proposer::ChangeProposal;
