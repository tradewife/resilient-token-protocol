//! Trading Wing — yield research, validation, and execution.
//!
//! Week 4 implementation. Currently a stub that responds to Coordinator messages.

use crate::types::{Message, WingId};

/// Stub handler for Trading Wing messages.
pub fn handle_message(msg: &Message) -> Option<Message> {
    match &msg.payload {
        crate::types::Payload::TradingConfig { .. } => Some(Message::new(
            WingId::Trading,
            WingId::Coordinator,
            crate::types::Payload::Ack { in_reply_to: msg.id },
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Payload;

    #[test]
    fn handles_trading_config() {
        let msg = Message::new(
            WingId::Coordinator,
            WingId::Trading,
            Payload::TradingConfig {
                strategy: "mr".to_string(),
                params: serde_json::json!({}),
            },
        );
        let response = handle_message(&msg);
        assert!(response.is_some());
    }
}
