use futures_util::SinkExt;
use project::client::message;
use tokio::io::AsyncBufReadExt;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
    let url = "ws://127.0.0.1:8080";
    let (ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to server");

    println!("Connected to server");

    process_client(ws_stream).await;
}

async fn process_client(
    mut ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) {
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if !line.trim().is_empty() {
            let message = message::create_chat_message("client_id", &line);
            ws_stream
                .send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                    message,
                ))
                .await
                .expect("Failed to send message");
        }
    }
}
