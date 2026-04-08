//! Knowledge Wing — in-memory knowledge graph and cross-wing recall.
//!
//! Stores strategy results, wing metrics, and decisions in a HashMap.
//! Any wing can query the knowledge store via `KnowledgeQuery`.
//!
//! Handles: KnowledgeQuery, YieldReport, Assessment, Heartbeat.

use crate::types::{Message, Payload, WingId};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// A single knowledge entry.
#[derive(Debug, Clone)]
struct KnowledgeEntry {
    values: Vec<String>,
    last_updated: chrono::DateTime<chrono::Utc>,
}

/// The Knowledge Wing — realtime knowledge store.
pub struct KnowledgeWing {
    /// Key → list of values (append-only log per key).
    store: Mutex<HashMap<String, KnowledgeEntry>>,
    query_count: AtomicU64,
}

impl KnowledgeWing {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
            query_count: AtomicU64::new(0),
        }
    }

    /// Handle an incoming message.
    /// Every payload type returns a response — unhandled types return `Payload::Error`.
    pub fn handle_message(&self, msg: &Message) -> Option<Message> {
        match &msg.payload {
            Payload::KnowledgeQuery { query, context } => {
                self.query_count.fetch_add(1, Ordering::Relaxed);
                let store = self.store.lock().ok()?;
                let results = Self::search(&store, query, context.as_deref());
                Some(Message::new(
                    WingId::Knowledge,
                    WingId::Coordinator,
                    Payload::KnowledgeResult { results },
                ))
            }

            Payload::YieldReport {
                usdc_yield,
                sol_reserves,
                drawdown,
            } => {
                let mut store = self.store.lock().ok()?;
                let entry = store.entry("yield_reports".to_string()).or_insert_with(|| {
                    KnowledgeEntry {
                        values: Vec::new(),
                        last_updated: Utc::now(),
                    }
                });
                entry.values.push(format!(
                    "yield={} sol={} dd={} at={}",
                    usdc_yield,
                    sol_reserves,
                    drawdown,
                    Utc::now().to_rfc3339()
                ));
                entry.last_updated = Utc::now();
                Some(Message::new(
                    WingId::Knowledge,
                    WingId::Coordinator,
                    Payload::Ack { in_reply_to: msg.id },
                ))
            }

            Payload::Assessment {
                wing,
                score,
                bottlenecks,
                recommendations,
            } => {
                let mut store = self.store.lock().ok()?;
                let key = format!("assessment:{}", wing);
                let entry = store.entry(key).or_insert_with(|| KnowledgeEntry {
                    values: Vec::new(),
                    last_updated: Utc::now(),
                });
                entry.values.push(format!(
                    "score={} bottlenecks=[{}] recs=[{}] at={}",
                    score,
                    bottlenecks.join(", "),
                    recommendations.join(", "),
                    Utc::now().to_rfc3339()
                ));
                entry.last_updated = Utc::now();
                Some(Message::new(
                    WingId::Knowledge,
                    WingId::Coordinator,
                    Payload::Ack { in_reply_to: msg.id },
                ))
            }

            Payload::Heartbeat { .. } => {
                let store = self.store.lock().ok()?;
                let metrics = serde_json::json!({
                    "store_size": store.len(),
                    "query_count": self.query_count.load(Ordering::Relaxed),
                });
                Some(Message::new(
                    WingId::Knowledge,
                    WingId::Coordinator,
                    Payload::Heartbeat {
                        wing: WingId::Knowledge,
                        status: crate::types::HealthStatus::Healthy,
                        metrics,
                    },
                ))
            }

            _ => Some(Message::new(
                WingId::Knowledge,
                WingId::Coordinator,
                Payload::Error {
                    reason: format!("Unimplemented payload: {:?}", msg.payload),
                    in_reply_to: Some(msg.id),
                },
            )),
        }
    }

    /// Search the store by key substring or content substring.
    fn search(
        store: &HashMap<String, KnowledgeEntry>,
        query: &str,
        context: Option<&str>,
    ) -> Vec<String> {
        let q = query.to_lowercase();
        let mut results = Vec::new();

        for (key, entry) in store.iter() {
            let key_lower = key.to_lowercase();
            let key_match = key_lower.contains(&q) || q.contains(&key_lower);
            let content_match = entry.values.iter().any(|v| v.to_lowercase().contains(&q));
            let ctx_match = context.is_some_and(|ctx| {
                entry
                    .values
                    .iter()
                    .any(|v| v.to_lowercase().contains(&ctx.to_lowercase()))
            });

            if key_match || content_match || ctx_match {
                for value in &entry.values {
                    results.push(format!("[{}] {}", key, value));
                }
            }
        }

        if results.is_empty() {
            results.push(format!("No results for: {}", query));
        }
        results
    }

    /// Store a value under a key (for testing and programmatic access).
    pub fn put(&self, key: &str, value: &str) {
        if let Ok(mut store) = self.store.lock() {
            let entry = store.entry(key.to_string()).or_insert_with(|| KnowledgeEntry {
                values: Vec::new(),
                last_updated: Utc::now(),
            });
            entry.values.push(value.to_string());
            entry.last_updated = Utc::now();
        }
    }

    /// Number of distinct keys in the store.
    pub fn store_size(&self) -> usize {
        self.store.lock().map(|s| s.len()).unwrap_or(0)
    }

    /// Total queries processed.
    pub fn query_count(&self) -> u64 {
        self.query_count.load(Ordering::Relaxed)
    }
}

