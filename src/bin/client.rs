use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8080";

    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    println!("Connected to the server!");

    let message = "Hello, WebSocket server!";
    ws_stream
        .send(Message::Text(message.to_string()))
        .await
        .expect("Failed to send message");

    println!("Sent message: {}", message);

    let response = ws_stream.next().await;
    match response {
        Some(Ok(Message::Text(response_message))) => {
            println!("Received message: {}", response_message);
        }
        Some(Ok(Message::Close(_))) => {
            println!("Server closed the connection");
        }
        Some(Err(e)) => {
            eprintln!("Error receiving message: {}", e);
        }
        _ => {
            println!("Unexpected message type or no message received");
        }
    }
}
