use futures_util::StreamExt;
use rust_chat::{broadcast_message, ClientsMap};
use std::time::Duration;
use tokio::time;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::protocol::Message};

#[tokio::test]
async fn test_broadcast_message() {
    let clients: ClientsMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

    // Mock 서버 및 클라이언트 연결 생성
    let listener = TcpListener::bind("127.0.0.1:9001")
        .await
        .expect("Failed to bind test server");

    let server_clients = clients.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            if let Ok(ws_stream) = accept_async(stream).await {
                let (write, _) = ws_stream.split();
                // `write`를 `Arc::new(Mutex::new(...))`로 감싸기
                server_clients
                    .lock()
                    .await
                    .insert("mock_client".to_string(), Arc::new(Mutex::new(write)));
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
                // `write`를 `Arc::new(Mutex::new(...))`로 감싸기
                server_clients
                    .lock()
                    .await
                    .insert("mock_client".to_string(), Arc::new(Mutex::new(write)));
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
    // Step 1: Start the server
    let mut server_process = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("server")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Failed to start server");

    // Wait for server to start
    time::sleep(Duration::from_secs(2)).await;

    // Step 2: Connect clients
    let (mut client_a, _) = connect_async("ws://127.0.0.1:8080")
        .await
        .expect("Client A connection failed");
    let (mut client_b, _) = connect_async("ws://127.0.0.1:8080")
        .await
        .expect("Client B connection failed");

    // Step 3: Test "join" message
    if let Some(Ok(Message::Text(received_message))) = client_b.next().await {
        assert!(received_message.contains("\"type\":\"join\""), "Join message not received");
    } else {
        panic!("Client B did not receive join broadcast");
    }

    // Step 4: Disconnect client A
    client_a.close(None).await.expect("Client A disconnect failed");
    time::sleep(Duration::from_secs(1)).await;

    // Step 5: Test "leave" message
    if let Some(Ok(Message::Text(received_message))) = client_b.next().await {
        assert!(received_message.contains("\"type\":\"leave\""), "Leave message not received");
    } else {
        panic!("Client B did not receive leave broadcast");
    }

    // Step 6: Stop the server
    server_process.kill().await.expect("Failed to stop server");
}