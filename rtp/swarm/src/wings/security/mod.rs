//! Security Wing — threat detection and defense.
//!
//! Monitors message patterns for anomalies, tracks security alerts,
//! rate-limits proposals per wing, and flags suspicious proposals.
//!
//! In-memory alert store with timestamp-based expiry (1 hour).
//! Handles: SecurityAlert, Proposal, Heartbeat.

use crate::types::{Message, Payload, ProposalKind, RiskLevel, WingId};
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Mutex;

/// Max proposals per wing per window before flagging rate-limit.
const RATE_LIMIT_THRESHOLD: u64 = 10;

/// Rate-limit window in seconds.
const RATE_LIMIT_WINDOW_SECS: i64 = 60;

/// Alert expiry in seconds.
const ALERT_EXPIRY_SECS: i64 = 3600;

/// A tracked security alert.
#[derive(Debug, Clone)]
#[allow(dead_code)] // fields used by future alert-query functionality
struct AlertEntry {
    severity: RiskLevel,
    threat: String,
    detected_at: chrono::DateTime<chrono::Utc>,
}

/// Rate-limit counter for a single wing.
#[derive(Debug)]
struct RateEntry {
    count: u64,
    window_start: chrono::DateTime<chrono::Utc>,
}

/// The Security Wing — threat detection and defense.
pub struct SecurityWing {
    alerts: Mutex<Vec<AlertEntry>>,
    rate_limits: Mutex<HashMap<WingId, RateEntry>>,
}

impl SecurityWing {
    pub fn new() -> Self {
        Self {
            alerts: Mutex::new(Vec::new()),
            rate_limits: Mutex::new(HashMap::new()),
        }
    }

    /// Handle an incoming message.
    /// Every payload type returns a response — unhandled types return `Payload::Error`.
    pub fn handle_message(&self, msg: &Message) -> Option<Message> {
        match &msg.payload {
            Payload::SecurityAlert { severity, threat } => {
                let mut alerts = self.alerts.lock().ok()?;
                alerts.push(AlertEntry {
                    severity: *severity,
                    threat: threat.clone(),
                    detected_at: Utc::now(),
                });
                // Prune expired alerts.
                let cutoff = Utc::now() - Duration::seconds(ALERT_EXPIRY_SECS);
                alerts.retain(|a| a.detected_at > cutoff);
                Some(Message::new(
                    WingId::Security,
                    WingId::Coordinator,
                    Payload::Ack {
                        in_reply_to: msg.id,
                    },
                ))
            }

            Payload::Proposal {
                kind,
                description,
                changes: _,
                confidence: _,
            } => {
                // Track rate limit for the sender.
                self.track_rate(&msg.from);

                // Check for suspicious patterns.
                if let Some((severity, threat)) = Self::check_suspicious(kind, description) {
                    // Store detection in alerts for audit trail and heartbeat metrics.
                    if let Ok(mut alerts) = self.alerts.lock() {
                        alerts.push(AlertEntry {
                            severity,
                            threat: threat.clone(),
                            detected_at: Utc::now(),
                        });
                        let cutoff = Utc::now() - Duration::seconds(ALERT_EXPIRY_SECS);
                        alerts.retain(|a| a.detected_at > cutoff);
                    }
                    return Some(Message::new(
                        WingId::Security,
                        WingId::Coordinator,
                        Payload::SecurityAlert { severity, threat },
                    ));
                }

                Some(Message::new(
                    WingId::Security,
                    WingId::Coordinator,
                    Payload::Ack {
                        in_reply_to: msg.id,
                    },
                ))
            }

            Payload::Heartbeat { .. } => {
                let alerts = self.alerts.lock().ok()?;
                let last_alert_ts = alerts.last().map(|a| a.detected_at.to_rfc3339());
                let count = alerts.len();
                let rate_info = {
                    let limits = self.rate_limits.lock().ok()?;
                    limits
                        .iter()
                        .map(|(w, e)| {
                            serde_json::json!({
                                "wing": w.to_string(),
                                "count": e.count,
                            })
                        })
                        .collect::<Vec<_>>()
                };

                let metrics = serde_json::json!({
                    "alert_count": count,
                    "last_alert": last_alert_ts,
                    "rate_limits": rate_info,
                });
                Some(Message::new(
                    WingId::Security,
                    WingId::Coordinator,
                    Payload::Heartbeat {
                        wing: WingId::Security,
                        status: crate::types::HealthStatus::Healthy,
                        metrics,
                    },
                ))
            }

            _ => Some(Message::new(
                WingId::Security,
                WingId::Coordinator,
                Payload::Error {
                    reason: format!("Unimplemented payload: {:?}", msg.payload),
                    in_reply_to: Some(msg.id),
                },
            )),
        }
    }

    /// Record one message from a wing for rate-limiting.
    fn track_rate(&self, wing: &WingId) {
        if let Ok(mut limits) = self.rate_limits.lock() {
            let now = Utc::now();
            let entry = limits.entry(*wing).or_insert(RateEntry {
                count: 0,
                window_start: now,
            });
            if (now - entry.window_start).num_seconds() > RATE_LIMIT_WINDOW_SECS {
                entry.count = 0;
                entry.window_start = now;
            }
            entry.count += 1;
        }
    }

    /// Check if a wing has exceeded the rate-limit threshold.
    pub fn is_rate_limited(&self, wing: &WingId) -> bool {
        self.rate_limits
            .lock()
            .ok()
            .and_then(|l| l.get(wing).map(|e| e.count > RATE_LIMIT_THRESHOLD))
            .unwrap_or(false)
    }

