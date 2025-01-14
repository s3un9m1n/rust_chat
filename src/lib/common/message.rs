use crate::common::protocol::MessageType;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_type: String,
    pub id: Option<String>,
    pub text: Option<String>,
}

/// JSON 메시지 생성 함수들
pub fn create_message(message_type: MessageType, id: Option<&str>, text: Option<&str>) -> String {
    let mut msg = json!({
        "type": message_type.as_str()
    });
    if let Some(id_value) = id {
        msg["id"] = json!(id_value);
    }
    if let Some(text_value) = text {
        msg["text"] = json!(text_value);
    }
    msg.to_string()
}

/// 메시지 파싱 함수
pub fn parse_message(message: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(message)
}

/// Rust 구조체로 파싱
pub fn parse_to_struct(message: &str) -> Result<ChatMessage, serde_json::Error> {
    serde_json::from_str(message)
}

/// 채팅 메시지 생성 (공통 사용)
pub fn create_chat_message(id: &str, text: &str) -> String {
    create_message(MessageType::Chat, Some(id), Some(text))
}
