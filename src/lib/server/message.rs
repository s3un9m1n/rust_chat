use crate::common::message;

/// 클라이언트 접속 알림 메시지 생성
pub fn create_hello_message(client_id: &str) -> String {
    message::create_user_join_message(client_id)
}

/// 클라이언트 종료 알림 메시지 생성
pub fn create_user_left_message(client_id: &str) -> String {
    message::create_user_exit_message(client_id)
}
