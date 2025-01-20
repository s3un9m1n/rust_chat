use futures_util::Sink;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;

pub type ClientsMap = Arc<
    Mutex<
        std::collections::HashMap<
            String,
            Box<dyn Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Send>,
        >,
    >,
>;

pub async fn broadcast_message(message: &str, clients: &ClientsMap) {
    let clients = clients.lock().await;

    for (_, client) in clients.iter() {
        let _ = client.send(Message::Text(message.to_string())).await;
    }
}
