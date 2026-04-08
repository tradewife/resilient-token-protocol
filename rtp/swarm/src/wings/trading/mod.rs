//! Trading Wing — yield research, validation, and execution.
//!
//! Handles TradingConfig, Proposal, ExecutePermit, YieldReport, and Heartbeat.
//! Uses bridge.rs to call the Python fractal-swarm binary for strategy execution.
//!
//! In-memory state: last proposal, last yield report, execution count.

use crate::bridge::{self, BridgeRequest};
use crate::types::{Message, Payload, WingId};
use std::sync::Mutex;

/// In-memory state for the Trading Wing.
#[derive(Debug)]
struct TradingState {
    last_proposal: Option<serde_json::Value>,
    last_yield_report: Option<serde_json::Value>,
    execution_count: u64,
}

/// The Trading Wing — yield generation and execution.
pub struct TradingWing {
    state: Mutex<TradingState>,
}

impl TradingWing {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(TradingState {
                last_proposal: None,
                last_yield_report: None,
                execution_count: 0,
            }),
        }
    }

    /// Handle an incoming message.
    /// Returns `Some(response)` for every payload type — unhandled types return
    /// `Payload::Error` so dropped messages are visible (I-1 fix).
    pub fn handle_message(&self, msg: &Message) -> Option<Message> {
        match &msg.payload {
            Payload::TradingConfig { strategy, params } => {
                let mut state = self.state.lock().ok()?;
                state.last_proposal = Some(serde_json::json!({
                    "strategy": strategy,
                    "params": params,
                }));
                Some(Message::new(
                    WingId::Trading,
                    WingId::Coordinator,
                    Payload::Ack { in_reply_to: msg.id },
                ))
            }

            Payload::Proposal {
                kind: _,
                description: _,
                changes,
                confidence: _,
            } => {
                // Validate and store the proposal for later execution.
                let mut state = self.state.lock().ok()?;
                state.last_proposal = Some(changes.clone());
                Some(Message::new(
                    WingId::Trading,
                    WingId::Coordinator,
                    Payload::Ack { in_reply_to: msg.id },
                ))
            }

            Payload::ExecutePermit { proposal_id } => {
                // Read proposal config in a single lock scope (avoids TOCTOU).
                let (symbol, config) = {
                    let state = self.state.lock().ok()?;
                    let config = state.last_proposal.as_ref().cloned().unwrap_or(serde_json::json!({}));
                    let symbol = config
                        .get("symbol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("SOL/USDT")
                        .to_string();
                    (symbol, config)
                };

                let request = BridgeRequest::new(&symbol, config);
                match bridge::call_bridge(&request) {
                    Ok(response) => {
                        let mut state = self.state.lock().ok()?;
                        state.execution_count += 1;
                        state.last_yield_report = Some(serde_json::json!({
                            "strategy": response.strategy,
                            "yield_estimate": response.yield_estimate,
                            "confidence": response.confidence,
                        }));
                        Some(Message::new(
                            WingId::Trading,
                            WingId::Coordinator,
                            Payload::YieldReport {
                                usdc_yield: response.yield_estimate,
                                sol_reserves: 0.0,
                                drawdown: 0.0,
                            },
                        ))
                    }
                    Err(e) => Some(Message::new(
                        WingId::Trading,
                        WingId::Coordinator,
                        Payload::Error {
                            reason: format!("Bridge execution failed: {}", e),
                            in_reply_to: Some(*proposal_id),
                        },
                    )),
                }
            }

            Payload::YieldReport {
                usdc_yield,
                sol_reserves,
                drawdown,
            } => {
                let mut state = self.state.lock().ok()?;
                state.last_yield_report = Some(serde_json::json!({
                    "usdc_yield": usdc_yield,
                    "sol_reserves": sol_reserves,
                    "drawdown": drawdown,
                }));
                Some(Message::new(
                    WingId::Trading,
                    WingId::Coordinator,
                    Payload::Ack { in_reply_to: msg.id },
                ))
            }

            Payload::Heartbeat { .. } => {
                let state = self.state.lock().ok()?;
                let metrics = serde_json::json!({
                    "execution_count": state.execution_count,
                    "has_proposal": state.last_proposal.is_some(),
                    "has_yield_report": state.last_yield_report.is_some(),
                });
                Some(Message::new(
                    WingId::Trading,
                    WingId::Coordinator,
                    Payload::Heartbeat {
                        wing: WingId::Trading,
                        status: crate::types::HealthStatus::Healthy,
                        metrics,
                    },
                ))
            }

            _ => Some(Message::new(
                WingId::Trading,
                WingId::Coordinator,
                Payload::Error {
                    reason: format!("Unimplemented payload: {:?}", msg.payload),
                    in_reply_to: Some(msg.id),
                },
            )),
        }
    }

    /// Get the current execution count.
    pub fn execution_count(&self) -> u64 {
        self.state.lock().map(|s| s.execution_count).unwrap_or(0)
    }

    /// Check if the wing has a stored proposal.
    pub fn has_proposal(&self) -> bool {
        self.state
            .lock()
            .map(|s| s.last_proposal.is_some())
            .unwrap_or(false)
    }
}

