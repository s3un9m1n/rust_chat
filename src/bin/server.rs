use futures_util::StreamExt;
use rust_chat::{broadcast_message, ClientsMap};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::main]
async fn main() {
    let clients: ClientsMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind server");

    println!("Server listening on 127.0.0.1:8080");

    while let Ok((stream, _)) = listener.accept().await {
        let clients = clients.clone();
        tokio::spawn(async move {
            if let Ok(ws_stream) = accept_async(stream).await {
                let (write, mut read) = ws_stream.split();
                let client_id = uuid::Uuid::new_v4().to_string();

                clients.lock().await.insert(client_id.clone(), write);

                broadcast_message(
                    &format!("{{\"type\":\"join\",\"user\":\"{}\"}}", client_id),
                    &clients,
                )
                .await;

                while let Some(Ok(Message::Text(text))) = read.next().await {
                    broadcast_message(&text, &clients).await;
                }

                clients.lock().await.remove(&client_id);
                broadcast_message(
                    &format!("{{\"type\":\"leave\",\"user\":\"{}\"}}", client_id),
                    &clients,
                )
                .await;
            }
        });
    }
}