impl Default for KnowledgeWing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_knowledge_query_empty() {
        let wing = KnowledgeWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::KnowledgeQuery {
                query: "yield".to_string(),
                context: None,
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::KnowledgeResult { results } => {
                assert!(results[0].contains("No results"));
            }
            _ => panic!("Expected KnowledgeResult"),
        }
        assert_eq!(wing.query_count(), 1);
    }

    #[test]
    fn stores_yield_report() {
        let wing = KnowledgeWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::YieldReport {
                usdc_yield: 5000.0,
                sol_reserves: 50000.0,
                drawdown: 0.03,
            },
        );
        wing.handle_message(&msg);
        assert_eq!(wing.store_size(), 1);
    }

    #[test]
    fn query_returns_stored_yield_data() {
        let wing = KnowledgeWing::new();
        wing.put("yield_reports", "yield=5000 sol=50000 dd=0.03");
        wing.put("yield_reports", "yield=3000 sol=40000 dd=0.05");

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::KnowledgeQuery {
                query: "yield".to_string(),
                context: None,
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::KnowledgeResult { results } => assert!(results.len() >= 2),
            _ => panic!("Expected KnowledgeResult"),
        }
    }

    #[test]
    fn stores_assessment() {
        let wing = KnowledgeWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::Assessment {
                wing: WingId::Trading,
                score: 0.85,
                bottlenecks: vec!["slow entry".to_string()],
                recommendations: vec!["optimize RSI".to_string()],
            },
        );
        wing.handle_message(&msg);
        assert_eq!(wing.store_size(), 1);
    }

    #[test]
    fn query_returns_stored_assessment() {
        let wing = KnowledgeWing::new();
        wing.put("assessment:Trading", "score=0.85 bottlenecks=[slow]");

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::KnowledgeQuery {
                query: "assessment:Trading".to_string(),
                context: None,
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::KnowledgeResult { results } => {
                assert!(results.iter().any(|r| r.contains("score=0.85")));
            }
            _ => panic!("Expected KnowledgeResult"),
        }
    }

    #[test]
    fn heartbeat_reports_store_metrics() {
        let wing = KnowledgeWing::new();
        wing.put("test", "data");

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::Heartbeat {
                wing: WingId::Knowledge,
                status: crate::types::HealthStatus::Healthy,
                metrics: serde_json::json!({}),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Heartbeat { wing, metrics, .. } => {
                assert_eq!(wing, WingId::Knowledge);
                assert_eq!(metrics["store_size"], 1);
                assert_eq!(metrics["query_count"], 0);
            }
            _ => panic!("Expected Heartbeat"),
        }
    }

    #[test]
    fn query_with_context_filters_results() {
        let wing = KnowledgeWing::new();
        wing.put("yield_reports", "yield=5000 for SOL/USDT");
        wing.put("yield_reports", "yield=3000 for BTC/USDT");

        let msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::KnowledgeQuery {
                query: "yield".to_string(),
                context: Some("SOL".to_string()),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::KnowledgeResult { results } => {
                assert!(results.iter().any(|r| r.contains("SOL")));
            }
            _ => panic!("Expected KnowledgeResult"),
        }
    }

    #[test]
    fn unhandled_payload_returns_error() {
        let wing = KnowledgeWing::new();
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Knowledge,
            Payload::Shutdown { reason: "test".to_string() },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Error { reason, .. } => assert!(reason.contains("Unimplemented")),
            _ => panic!("Expected Error payload"),
        }
    }
}
