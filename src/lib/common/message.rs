use serde_json::{json, Value};
use crate::common::protocol::MessageType;

/// 채팅 메시지 생성 함수
pub fn create_chat_message(id: &str, text: &str) -> String {
    json!({
        "type": MessageType::Chat.as_str(),
        "id": id,
        "text": text
    })
    .to_string()
}

/// 사용자 접속 메시지 생성 함수
pub fn create_user_join_message(id: &str) -> String {
    json!({
        "type": MessageType::UserJoined.as_str(),
        "id": id
    })
    .to_string()
}

/// 사용자 종료 메시지 생성 함수
pub fn create_user_exit_message(id: &str) -> String {
    json!({
        "type": MessageType::UserExit.as_str(),
        "id": id
    })
    .to_string()
}

/// 메시지 파싱 함수
pub fn parse_message(message: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(message)
}
