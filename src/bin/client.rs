use ctrlc;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::process;
use tokio::io::{self, AsyncBufReadExt};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::main]
async fn main() {
    // Ctrl+C 처리
    ctrlc::set_handler(move || {
        println!("\nExit program.");
        process::exit(0);
    })
    .expect("Error setting Ctrl+C handler");

    let url = "ws://localhost:8080";

    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Connected to the server!");

    // 비동기 입력을 처리하기 위한 채널
    let (tx, mut rx) = mpsc::channel(32);

    // 사용자 입력을 읽는 태스크
    tokio::spawn(read_user_input(tx));

    loop {
        // 두 개 이상의 future 중 먼저 완료되는 future 값을 return 해줌
        tokio::select! {
            // 사용자 입력을 처리
            Some(message_input) = rx.recv() => {
                if handle_user_input(&mut ws_stream, message_input).await.is_err() {
                    break;
                }
            }
            // 서버로부터 메시지 수신
            Some(message_received) = ws_stream.next() => {
                if let Err(e) = handle_server_message(message_received).await {
                    eprintln!("Error handling server message: {:?}", e);
                    break;
                }
            }
            // 소켓 스트림이 종료된 경우
            else => {
                println!("Connection closed.");
                break;
            }
        }
    }
}

async fn read_user_input(tx: mpsc::Sender<String>) {
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if !line.trim().is_empty() {
            if tx.send(line).await.is_err() {
                break; // 채널이 닫힌 경우 종료
            }
        }
    }
}
async fn handle_user_input<S>(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<S>,
    message_input: String,
) -> Result<(), ()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // 빈 메시지는 무시
    if message_input.trim().is_empty() {
        return Ok(());
    }

    // 종료 요청
    if message_input == "exit" {
        // TODO: 정상 종료 메시지 전송
        let exit_message = json!({
            "type": "user_exit",
        })
        .to_string();

        ws_stream
            .send(Message::Text(exit_message))
            .await
            .expect("Failed to send exit message");

        return Err(());
    }
    // 일반 데이터 전송
    else {
        let chat_message = json!({
            "type": "chat",
            "text": message_input
        })
        .to_string();

        ws_stream
            .send(Message::Text(chat_message.clone()))
            .await
            .expect("Failed to send message");
        println!("Sent message: {}", message_input);
    }

    Ok(())
}

async fn handle_server_message(
    message_received: Result<Message, tokio_tungstenite::tungstenite::Error>,
) -> Result<(), ()> {
    match message_received {
        Ok(Message::Text(message_json)) => {
            if let Ok(message) = serde_json::from_str::<Value>(&message_json) {
                match message.get("type").and_then(|t| t.as_str()) {
                    Some("user_joined") => {
                        if let Some(id) = message.get("id") {
                            println!("User join! (ID){}", id.as_str().unwrap_or_default());
                        } else {
                            println!("Invalid message format: {}", message_json);
                        }
                    }
                    Some("user_left") => {
                        if let Some(id) = message.get("id") {
                            println!("User left! (ID){}", id.as_str().unwrap_or_default());
                        } else {
                            println!("Invalid message format: {}", message_json);
                        }
                    }
                    Some("chat") => {
                        if let (Some(id), Some(text)) = (message.get("id"), message.get("text")) {
                            println!(
                                "Received message. (FROM){}, (MSG){}",
                                id.as_str().unwrap_or("unknown"),
                                text.as_str().unwrap_or("")
                            );
                        } else {
                            println!("Invalid message format: {}", message_json);
                        }
                    }
                    _ => {
                        println!("Unknown message type: {}", message_json);
                    }
                }
            } else {
                println!("Invalid message format: {}", message_json);
            }
            Ok(())
        }
        Ok(Message::Close(_)) => {
            println!("Server closed the connection");
            Err(())
        }
        Err(e) => {
            println!("Connection error: {:?}", e);
            match e {
                tokio_tungstenite::tungstenite::Error::ConnectionClosed => {
                    println!("Connection closed by server.");
                }
                tokio_tungstenite::tungstenite::Error::AlreadyClosed => {
                    println!("The connection is already closed.");
                }
                tokio_tungstenite::tungstenite::Error::Io(_) => {
                    println!("Network error occurred. The server might be down.");
                }
                _ => {
                    println!("An unexpected error occurred: {:?}", e);
                }
            }
            Err(())
        }
        _ => {
            println!("Unknown server message. {:?}", message_received);
            Ok(())
        }
    }
}
