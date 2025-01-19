use futures_util::{SinkExt, StreamExt};
use tokio::io::{self, AsyncBufReadExt};
use tokio_tungstenite::connect_async;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let client_id = Uuid::new_v4().to_string();
    let url = "ws://127.0.0.1:8080";

    let (ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to server");
    println!("Connected to server with ID: {}", client_id);

    let (mut write, mut read) = ws_stream.split();

    // Spawn a task to handle incoming messages from the server
    tokio::spawn(async move {
        while let Some(Ok(message)) = read.next().await {
            if let tokio_tungstenite::tungstenite::protocol::Message::Text(text) = message {
                handle_server_message(&text);
            }
        }
    });

    // Handle user input
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if !line.trim().is_empty() {
            let message = format!(
                "{{\"type\":\"chat\",\"sender\":\"{}\",\"message\":\"{}\"}}",
                client_id,
                line.trim()
            );
            write
                .send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                    message,
                ))
                .await
                .expect("Failed to send message");
        }
    }
}

fn handle_server_message(message: &str) {
    use serde_json::Value;

    match serde_json::from_str::<Value>(message) {
        Ok(json) => {
            if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                match msg_type {
                    "join" => {
                        if let Some(user) = json.get("user").and_then(|v| v.as_str()) {
                            println!("[SERVER]: User {} has joined the chat", user);
                        }
                    }
                    "leave" => {
                        if let Some(user) = json.get("user").and_then(|v| v.as_str()) {
                            println!("[SERVER]: User {} has left the chat", user);
                        }
                    }
                    "chat" => {
                        if let Some(sender) = json.get("sender").and_then(|v| v.as_str()) {
                            if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                                println!("[{}]: {}", sender, msg);
                            }
                        }
                    }
                    _ => {
                        println!("[SERVER]: Unknown message type");
                    }
                }
            } else {
                println!("[SERVER]: Invalid message format");
            }
        }
        Err(_) => println!("[SERVER]: Failed to parse message"),
    }
}
