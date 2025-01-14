use crate::common::message;
use crate::common::protocol::MessageType;

pub fn create_hello_message(client_id: &str) -> String {
    message::create_message(MessageType::Hello, Some(client_id), None)
}

pub fn create_user_left_message(client_id: &str) -> String {
    message::create_message(MessageType::UserLeft, Some(client_id), None)
}

pub fn create_chat_message(client_id: &str, text: &str) -> String {
    message::create_chat_message(client_id, text)
}