    /// Proposal count for a wing in the current window.
    pub fn proposal_count(&self, wing: &WingId) -> u64 {
        self.rate_limits
            .lock()
            .ok()
            .and_then(|l| l.get(wing).map(|e| e.count))
            .unwrap_or(0)
    }

    /// Check for suspicious proposal patterns. Returns Some((severity, threat))
    /// if the proposal is suspicious, None otherwise.
    fn check_suspicious(kind: &ProposalKind, description: &str) -> Option<(RiskLevel, String)> {
        match kind {
            ProposalKind::SoulcontractAmendment => Some((
                RiskLevel::Critical,
                format!(
                    "Suspicious SoulcontractAmendment proposal detected: '{}'",
                    description
                ),
            )),
            ProposalKind::RiskThresholdChange => Some((
                RiskLevel::High,
                format!(
                    "Suspicious RiskThresholdChange proposal detected: '{}'",
                    description
                ),
            )),
            ProposalKind::PhaseTransition => Some((
                RiskLevel::Medium,
                format!(
                    "PhaseTransition proposal flagged for review: '{}'",
                    description
                ),
            )),
            _ => None,
        }
    }

    /// Number of active (non-expired) alerts.
    pub fn alert_count(&self) -> usize {
        self.alerts.lock().map(|a| a.len()).unwrap_or(0)
    }
}

impl Default for SecurityWing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proposal(kind: ProposalKind) -> Message {
        Message::new(
            WingId::Trading,
            WingId::Security,
            Payload::Proposal {
                kind,
                description: "test proposal".to_string(),
                changes: serde_json::json!({}),
                confidence: 0.9,
            },
        )
    }

    #[test]
    fn handles_heartbeat() {
        let wing = SecurityWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Security,
            Payload::Heartbeat {
                wing: WingId::Security,
                status: crate::types::HealthStatus::Healthy,
                metrics: serde_json::json!({}),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Heartbeat { wing, .. } => assert_eq!(wing, WingId::Security),
            _ => panic!("Expected Heartbeat"),
        }
    }

    #[test]
    fn tracks_security_alert() {
        let wing = SecurityWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Security,
            Payload::SecurityAlert {
                severity: RiskLevel::Medium,
                threat: "Anomaly detected".to_string(),
            },
        );
        wing.handle_message(&msg);
        assert_eq!(wing.alert_count(), 1);
    }

    #[test]
    fn flags_soulcontract_amendment_as_critical() {
        let wing = SecurityWing::new();
        let msg = make_proposal(ProposalKind::SoulcontractAmendment);
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::SecurityAlert { severity, threat } => {
                assert_eq!(severity, RiskLevel::Critical);
                assert!(threat.contains("SoulcontractAmendment"));
            }
            _ => panic!("Expected SecurityAlert"),
        }
    }

    #[test]
    fn flags_risk_threshold_change_as_high() {
        let wing = SecurityWing::new();
        let msg = make_proposal(ProposalKind::RiskThresholdChange);
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::SecurityAlert { severity, threat } => {
                assert_eq!(severity, RiskLevel::High);
                assert!(threat.contains("RiskThresholdChange"));
            }
            _ => panic!("Expected SecurityAlert"),
        }
    }

    #[test]
    fn flags_phase_transition_as_medium() {
        let wing = SecurityWing::new();
        let msg = make_proposal(ProposalKind::PhaseTransition);
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::SecurityAlert { severity, threat } => {
                assert_eq!(severity, RiskLevel::Medium);
                assert!(threat.contains("PhaseTransition"));
            }
            _ => panic!("Expected SecurityAlert"),
        }
    }

    #[test]
    fn safe_proposal_returns_ack() {
        let wing = SecurityWing::new();
        let msg = make_proposal(ProposalKind::StrategyChange);
        let response = wing.handle_message(&msg).unwrap();
        assert!(matches!(response.payload, Payload::Ack { .. }));
        assert_eq!(wing.proposal_count(&WingId::Trading), 1);
    }

    #[test]
    fn rate_limit_triggers_after_threshold() {
        let wing = SecurityWing::new();
        for _ in 0..15 {
            wing.handle_message(&make_proposal(ProposalKind::StrategyChange));
        }
        assert_eq!(wing.proposal_count(&WingId::Trading), 15);
        assert!(wing.is_rate_limited(&WingId::Trading));
        assert!(!wing.is_rate_limited(&WingId::Evolve));
    }

    #[test]
    fn heartbeat_reports_alert_count() {
        let wing = SecurityWing::new();
        wing.handle_message(&Message::new(
            WingId::Coordinator,
            WingId::Security,
            Payload::SecurityAlert {
                severity: RiskLevel::Low,
                threat: "test".to_string(),
            },
        ));
        let hb = wing
            .handle_message(&Message::new(
                WingId::Coordinator,
                WingId::Security,
                Payload::Heartbeat {
                    wing: WingId::Security,
                    status: crate::types::HealthStatus::Healthy,
                    metrics: serde_json::json!({}),
                },
            ))
            .unwrap();
        match hb.payload {
            Payload::Heartbeat { metrics, .. } => assert_eq!(metrics["alert_count"], 1),
            _ => panic!("Expected Heartbeat"),
        }
    }

    #[test]
    fn unhandled_payload_returns_error() {
        let wing = SecurityWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Security,
            Payload::YieldReport {
                usdc_yield: 100.0,
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
