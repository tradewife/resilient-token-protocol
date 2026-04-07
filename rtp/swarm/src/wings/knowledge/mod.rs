//! Knowledge Wing — realtime knowledge graph and recall.
//!
//! Week 3 implementation. Currently a stub.

use crate::types::{Message, WingId};

pub fn handle_message(msg: &Message) -> Option<Message> {
    match &msg.payload {
        crate::types::Payload::KnowledgeQuery { query, .. } => Some(Message::new(
            WingId::Knowledge,
            WingId::Coordinator,
            crate::types::Payload::KnowledgeResult {
                results: vec![format!("No results for: {}", query)],
            },
        )),
        _ => None,
    }
}
