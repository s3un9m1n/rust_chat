use rust_chat::{broadcast_message, ClientsMap};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::test]
async fn test_broadcast_message() {
    // Mock 클라이언트 저장소
    let clients: ClientsMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

    // Mock 클라이언트 추가
    let mock_client = Arc::new(Mutex::new(Vec::new()));
    clients.lock().await.insert(
        "mock_client".to_string(),
        Box::new(MockSink::new(mock_client.clone()))
            as Box<
                dyn futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
                    + Send,
            >,
    );

    // 브로드캐스트 메시지 테스트
    let test_message = "Test broadcast message";
    broadcast_message(test_message, &clients).await;

    // Mock 클라이언트가 메시지를 받았는지 확인
    let received_messages = mock_client.lock().await;
    assert_eq!(received_messages.len(), 1);
    assert_eq!(received_messages[0], test_message);
}

#[derive(Clone)]
struct MockSink {
    messages: Arc<Mutex<Vec<String>>>,
}

impl MockSink {
    fn new(messages: Arc<Mutex<Vec<String>>>) -> Self {
        Self { messages }
    }
}

#[async_trait::async_trait]
impl futures_util::Sink<Message> for MockSink {
    type Error = tokio_tungstenite::tungstenite::Error;

    async fn send(&mut self, item: Message) -> Result<(), Self::Error> {
        if let Message::Text(text) = item {
            self.messages.lock().await.push(text);
        }
        Ok(())
    }

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        futures::executor::block_on(self.get_mut().send(item))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}
