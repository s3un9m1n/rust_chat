use crate::common::message;
use crate::common::protocol::MessageType;

pub fn create_chat_message(client_id: &str, text: &str) -> String {
    message::create_chat_message(client_id, text)
}

pub fn create_exit_message(client_id: &str) -> String {
    message::create_message(MessageType::UserExit, Some(client_id), None)
}
