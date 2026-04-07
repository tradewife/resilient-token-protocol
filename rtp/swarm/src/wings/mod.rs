//! Wings — specialized agents of the RTP swarm.
//!
//! Six wings, each independently testable, communicating only through
//! the Coordinator:
//!
//! - **Trading**: yield research, validation, execution (Week 4)
//! - **Security**: threat detection, defense (Week 3)
//! - **Evolve**: self-modification, adaptation (Week 2)
//! - **Knowledge**: realtime knowledge graph (Week 3)
//! - **Audit**: soulcontract enforcement, compliance (Week 3)
//! - **Futureproof**: quantum, deprecation monitoring (Week 5)

pub mod audit;
pub mod evolve;
pub mod futureproof;
pub mod knowledge;
pub mod security;
pub mod trading;
