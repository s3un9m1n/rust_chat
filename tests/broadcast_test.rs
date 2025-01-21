use futures_util::StreamExt;
use rust_chat::{broadcast_message, ClientsMap};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message; // 필요한 트레이트를 가져옵니다.

#[tokio::test]
async fn test_broadcast_message() {
    let clients: ClientsMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

    // Mock 서버 및 클라이언트 연결 생성
    let listener = TcpListener::bind("127.0.0.1:9001")
        .await
        .expect("Failed to bind test server");

    let server_clients = clients.clone(); // Clone for server
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            if let Ok(ws_stream) = accept_async(stream).await {
                let (write, _) = ws_stream.split();
                server_clients
                    .lock()
                    .await
                    .insert("mock_client".to_string(), write);
            }
        }
    });

    // 클라이언트 WebSocket 연결 생성
    let (mut ws_stream, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:9001")
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 테스트 메시지
    let test_message = "Test broadcast message";
    broadcast_message(test_message, &clients).await;

    // 클라이언트에서 메시지 수신 확인
    if let Some(Ok(Message::Text(received_message))) = ws_stream.next().await {
        assert_eq!(received_message, test_message);
    } else {
        panic!("No message received!");
    }
}
