use futures_util::{SinkExt, StreamExt};
use project::server::message;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    println!("Server listening on {}", addr);

    let _clients: std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<
                String,
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            >,
        >,
    > = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_client(stream));
    }
}

async fn handle_client(stream: tokio::net::TcpStream) {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Failed to accept connection");
    let (mut write, mut read) = ws_stream.split();

    while let Some(Ok(msg)) = read.next().await {
        if let tokio_tungstenite::tungstenite::protocol::Message::Text(text) = msg {
            println!("Received: {}", text);

            let response = message::create_chat_message("server", &text);
            write
                .send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                    response,
                ))
                .await
                .expect("Failed to send message");
        }
    }
}
