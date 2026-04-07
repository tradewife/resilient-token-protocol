//! # RTP Swarm Runtime
//!
//! Resilient Token Protocol — a Solana-native, self-funding treasury
//! governed by a modular Rust swarm.
//!
//! Any token project adopts RTP by enabling TransferFeeConfig on their mint.
//! Trading fees auto-route to a PDA-owned treasury. A Rust swarm researches,
//! validates, and executes yield strategies — returning yield back to the
//! project and its holders.
//!
//! ## Architecture
//!
//! ```text
//! Wing -> Coordinator (soulguard check) -> Router -> Wing
//! ```
//!
//! Six wings, independently testable, communicating through the Coordinator:
//! - Trading (yield gen)
//! - Security (defense)
//! - Evolve (self-modification)
//! - Knowledge (memory)
//! - Audit (compliance)
//! - Futureproof (horizon scanning)
//!
//! ## Key Invariants
//!
//! - Wings NEVER modify each other directly — all via Coordinator
//! - Every message passes through soulguard (soulcontract enforcement)
//! - Python <-> Rust interface is typed JSON
//! - soulcontract.md core values cannot be amended without human signature

pub mod coordinator;
pub mod types;
pub mod wings;

// Re-exports for convenience.
pub use coordinator::{Coordinator, ProcessingResult};
pub use coordinator::lifecycle::HealthConfig as LifecycleHealthConfig;
pub use coordinator::soulcontract_spec::{DriftReport, SoulcontractSpec};
pub use types::{
    AuditLogEntry, HealthStatus, Message, MessageId, Payload, Priority, ProposalKind, RiskLevel,
    TrackedChange, WingId,
};
pub use wings::evolve::assessor::{Assessment, PerformanceMetrics};
pub use wings::evolve::proposer::ChangeProposal;
pub use wings::evolve::EvolveWing;
