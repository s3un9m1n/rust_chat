use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    println!("Server listening on {}", addr);

    let clients: std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<
                String,
                futures_util::stream::SplitSink<
                    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
                    tokio_tungstenite::tungstenite::protocol::Message,
                >,
            >,
        >,
    > = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let clients = clients.clone();
        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    handle_client(ws_stream, clients).await;
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {:?}", e);
                }
            }
        });
    }
}

async fn handle_client(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    clients: std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<
                String,
                futures_util::stream::SplitSink<
                    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
                    tokio_tungstenite::tungstenite::protocol::Message,
                >,
            >,
        >,
    >,
) {
    let client_id = Uuid::new_v4().to_string();
    let (write, mut read) = ws_stream.split();

    // Add the new client to the clients map
    clients.lock().await.insert(client_id.clone(), write);

    println!("Client connected: {}", client_id);

    // Broadcast a join message
    broadcast_message(
        &format!("{{\"type\":\"join\",\"user\":\"{}\"}}", client_id),
        &clients,
    )
    .await;

    while let Some(Ok(message)) = read.next().await {
        if let tokio_tungstenite::tungstenite::protocol::Message::Text(text) = message {
            println!("Message received from {}: {}", client_id, text);
            broadcast_message(&text, &clients).await;
        }
    }

    // Remove the client from the clients map
    clients.lock().await.remove(&client_id);

    // Broadcast a leave message
    broadcast_message(
        &format!("{{\"type\":\"leave\",\"user\":\"{}\"}}", client_id),
        &clients,
    )
    .await;

    println!("Client disconnected: {}", client_id);
}

async fn broadcast_message(
    message: &str,
    clients: &std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<
                String,
                futures_util::stream::SplitSink<
                    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
                    tokio_tungstenite::tungstenite::protocol::Message,
                >,
            >,
        >,
    >,
) {
    let mut clients = clients.lock().await;
    for (_, write) in clients.iter_mut() {
        let _ = write
            .send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                message.to_string(),
            ))
            .await;
    }
}
