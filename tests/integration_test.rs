use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{accept_async, connect_async};

#[tokio::test]
async fn test_server_client_integration() {
    // 1. 서버 시작
    let addr = "127.0.0.1:8081";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let ws_stream = accept_async(stream)
                    .await
                    .expect("Failed to accept connection");
                let (mut write, mut read) = ws_stream.split();

                // Echo server: Send received messages back to the client
                while let Some(Ok(message)) = read.next().await {
                    if let Message::Text(text) = message {
                        let _ = write.send(Message::Text(text)).await;
                    }
                }
            });
        }
    });

    // 2. 클라이언트 연결
    let (mut ws_stream, _) = connect_async(format!("ws://{}", addr))
        .await
        .expect("Failed to connect to server");

    // 3. 클라이언트 메시지 전송
    let test_message = json!({
        "type": "chat",
        "sender": "test_user",
        "message": "Hello, World!"
    })
    .to_string();

    ws_stream
        .send(Message::Text(test_message.clone()))
        .await
        .expect("Failed to send message");

    // 4. 서버 메시지 수신
    if let Some(Ok(Message::Text(received_message))) = ws_stream.next().await {
        assert_eq!(received_message, test_message);
    } else {
        panic!("Failed to receive echo message from server");
    }

    // 5. 연결 종료
    ws_stream
        .close(None)
        .await
        .expect("Failed to close connection");
}
