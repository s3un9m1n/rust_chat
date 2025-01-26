use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::StreamExt;
use rust_chat::broadcast_message;
use rust_chat::ClientsMap;

#[tokio::main]
async fn main() {
    let clients: ClientsMap = Arc::new(Mutex::new(HashMap::new()));
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind server");

    println!("Server running on ws://127.0.0.1:8080");

    while let Ok((stream, _)) = listener.accept().await {
        let clients = clients.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, clients).await {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(stream: TcpStream, clients: ClientsMap) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = accept_async(stream).await?;
    let (write, mut read) = ws_stream.split();

    // 클라이언트 ID 생성
    let client_id = uuid::Uuid::new_v4().to_string();
    clients.lock().await.insert(client_id.clone(), Arc::new(Mutex::new(write)));

    // 접속 알림 broadcast
    let join_message = format!("{{\"type\":\"join\",\"user\":\"{}\"}}", client_id);
    broadcast_message(&join_message, &clients).await;

    while let Some(Ok(Message::Text(text))) = read.next().await {
        // 받은 메시지를 그대로 broadcast
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
            if parsed["type"] == "chat" {
                broadcast_message(&text, &clients).await;
            } else {
                eprintln!("Unknown message type: {}", parsed["type"]);
            }
        } else {
            eprintln!("Failed to parse message: {}", text);
        }
    }

    // 클라이언트 종료 처리
    clients.lock().await.remove(&client_id);

    let leave_message = format!("{{\"type\":\"leave\",\"user\":\"{}\"}}", client_id);
    broadcast_message(&leave_message, &clients).await;

    Ok(())
}
