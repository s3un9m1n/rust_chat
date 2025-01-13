use crate::common::protocol::MessageType;
use serde_json::{json, Value};

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

pub fn create_chat_message(client_id: &str, text: &str) -> String {
    create_message(MessageType::Chat, Some(client_id), Some(text))
}

/// 메시지 파싱 함수
pub fn parse_message(message: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(message)
}
