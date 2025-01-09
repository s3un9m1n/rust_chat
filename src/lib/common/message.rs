use serde_json::{json, Value};
use crate::common::protocol::MessageType;

/// JSON 메시지 생성 함수
/// - `json_type`: 메시지 타입 (MessageType 열거형)
/// - `content`: 추가 데이터를 포함하는 JSON 객체 (옵션)
pub fn create_message(json_type: &MessageType, content: Option<&Value>) -> String {
    let mut message = json!({ "type": json_type.as_str() });
    if let Some(content) = content {
        for (key, value) in content.as_object().unwrap() {
            message[key] = value.clone();
        }
    }
    message.to_string()
}

/// JSON 메시지 파싱 함수
/// - 입력: JSON 문자열
/// - 출력: Serde JSON 객체로 변환
pub fn parse_message(message: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(message)
}
