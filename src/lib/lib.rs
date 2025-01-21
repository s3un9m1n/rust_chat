use futures_util::SinkExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;

pub type ClientsMap = Arc<
    Mutex<
        HashMap<
            String,
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
                Message,
            >,
        >,
    >,
>;

pub async fn broadcast_message(message: &str, clients: &ClientsMap) {
    let mut clients = clients.lock().await; // 가변 참조를 얻습니다.
    for (_, write) in clients.iter_mut() {
        let _ = write.send(Message::Text(message.to_string())).await; // 가변 참조를 사용하여 send 호출
    }
}
