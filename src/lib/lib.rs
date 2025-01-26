use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;

// ClientsMap 타입 정의
pub type ClientsMap = Arc<
    Mutex<
        HashMap<
            String,
            Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
        >,
    >,
>;

// broadcast_message 함수
pub async fn broadcast_message(message: &str, clients: &ClientsMap) {
    let clients_guard = clients.lock().await;
    for (client_id, client) in clients_guard.iter() {
        let mut client = client.lock().await;
        if let Err(e) = client.send(Message::Text(message.to_string())).await {
            eprintln!("Failed to send message to {}: {}", client_id, e);
        }
    }
}
