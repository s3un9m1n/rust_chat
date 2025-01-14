use project::client::handler;
use project::client::message;

#[tokio::main]
async fn main() {
    let chat_message = message::create_chat_message("client_id", "Hello, server!");
    println!("Created chat message: {}", chat_message);

    let incoming_message = r#"{\"type\":\"chat\",\"text\":\"Hello, client!\"}"#;
    handler::handle_incoming_message(incoming_message);
}
