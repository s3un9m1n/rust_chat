use futures_util::{SinkExt, StreamExt};
use project::client::message as client_message;
use project::common::message as common_message;
use project::common::protocol::MessageType;
use tokio::io::{self, AsyncBufReadExt};
use tokio::signal;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::Error;

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8080";

    // 사용자 ID 저장용
    let mut user_id = None;

    // (1) WebSocket 서버에 연결
    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Connected to the server!");

    // (2) 사용자 입력 채널 생성
    let (tx, mut rx) = mpsc::channel(32);

    // 사용자 입력 비동기 처리 태스크 실행
    tokio::spawn(read_user_input(tx));

    loop {
        tokio::select! {
            // (3) 서버로부터 메시지 수신
            Some(message_received) = ws_stream.next() => {
                if let Err(e) = handle_server_message(message_received, &mut user_id).await {
                    eprintln!("Error handling server message: {:?}", e);
                    break;
                }
            }
            // (4) 사용자 입력 처리
            Some(message_input) = rx.recv() => {
                if handle_user_input(&mut ws_stream, &message_input, user_id.as_ref().unwrap_or(&"".to_string())).await.is_err() {
                    break;
                }
            }
            // (5) Ctrl+C 처리
            _ = signal::ctrl_c() => {
                let exit_message = client_message::create_exit_message(user_id.as_deref().unwrap_or(""));
                if let Err(e) = send_to_server(&mut ws_stream, exit_message).await {
                    eprintln!("Failed to send exit message: {:?}", e);
                }
                println!("\nExit program.");
                break;
            }
            // WebSocket 연결 종료
            else => {
                println!("Connection closed.");
                break;
            }
        }
    }

    // WebSocket 스트림 종료
    if let Err(e) = ws_stream.close(None).await {
        eprintln!("Error closing WebSocket: {:?}", e);
    }
}

/// 사용자 입력을 읽어 채널로 전송하는 비동기 함수
async fn read_user_input(tx: mpsc::Sender<String>) {
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if !line.trim().is_empty() {
            if tx.send(line).await.is_err() {
                break;
            }
        }
    }
}

/// 사용자 입력을 WebSocket 서버로 전송
async fn handle_user_input<S>(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<S>,
    message_input: &String,
    user_id: &String,
) -> Result<(), ()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    if message_input.trim().is_empty() {
        return Ok(());
    }

    if message_input == "exit" {
        let exit_message = client_message::create_exit_message(user_id);
        send_to_server(ws_stream, exit_message)
            .await
            .expect("Failed to send exit message");
    } else {
        let chat_message = client_message::create_chat_message(&message_input, user_id);
        send_to_server(ws_stream, chat_message)
            .await
            .expect("Failed to send chat message");
        println!("Sent message: {}", message_input);
    }
    Ok(())
}

/// 서버에서 받은 메시지 처리
async fn handle_server_message(
    message_received: Result<Message, Error>,
    user_id: &mut Option<String>,
) -> Result<(), ()> {
    match message_received {
        Ok(Message::Text(message_json)) => handle_server_json_message(message_json, user_id),
        Ok(Message::Close(_)) => {
            println!("Server closed the connection");
            Err(())
        }
        Err(e) => {
            eprintln!("Connection error: {:?}", e);
            Err(())
        }
        _ => {
            println!("Unknown server message.");
            Err(())
        }
    }
}

/// 서버로 메시지 전송
async fn send_to_server<S>(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<S>,
    message: String,
) -> Result<(), Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    ws_stream.send(Message::Text(message)).await.map_err(|err| {
        eprintln!("Failed to send message: {:?}", err);
        err
    })
}

/// JSON 메시지를 파싱하여 서버 메시지 처리
fn handle_server_json_message(message: String, user_id: &mut Option<String>) -> Result<(), ()> {
    let json = common_message::parse_message(&message).map_err(|_| {
        eprintln!("Invalid message format: {}", message);
    })?;

    match MessageType::from_str(json.get("type").and_then(|t| t.as_str()).unwrap_or("")) {
        Some(MessageType::Hello) => handle_server_user_hello(&json, user_id),
        Some(MessageType::UserJoined) => handle_server_user_join(&json),
        Some(MessageType::UserLeft) => handle_server_user_left(&json),
        Some(MessageType::Chat) => handle_server_user_chat(&json),
        _ => {
            eprintln!("Unknown message type: {}", json);
            Err(())
        }
    }
}

// 서버에서 받은 메시지 유형별 처리 함수들 (Hello, Join, Leave, Chat)
fn handle_server_user_hello(
    json: &serde_json::Value,
    user_id: &mut Option<String>,
) -> Result<(), ()> {
    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
        *user_id = Some(id.to_string());
        println!("Hello! Your ID is {}", id);
        Ok(())
    } else {
        eprintln!("Invalid hello message format: {}", json);
        Err(())
    }
}

fn handle_server_user_join(json: &serde_json::Value) -> Result<(), ()> {
    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
        println!("User joined: {}", id);
        Ok(())
    } else {
        eprintln!("Invalid join message format: {}", json);
        Err(())
    }
}

fn handle_server_user_left(json: &serde_json::Value) -> Result<(), ()> {
    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
        println!("User left: {}", id);
        Ok(())
    } else {
        eprintln!("Invalid leave message format: {}", json);
        Err(())
    }
}

fn handle_server_user_chat(json: &serde_json::Value) -> Result<(), ()> {
    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
        if let Some(text) = json.get("text").and_then(|v| v.as_str()) {
            println!("[{}]: {}", id, text);
            Ok(())
        } else {
            eprintln!("Invalid chat message format: {}", json);
            Err(())
        }
    } else {
        eprintln!("Invalid chat message format: {}", json);
        Err(())
    }
}
