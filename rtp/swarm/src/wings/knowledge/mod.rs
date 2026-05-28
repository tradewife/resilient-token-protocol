//! Knowledge Wing — persistent knowledge store and cross-wing recall.
//!
//! Stores strategy results, wing metrics, and decisions in a HashMap.
//! Any wing can query the knowledge store via `KnowledgeQuery`.
//! When persistence is enabled, the store serializes to JSON on every write
//! and loads from disk on startup — surviving process restarts.

use crate::types::{Message, Payload, WingId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::atomic::{AtomicU64, Ordering};

/// Lock the store mutex, logging a warning if it was poisoned by a previous panic.
/// Returns `None` only if the lock cannot be acquired at all (should never happen).
fn lock_store<'a>(
    mtx: &'a Mutex<HashMap<String, KnowledgeEntry>>,
) -> Option<MutexGuard<'a, HashMap<String, KnowledgeEntry>>> {
    match mtx.lock() {
        Ok(guard) => Some(guard),
        Err(poisoned) => {
            tracing::warn!("[KnowledgeWing] Mutex poisoned — recovering from previous panic");
            Some(poisoned.into_inner())
        }
    }
}

/// A single knowledge entry (serializable for persistence).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeEntry {
    values: Vec<String>,
    last_updated: chrono::DateTime<chrono::Utc>,
}

/// The Knowledge Wing — persistent knowledge store.
pub struct KnowledgeWing {
    /// Key → list of values (append-only log per key).
    store: Mutex<HashMap<String, KnowledgeEntry>>,
    query_count: AtomicU64,
    /// Optional file path for JSON persistence. When set, the store is
    /// serialized to this path after every write and loaded on construction.
    persist_path: Option<PathBuf>,
}

impl KnowledgeWing {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
            query_count: AtomicU64::new(0),
            persist_path: None,
        }
    }

    /// Create a KnowledgeWing that persists its store to a JSON file.
    /// Loads existing data from disk on construction.
    pub fn new_with_persistence(path: PathBuf) -> Self {
        let mut store = HashMap::new();
        if let Ok(data) = std::fs::read_to_string(&path)
            && let Ok(loaded) = serde_json::from_str::<HashMap<String, KnowledgeEntry>>(&data)
        {
            store = loaded;
            tracing::info!(
                "[KnowledgeWing] loaded {} entries from {}",
                store.len(),
                path.display()
            );
        }
        Self {
            store: Mutex::new(store),
            query_count: AtomicU64::new(0),
            persist_path: Some(path),
        }
    }

    /// Serialize the store to disk (if persistence is enabled).
    fn persist(&self) {
        if let Some(path) = &self.persist_path
            && let Ok(store) = self.store.lock()
        {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&*store)
                && let Err(e) = std::fs::write(path, &json)
            {
                tracing::warn!("[KnowledgeWing] persist failed: {}", e);
            }
        }
    }

    /// Handle an incoming message.
    /// Every payload type returns a response — unhandled types return `Payload::Error`.
    pub fn handle_message(&self, msg: &Message) -> Option<Message> {
        match &msg.payload {
            Payload::KnowledgeQuery { query, context } => {
                self.query_count.fetch_add(1, Ordering::Relaxed);
                let store = lock_store(&self.store)?;
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
                ..
            } => {
                let mut store = lock_store(&self.store)?;
                let entry =
                    store
                        .entry("yield_reports".to_string())
                        .or_insert_with(|| KnowledgeEntry {
                            values: Vec::new(),
                            last_updated: Utc::now(),
                        });
                entry.values.push(format!(
                    "yield={} sol={} dd={} at={}",
                    usdc_yield,
                    sol_reserves,
                    drawdown,
                    Utc::now().to_rfc3339()
                ));
                entry.last_updated = Utc::now();
                drop(store); // Release lock before persist
                self.persist();
                Some(Message::new(
                    WingId::Knowledge,
                    WingId::Coordinator,
                    Payload::Ack {
                        in_reply_to: msg.id,
                    },
                ))
            }

            Payload::Assessment {
                wing,
                score,
                bottlenecks,
                recommendations,
            } => {
                let mut store = lock_store(&self.store)?;
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
                drop(store); // Release lock before persist
                self.persist();
                Some(Message::new(
                    WingId::Knowledge,
                    WingId::Coordinator,
                    Payload::Ack {
                        in_reply_to: msg.id,
                    },
                ))
            }

            Payload::Heartbeat { .. } => {
                let store = lock_store(&self.store)?;
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
            let entry = store
                .entry(key.to_string())
                .or_insert_with(|| KnowledgeEntry {
                    values: Vec::new(),
                    last_updated: Utc::now(),
                });
            entry.values.push(value.to_string());
            entry.last_updated = Utc::now();
            drop(store);
            self.persist();
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
                source: None,
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
            Payload::Shutdown {
                reason: "test".to_string(),
            },
        );
        let response = wing.handle_message(&msg).unwrap();
        match response.payload {
            Payload::Error { reason, .. } => assert!(reason.contains("Unimplemented")),
            _ => panic!("Expected Error payload"),
        }
    }
}
