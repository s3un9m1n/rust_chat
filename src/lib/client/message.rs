use crate::common::message;

/// 클라이언트 전용 채팅 메시지 생성 함수
pub fn create_chat_message(text: &str) -> String {
    message::create_chat_message("client", text) // ID를 "client"로 설정 (예제)
}

/// 클라이언트 종료 메시지 생성 함수
pub fn create_exit_message() -> String {
    message::create_user_exit_message("client")
}
