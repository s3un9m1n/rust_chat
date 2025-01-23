use futures_util::StreamExt;
use rust_chat::{broadcast_message, ClientsMap};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

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

#[tokio::test]
async fn test_broadcast_message_between_two_clients() {
    // Initialize shared clients map
    let clients: ClientsMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

    // Start mock WebSocket server
    let listener = TcpListener::bind("127.0.0.1:9002")
        .await
        .expect("Failed to bind test server");

    let server_clients = clients.clone();
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

    // Connect two WebSocket clients
    let (mut _client_a, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:9002")
        .await
        .unwrap();
    let (mut client_b, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:9002")
        .await
        .unwrap();

    // Wait for clients to connect
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send a test message from client_a
    let test_message = "Hello from client_a";
    broadcast_message(test_message, &clients).await;

    // Receive the message on client_b
    if let Some(Ok(Message::Text(received_message))) = client_b.next().await {
        assert_eq!(received_message, test_message);
    } else {
        panic!("Client B did not receive the broadcast message");
    }
}

#[tokio::test]
async fn test_client_join_and_leave_broadcast() {
    let clients: ClientsMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

    let listener = TcpListener::bind("127.0.0.1:9003")
        .await
        .expect("Failed to bind test server");

    let server_clients = clients.clone();
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

    let (mut client_a, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:9003")
        .await
        .unwrap();
    let (mut client_b, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:9003")
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // "join" 메시지 수신 테스트
    use tokio::time;
    if let Ok(Some(Ok(Message::Text(received_message)))) =
        time::timeout(tokio::time::Duration::from_secs(5), client_b.next()).await
    {
        println!("Client B received: {}", received_message); // 디버깅 로그
        assert!(received_message.contains("\"type\":\"join\""));
    } else {
        panic!("Client B did not receive join broadcast within timeout");
    }

    // 클라이언트 A 종료
    client_a.close(None).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // "leave" 메시지 수신 테스트
    if let Ok(Some(Ok(Message::Text(received_message)))) =
        time::timeout(tokio::time::Duration::from_secs(5), client_b.next()).await
    {
        println!("Client B received: {}", received_message); // 디버깅 로그
        assert!(received_message.contains("\"type\":\"leave\""));
    } else {
        panic!("Client B did not receive leave broadcast within timeout");
    }
}
