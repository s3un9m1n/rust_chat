use ctrlc;
use futures_util::{SinkExt, StreamExt};
use std::process;
use tokio::io::{self, AsyncBufReadExt};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use serde_json::Value;

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
    tokio::spawn(async move {
        let stdin = io::BufReader::new(io::stdin());
        let mut lines = stdin.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                if tx.send(line).await.is_err() {
                    break; // 채널이 닫힌 경우 루프 종료
                }
            }
        }
    });

    loop {
        // 두 개 이상의 future 중 먼저 완료되는 future 값을 return 해줌
        tokio::select! {
            // 사용자 입력을 처리
            Some(message) = rx.recv() => {
                if !message.trim().is_empty() {
                    ws_stream
                        .send(Message::Text(message.clone()))
                        .await
                        .expect("Failed to send message");
                    println!("Sent message: {}", message);
                }
            }

            // 서버로부터 메시지 수신
            Some(Ok(msg)) = ws_stream.next() => {
                match msg {
                    Message::Text(message) => {
                        if let Ok(received) = serde_json::from_str::<Value>(&message) {
                            if let (Some(id), Some(text)) = (received.get("id"), received.get("text")) {
                                println!("Received message. (FROM){}, (MSG){}",
                                    id.as_str().unwrap_or("unknown"),
                                    text.as_str().unwrap_or(""));
                            } else {
                                println!("Invalid message format: {}", message);
                            }
                        } else {
                            println!("Failed to parse message: {}", message);
                        }
                    }
                    Message::Close(_) => {
                        println!("Server closed the connection");
                        break;
                    }
                    _ => println!("Unexpected message: {:?}", msg),
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