impl Default for TradingWing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProposalKind;

    #[test]
    fn handles_trading_config() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::TradingConfig {
                strategy: "mr".to_string(),
                params: serde_json::json!({"rsi_entry": 28}),
            },
        );
        assert!(wing.handle_message(&msg).is_some());
        assert!(wing.has_proposal());
    }

    #[test]
    fn handles_proposal() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "Update RSI".to_string(),
                changes: serde_json::json!({"strategy": "mr", "symbol": "SOL/USDT"}),
                confidence: 0.9,
            },
        );
        assert!(wing.handle_message(&msg).is_some());
        assert!(wing.has_proposal());
    }

    #[test]
    fn handles_yield_report() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::YieldReport {
                usdc_yield: 5000.0,
                sol_reserves: 50000.0,
                drawdown: 0.03,
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        assert!(matches!(response.payload, Payload::Ack { .. }));
    }

    #[test]
    fn handles_heartbeat() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Heartbeat {
                wing: WingId::Trading,
                status: crate::types::HealthStatus::Healthy,
                metrics: serde_json::json!({}),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Heartbeat { wing, metrics, .. } => {
                assert_eq!(wing, WingId::Trading);
                assert_eq!(metrics["execution_count"], 0);
            }
            _ => panic!("Expected Heartbeat response"),
        }
    }

    #[test]
    fn execute_permit_bridge_not_found_returns_error() {
        let wing = TradingWing::new();
        // Set a proposal first.
        let proposal = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "test".to_string(),
                changes: serde_json::json!({"symbol": "SOL/USDT"}),
                confidence: 0.9,
            },
        );
        wing.handle_message(&proposal);

        // Execute — binary doesn't exist, expect Error payload.
        let permit = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::ExecutePermit {
                proposal_id: proposal.id,
            },
        );
        let response = wing.handle_message(&permit).unwrap();
        match response.payload {
            Payload::Error { reason, .. } => {
                assert!(reason.contains("Bridge execution failed"));
            }
            _ => panic!("Expected Error payload from failed bridge call"),
        }
    }

    #[test]
    fn unhandled_payload_returns_error() {
        let wing = TradingWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::SecurityAlert {
                severity: crate::types::RiskLevel::Low,
                threat: "test".to_string(),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Error { reason, .. } => assert!(reason.contains("Unimplemented")),
            _ => panic!("Expected Error payload for unhandled type"),
        }
    }

    #[test]
    fn execution_count_zero_when_bridge_fails() {
        let wing = TradingWing::new();
        let proposal = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::Proposal {
                kind: ProposalKind::StrategyChange,
                description: "test".to_string(),
                changes: serde_json::json!({"symbol": "SOL/USDT"}),
                confidence: 0.9,
            },
        );
        wing.handle_message(&proposal);

        let permit = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::ExecutePermit {
                proposal_id: proposal.id,
            },
        );
        wing.handle_message(&permit);
        assert_eq!(wing.execution_count(), 0);
    }
}
