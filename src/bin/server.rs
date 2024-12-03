use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};

#[tokio::main]
async fn main() {
    // 서버 주소 설정 (localhost:8080)
    let addr = "127.0.0.1:8080".to_string();
    let listener = TcpListener::bind(&addr).await.unwrap();

    println!("WebSocket server listening on {}", addr);

    // 클라이언트 연결 대기
    while let Ok((stream, _)) = listener.accept().await {
        // 각 클라이언트마다 비동기적으로 처리
        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(stream: tokio::net::TcpStream) {
    // WebSocket 핸드쉐이크 수행
    let ws_stream = accept_async(stream)
        .await
        .expect("Error during WebSocket handshake");

    println!("New WebSocket connection");

    // WebSocket 스트림에서 메시지 받기
    let (mut write, mut read) = ws_stream.split();

    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                println!("Received message: {}", text);
                // 받은 메시지를 다시 클라이언트로 전송
                if let Err(e) = write.send(Message::Text(text)).await {
                    eprintln!("Error sending message: {}", e);
                    return;
                }
            }
            Ok(Message::Binary(_)) => {
                // 바이너리 메시지는 처리하지 않음
                println!("Received binary message");
            }
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                return;
            }
            _ => {}
        }
    }
}
