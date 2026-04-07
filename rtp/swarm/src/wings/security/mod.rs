//! Security Wing — threat detection and defense.
//!
//! Week 3 implementation. Currently a stub.

use crate::types::{Message, WingId};

pub fn handle_message(msg: &Message) -> Option<Message> {
    match &msg.payload {
        crate::types::Payload::Heartbeat { .. } => Some(Message::new(
            WingId::Security,
            WingId::Coordinator,
            crate::types::Payload::Ack { in_reply_to: msg.id },
        )),
        _ => None,
    }
}
