/// 메시지 타입을 정의하는 열거형
#[derive(Debug)]
pub enum MessageType {
    Hello,
    Chat,
    UserJoined,
    UserLeft,
    UserExit,
}

impl MessageType {
    /// 열거형 값을 문자열로 변환
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Hello => "hello",
            MessageType::Chat => "chat",
            MessageType::UserJoined => "user_joined",
            MessageType::UserLeft => "user_left",
            MessageType::UserExit => "user_exit",
        }
    }

    /// 문자열을 열거형 값으로 변환
    pub fn from_str(type_str: &str) -> Option<Self> {
        match type_str {
            "hello" => Some(MessageType::Hello),
            "chat" => Some(MessageType::Chat),
            "user_joined" => Some(MessageType::UserJoined),
            "user_left" => Some(MessageType::UserLeft),
            "user_exit" => Some(MessageType::UserExit),
            _ => None,
        }
    }
}
