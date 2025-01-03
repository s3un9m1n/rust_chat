use serde_json::json;

/// JSON 메시지 생성
pub fn create_message(json_type: &str, text: Option<&String>) -> String {
    match text {
        // text 필드 있는 경우
        Some(content) => json!({
            "type": json_type,
            "text": content,
        })
        .to_string(),
        // text 필드 없는 경우
        None => json!({
            "type": json_type,
        })
        .to_string(),
    }
}
