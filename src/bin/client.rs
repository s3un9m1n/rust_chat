use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::io::{self, AsyncBufReadExt};
use tokio::signal;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::Error;

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8080";

    // (작업#1) 웹소켓 서버 연결
    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Connected to the server!");

    // 비동기 입력을 처리하기 위한 채널
    let (tx, mut rx) = mpsc::channel(32);

    // (작업#2) 사용자 입력을 읽는 태스크
    tokio::spawn(read_user_input(tx));

    loop {
        // 두 개 이상의 future 중 먼저 완료되는 future 값을 return 해줌
        tokio::select! {
            // (작업#1) 서버로부터 메시지 수신
            Some(message_received) = ws_stream.next() => {
                if let Err(e) = handle_server_message(message_received).await {
                    eprintln!("Error handling server message: {:?}", e);
                    break;
                }
            }
            // (작업#2) 사용자 입력을 처리
            Some(message_input) = rx.recv() => {
                if handle_user_input(&mut ws_stream, message_input).await.is_err() {
                    break;
                }
            }
            // (작업#3) Ctrl+C 처리
            _ = signal::ctrl_c() => {
                // 서버로 종료 메시지 전송
                let exit_message = create_json_message("user_exit", None);
                if let Err(e) = send_to_server(&mut ws_stream, exit_message).await {
                    eprintln!("Failed to send exit message: {:?}", e);
                }

                println!("\nExit program.");
                break;
            }
            // 소켓 스트림이 종료된 경우
            else => {
                println!("Connection closed.");
                break;
            }
        }
    }

    // WebSocket 스트림 종료
    // FIXME: 서버쪽에서 Ctrl+c 입력 시 비정상 종료
    if let Err(e) = ws_stream.close(None).await {
        eprintln!("Error closing WebSocket: {:?}", e);
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
        let exit_message = create_json_message("uesr_exit", None);

        send_to_server(ws_stream, exit_message)
            .await
            .expect("Failed to send exit message");
    }
    // 일반 데이터 전송
    else {
        let chat_message = create_json_message("chat", Some(&message_input));

        send_to_server(ws_stream, chat_message)
            .await
            .expect("Failed to send exit message");

        println!("Sent message: {}", message_input);
    }

    Ok(())
}

async fn handle_server_message(
    message_received: Result<Message, tokio_tungstenite::tungstenite::Error>,
) -> Result<(), ()> {
    match message_received {
        Ok(Message::Text(message_json)) => handle_server_json_message(message_json),
        Ok(Message::Close(_)) => {
            println!("Server closed the connection");
            Err(())
        }
        Err(e) => {
            handle_server_connection_error(e);
            Err(())
        }
        _ => {
            println!("Unknown server message. {:?}", message_received);
            Err(())
        }
    }
}

fn create_json_message(json_type: &str, text: Option<&String>) -> String {
    match text {
        // text 필드 있는 경우
        Some(content) => json!({
            "type": json_type,
            "text": content,
        })
        .to_string(),
        // text 필드 없는 경우
        None => json!({
            "type": json_type,
        })
        .to_string(),
    }
}

async fn send_to_server<S>(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<S>,
    message: String,
) -> Result<(), Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // ws_stream.send()의 결과 처리
    ws_stream.send(Message::Text(message)).await.map_err(|err| {
        eprintln!("Failed to send message. (ERROR){:?}", err);

        err
    })
}

fn handle_server_json_message(message: String) -> Result<(), ()> {
    let json: serde_json::Value = match serde_json::from_str(&message) {
        Ok(json) => json,
        Err(_) => {
            eprintln!("Invalid message format. (MSG){:?}", message);
            return Err(());
        }
    };

    match json.get("type").and_then(|t| t.as_str()) {
        Some("user_joined") => handle_server_user_join(&json),
        Some("user_left") => handle_server_user_left(&json),
        Some("chat") => handle_server_user_chat(&json),
        _ => {
            eprintln!("Unknown message type. (JSON){:?}", json);
            Err(())
        }
    }
}

fn handle_server_connection_error(e: Error) {
    let details = match e {
        tokio_tungstenite::tungstenite::Error::ConnectionClosed => "Connection closed by server.",
        tokio_tungstenite::tungstenite::Error::AlreadyClosed => "The connection is already closed.",
        tokio_tungstenite::tungstenite::Error::Io(_) => {
            "Network error occurred. The server might be down."
        }
        _ => "An unexpected error occurred.",
    };

    println!("Connection error: {:?}\nDetails: {}", e, details);
}

fn handle_server_user_join(json: &serde_json::Value) -> Result<(), ()> {
    let id = match json.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            eprintln!("Invalid message format. (JSON){:?}", json);
            return Err(());
        }
    };

    println!("User join! (ID){}", id);
    Ok(())
}

fn handle_server_user_left(json: &serde_json::Value) -> Result<(), ()> {
    let id = match json.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            eprintln!("Invalid message format. (JSON){:?}", json);
            return Err(());
        }
    };

    println!("User left! (ID){}", id);
    Ok(())
}

fn handle_server_user_chat(json: &serde_json::Value) -> Result<(), ()> {
    let id = match json.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            eprintln!("Invalid JSON format(ID) (JSON){}", json);
            return Err(());
        }
    };

    let text = match json.get("text").and_then(|v| v.as_str()) {
        Some(text) => text,
        None => {
            eprintln!("Invalid JSON format(TEXT) (JSON){}", json);
            return Err(());
        }
    };

    println!("Received message. (FROM){}, (MSG){}", id, text);
    Ok(())
}
