use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use project::common::message;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{protocol::Message, Error},
    WebSocketStream,
};

type ClientMap = Arc<
    RwLock<
        HashMap<
            String,
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
                Message,
            >,
        >,
    >,
>;

static mut NEXT_CLIENT_ID: u64 = 1;

fn generate_client_id() -> u64 {
    unsafe {
        let id = NEXT_CLIENT_ID;
        NEXT_CLIENT_ID += 1;
        id
    }
}

#[tokio::main]
async fn main() {
    // 서버 주소 설정 (localhost:8080)
    let addr = "127.0.0.1:8080".to_string();
    let listener = TcpListener::bind(&addr).await.unwrap();

    println!("WebSocket server listening on {}", addr);

    // 클라이언트 관리 목록
    // 모든 스레드에서 접근 및 수정이 가능해야 하기 때문에 스마트포인터(`Arc`) 사용
    let clients: ClientMap = Arc::new(RwLock::new(HashMap::new()));

    // FIXME: ctrl+c 입력시 클라이언트들 대상으로 우아한 종료 시도

    // 클라이언트 연결 대기
    while let Ok((stream, _)) = listener.accept().await {
        // 스레드마다 전달하기 위해 스마트포인터 `clone`
        let clients = Arc::clone(&clients);

        // 각 클라이언트마다 비동기적으로 처리
        tokio::spawn(handle_connection(stream, clients));
    }
}

async fn handle_connection(stream: tokio::net::TcpStream, clients: ClientMap) {
    // WebSocket 핸드쉐이크 수행
    let ws_stream = accept_async(stream)
        .await
        .expect("Error during WebSocket handshake");

    println!("New WebSocket connection");

    // WebSocket 스트림 분리
    let (write, mut read) = ws_stream.split();

    // 클라이언트 ID 발급
    let client_id = generate_client_id().to_string();

    // `write` 스트림을 클라이언트 목록에 추가
    insert_client_list(&clients, &client_id, write).await;
    println!("Client joined. (ID){}", client_id);

    // 클라이언트에게 ID 전송
    let mut fields = serde_json::Map::new();
    fields.insert("id".to_string(), serde_json::json!(client_id));
    fields.insert("text".to_string(), serde_json::json!(format!("Hello {}!", client_id)));

    let hello_message = message::create_message("hello", Some(&fields));
    unicast_message(&client_id, &clients, &hello_message.to_string()).await;

    // 사용자 접속 알림(broadcast)
    notify_join(&client_id, &clients).await;

    while let Some(message) = read.next().await {
        if let Err(_) = process_message(&client_id, &clients, message).await {
            break;
        }
    }

    // `write` 스트림을 클라이언트 목록에 삭제
    remove_client_list(&clients, &client_id).await;

    // 사용저 접속 종료 알림
    notify_leave(&client_id, &clients).await;
}

async fn insert_client_list(
    clients: &ClientMap,
    client_id: &str,
    ws_write: SplitSink<WebSocketStream<TcpStream>, Message>,
) {
    let mut clients_lock: tokio::sync::RwLockWriteGuard<
        '_,
        HashMap<String, SplitSink<WebSocketStream<TcpStream>, Message>>,
    > = clients.write().await;
    clients_lock.insert(client_id.to_string(), ws_write);
}

async fn remove_client_list(clients: &ClientMap, client_id: &str) {
    let mut clients_lock = clients.write().await;
    clients_lock.remove(client_id);
}

async fn notify_join(client_id: &str, clients: &ClientMap) {
    // 접속 알림 메시지
    let join_message = json!({
        "type": "user_joined",
        "id": client_id
    })
    .to_string();

    broadcast_message(&client_id, &clients, &join_message).await;
}

async fn notify_leave(client_id: &str, clients: &ClientMap) {
    // 접속 종료 알림 메시지
    let leave_message = json!({
        "type": "user_left",
        "id": client_id
    })
    .to_string();

    broadcast_message(&client_id, &clients, &leave_message).await;
}

async fn handle_chat_message(client_id: &str, clients: &ClientMap, text: &str) {
    let chat_message = json!({
        "type": "chat",
        "id": client_id,
        "text": text
    })
    .to_string();

    broadcast_message(&client_id, &clients, &chat_message).await;
}

async fn broadcast_message(client_id: &str, clients: &ClientMap, message: &str) {
    // 락 획득
    // FIXME: write lock 밖에 사용하지 않는 것 같음
    let mut clients_lock = clients.write().await;

    // 자신을 제외한 클라이언트에게 브로드캐스트
    for (id, sender) in clients_lock.iter_mut() {
        if id == client_id {
            continue;
        }
        if let Err(e) = sender.send(Message::Text(message.to_string())).await {
            eprintln!("Error broadcasting message. (TO){}, (ERR){:?}", id, e);
        }
    }
}

async fn unicast_message(client_id: &str, clients: &ClientMap, message: &str) {
    // 락 획득
    // FIXME: write lock 밖에 사용하지 않는 것 같음
    let mut clients_lock = clients.write().await;

    // 자신을 제외한 클라이언트에게 브로드캐스트
    for (id, sender) in clients_lock.iter_mut() {
        if id != client_id {
            continue;
        }
        if let Err(e) = sender.send(Message::Text(message.to_string())).await {
            eprintln!("Error unicasting message. (TO){}, (ERR){:?}", id, e);
        }
    }
}

async fn process_message(
    client_id: &str,
    clients: &ClientMap,
    message: Result<Message, Error>,
) -> Result<(), ()> {
    match message {
        Ok(Message::Text(text)) => process_json_message(client_id, clients, &text).await,
        Ok(Message::Binary(_)) => process_binary_message(),
        Err(e) => process_error_message(client_id, e),
        Ok(_) => Ok(()),
    }
}

async fn process_json_message(
    client_id: &str,
    clients: &ClientMap,
    message: &str,
) -> Result<(), ()> {
    if let Ok(json_message) = serde_json::from_str::<Value>(&message) {
        match json_message.get("type").and_then(|t| t.as_str()) {
            Some("user_exit") => {
                println!("Exit client. (FROM){}", client_id);
                return Err(());
            }
            Some("chat") => {
                if let Some(text) = json_message.get("text").and_then(|t| t.as_str()) {
                    println!("Received message. (FROM){}, (MSG){}", client_id, text);
                    handle_chat_message(&client_id, &clients, text).await;
                }
            }
            _ => {}
        }
    } else {
        println!("Invalid JSON received: {}", message);
        return Err(());
    }
    Ok(())
}

fn process_binary_message() -> Result<(), ()> {
    // 바이너리 메시지는 처리하지 않음
    println!("Received binary message");
    Ok(())
}

fn process_error_message(client_id: &str, e: Error) -> Result<(), ()> {
    match e {
        Error::Protocol(protocol_error) => {
            if protocol_error
                .to_string()
                .contains("Connection reset without closing handshake")
            {
                // 강제 종료 감지
                println!(
                    "Client forcibly disconnected without closing handshake. (ID){}",
                    client_id
                );
            } else {
                // 다른 프로토콜 에러 처리
                eprintln!("Protocol error: {} (ID){})", protocol_error, client_id);
            }
        }
        _ => {
            eprintln!("Error reading message: {}", e);
        }
    }
    Err(())
}
