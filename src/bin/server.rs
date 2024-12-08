use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use std::collections::HashMap;
use std::sync::Arc;

type ClientMap = Arc<RwLock<HashMap<String, futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, Message>>>>;

#[tokio::main]
async fn main() {
    // 서버 주소 설정 (localhost:8080)
    let addr = "127.0.0.1:8080".to_string();
    let listener = TcpListener::bind(&addr).await.unwrap();

    println!("WebSocket server listening on {}", addr);

    // 클라이언트 관리 목록
    // 모든 스레드에서 접근 및 수정이 가능해야 하기 때문에 스마트포인터(`Arc`) 사용
    let clients: ClientMap = Arc::new(RwLock::new(HashMap::new()));

    // TODO: `id` 값을 특정 값으로 변경
    let mut id = 0;

    // 클라이언트 연결 대기
    while let Ok((stream, _)) = listener.accept().await {
        // 스레드마다 전달하기 위해 스마트포인터 `clone`
        let clients = Arc::clone(&clients);

        // 각 클라이언트마다 비동기적으로 처리
        tokio::spawn(handle_connection(stream, id.to_string(), clients));

        id += 1;
    }
}

async fn handle_connection(stream: tokio::net::TcpStream, client_id: String, clients: ClientMap) {
    // WebSocket 핸드쉐이크 수행
    let ws_stream = accept_async(stream)
        .await
        .expect("Error during WebSocket handshake");

    println!("New WebSocket connection");

    // WebSocket 스트림 분리
    let (write, mut read) = ws_stream.split();

    // `write` 스트림을 클라이언트 목록에 추가
    {
        let mut clients_lock = clients.write().await;
        clients_lock.insert(client_id.clone(), write);
        println!("Client {} joined", client_id);
    }

    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                println!("Received message: {}", text);

                // read 락 획득
                let mut clients_lock = clients.write().await;

                // 클라이언트 목록에서 현재 클라이언트의 `sender` 획득
                // TODO: 현재 id에 해당하는 클라이언트 -> 전체 id 브로드캐스트
                if let Some(client) = clients_lock.get_mut(&client_id) {
                    // 받은 메시지를 다시 클라이언트로 전송
                    if let Err(e) = client.send(Message::Text(text)).await {
                        eprintln!("Error sending message: {}", e);
                        break;
                    }
                }
            }
            Ok(Message::Binary(_)) => {
                // 바이너리 메시지는 처리하지 않음
                println!("Received binary message");
            }
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                break;
            }
            _ => {}
        }
    }


    // `write` 스트림을 클라이언트 목록에 삭제
    {
        let mut clients_lock = clients.write().await;
        clients_lock.remove(&client_id);
        println!("Client {} disconnected", client_id);
    }
}
