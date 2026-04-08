//! Future-proof Wing — quantum, deprecation, and horizon scanning.
//!
//! Monitors existential and technological risks on the horizon.
//! Week 3: heartbeat with deprecation checks + shutdown ack.
//! Handles: Heartbeat, Shutdown.

use crate::types::{Message, Payload, WingId};

/// Known crate versions to monitor for deprecation.
const CRATE_VERSIONS: &[(&str, &str)] = &[
    ("anchor-lang", "1.0.0"),
    ("spl-token-2022-interface", "2.1.0"),
    ("tokio", "1.x"),
    ("serde", "1.x"),
    ("dashmap", "6.x"),
    ("chrono", "0.4"),
    ("uuid", "1.x"),
];

/// The Future-proof Wing — horizon scanning and deprecation monitoring.
pub struct FutureproofWing;

impl FutureproofWing {
    pub fn new() -> Self {
        Self
    }

    /// Handle an incoming message.
    /// Handles Heartbeat (with deprecation checks) and Shutdown.
    /// Unhandled payloads return `Payload::Error`.
    pub fn handle_message(&self, msg: &Message) -> Option<Message> {
        match &msg.payload {
            Payload::Heartbeat { .. } => {
                let checks = self.deprecation_checks();
                let metrics = serde_json::json!({
                    "deprecation_checks": checks,
                    "status": "horizon_scanning",
                });
                Some(Message::new(
                    WingId::Futureproof,
                    WingId::Coordinator,
                    Payload::Heartbeat {
                        wing: WingId::Futureproof,
                        status: crate::types::HealthStatus::Healthy,
                        metrics,
                    },
                ))
            }

            Payload::Shutdown { .. } => Some(Message::new(
                WingId::Futureproof,
                WingId::Coordinator,
                Payload::Ack { in_reply_to: msg.id },
            )),

            _ => Some(Message::new(
                WingId::Futureproof,
                WingId::Coordinator,
                Payload::Error {
                    reason: format!("Unimplemented payload: {:?}", msg.payload),
                    in_reply_to: Some(msg.id),
                },
            )),
        }
    }

    /// Build deprecation check results for monitored crates.
    fn deprecation_checks(&self) -> Vec<serde_json::Value> {
        CRATE_VERSIONS
            .iter()
            .map(|(name, version)| {
                serde_json::json!({
                    "crate": name,
                    "version": version,
                    "status": "monitored",
                })
            })
            .collect()
    }

    /// List all monitored crate names and versions.
    pub fn monitored_crates(&self) -> Vec<(&str, &str)> {
        CRATE_VERSIONS.to_vec()
    }
}

impl Default for FutureproofWing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_heartbeat() {
        let wing = FutureproofWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Futureproof,
            Payload::Heartbeat {
                wing: WingId::Futureproof,
                status: crate::types::HealthStatus::Healthy,
                metrics: serde_json::json!({}),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Heartbeat { wing, metrics, .. } => {
                assert_eq!(wing, WingId::Futureproof);
                assert!(metrics["deprecation_checks"].is_array());
            }
            _ => panic!("Expected Heartbeat"),
        }
    }

    #[test]
    fn handles_shutdown() {
        let wing = FutureproofWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Futureproof,
            Payload::Shutdown {
                reason: "maintenance".to_string(),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        assert!(matches!(response.payload, Payload::Ack { .. }));
    }

    #[test]
    fn heartbeat_includes_deprecation_data() {
        let wing = FutureproofWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Futureproof,
            Payload::Heartbeat {
                wing: WingId::Futureproof,
                status: crate::types::HealthStatus::Healthy,
                metrics: serde_json::json!({}),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Heartbeat { metrics, .. } => {
                let checks = metrics["deprecation_checks"].as_array().unwrap();
                assert!(!checks.is_empty());
                let first = &checks[0];
                assert!(first["crate"].is_string());
                assert_eq!(first["status"], "monitored");
            }
            _ => panic!("Expected Heartbeat"),
        }
    }

    #[test]
    fn monitored_crates_includes_anchor() {
        let wing = FutureproofWing::new();
        let crates = wing.monitored_crates();
        assert!(crates.iter().any(|(name, _)| *name == "anchor-lang"));
        assert!(!crates.is_empty());
    }

    #[test]
    fn unhandled_payload_returns_error() {
        let wing = FutureproofWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Futureproof,
            Payload::YieldReport {
                usdc_yield: 0.0,
                sol_reserves: 0.0,
                drawdown: 0.0,
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Error { reason, .. } => assert!(reason.contains("Unimplemented")),
            _ => panic!("Expected Error payload"),
        }
    }
}
