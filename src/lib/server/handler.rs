use crate::common::message;
use crate::common::protocol::MessageType;

pub fn handle_incoming_message(message: &str) {
    match message::parse_message(message) {
        Ok(parsed) => match parsed["type"].as_str() {
            Some("chat") => println!("Chat message received: {}", parsed["text"]),
            Some("user_exit") => println!("User exit message received."),
            _ => println!("Unknown message type."),
        },
        Err(e) => println!("Failed to parse message: {:?}", e),
    }
}
