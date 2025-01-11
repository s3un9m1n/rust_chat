use futures_util::{SinkExt, StreamExt};
use project::server::message as server_message;
use project::common::message as common_message;
use project::common::protocol::MessageType;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, WebSocketStream}; // 추가된 부분
use tokio_tungstenite::tungstenite::{protocol::Message, Error};

/// 클라이언트 ID와 메시지 전송 Sink를 관리하는 타입
type ClientMap = Arc<RwLock<HashMap<String, futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>>>>;

/// 전역으로 사용할 클라이언트 ID 생성기
static mut NEXT_CLIENT_ID: u64 = 1;

/// 고유 클라이언트 ID를 생성
fn generate_client_id() -> u64 {
    unsafe {
        let id = NEXT_CLIENT_ID;
        NEXT_CLIENT_ID += 1;
        id
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await.unwrap();

    println!("WebSocket server listening on {}", addr);

    // 클라이언트 관리 맵 초기화
    let clients: ClientMap = Arc::new(RwLock::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let clients = Arc::clone(&clients);

        // 클라이언트 연결 처리
        tokio::spawn(handle_connection(stream, clients));
    }
}

/// 클라이언트 연결을 처리하는 비동기 함수
async fn handle_connection(stream: TcpStream, clients: ClientMap) {
    let ws_stream = accept_async(stream)
        .await
        .expect("Error during WebSocket handshake");

    let (write, mut read) = ws_stream.split();
    let client_id = generate_client_id().to_string();

    insert_client(&clients, &client_id, write).await;

    println!("Client joined: {}", client_id);

    let hello_message = server_message::create_hello_message(&client_id);
    unicast_message(&client_id, &clients, &hello_message).await;

    notify_join(&client_id, &clients).await;

    while let Some(message) = read.next().await {
        if let Err(_) = process_message(&client_id, &clients, message).await {
            break;
        }
    }

    remove_client(&clients, &client_id).await;
    notify_leave(&client_id, &clients).await;
}

/// 클라이언트 목록에 추가
async fn insert_client(
    clients: &ClientMap,
    client_id: &str,
    ws_write: futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>,
) {
    let mut clients_lock = clients.write().await;
    clients_lock.insert(client_id.to_string(), ws_write);
}

/// 클라이언트 목록에서 제거
async fn remove_client(clients: &ClientMap, client_id: &str) {
    let mut clients_lock = clients.write().await;
    clients_lock.remove(client_id);
}

/// 클라이언트 접속 알림
async fn notify_join(client_id: &str, clients: &ClientMap) {
    let join_message = server_message::create_hello_message(&client_id);
    broadcast_message(client_id, clients, &join_message).await;
}

/// 클라이언트 종료 알림
async fn notify_leave(client_id: &str, clients: &ClientMap) {
    let leave_message = server_message::create_user_left_message(client_id);
    broadcast_message(client_id, clients, &leave_message).await;
}

async fn broadcast_message(
    sender_id: &str,
    clients: &ClientMap,
    message: &str,
) {
    let mut clients_lock = clients.write().await; // 수정 가능한 락을 획득

    for (id, client) in clients_lock.iter_mut() { // iter_mut()를 사용
        if id == sender_id {
            continue;
        }
        if let Err(e) = client.send(Message::Text(message.to_string())).await {
            eprintln!("Failed to send message to {}: {:?}", id, e);
        }
    }
}

async fn unicast_message(
    client_id: &str,
    clients: &ClientMap,
    message: &str,
) {
    let mut clients_lock = clients.write().await; // 수정 가능한 락을 획득

    if let Some(client) = clients_lock.get_mut(client_id) { // get_mut()를 사용
        if let Err(e) = client.send(Message::Text(message.to_string())).await {
            eprintln!("Failed to send message to {}: {:?}", client_id, e);
        }
    }
}

/// 수신 메시지 처리
async fn process_message(
    client_id: &str,
    clients: &ClientMap,
    message: Result<Message, Error>,
) -> Result<(), ()> {
    match message {
        Ok(Message::Text(text)) => process_json_message(client_id, clients, &text).await,
        Ok(_) => Ok(()), // 바이너리 메시지는 무시
        Err(e) => {
            eprintln!("Error processing message: {:?}", e);
            Err(())
        }
    }
}

/// JSON 메시지를 처리
async fn process_json_message(
    client_id: &str,
    clients: &ClientMap,
    message: &str,
) -> Result<(), ()> {
    let json_message = common_message::parse_message(message).map_err(|_| {
        eprintln!("Invalid JSON: {}", message);
    })?;

    match MessageType::from_str(json_message.get("type").and_then(|t| t.as_str()).unwrap_or("")) {
        Some(MessageType::UserExit) => {
            println!("Client {} disconnected", client_id);
            Err(())
        }
        Some(MessageType::Chat) => {
            if let Some(text) = json_message.get("text").and_then(|t| t.as_str()) {
                println!("Message from {}: {}", client_id, text);
                let chat_message = common_message::create_chat_message(&client_id, &text);
                broadcast_message(client_id, clients, &chat_message).await;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
