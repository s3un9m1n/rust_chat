use project::server::handler;
use project::server::message;

#[tokio::main]
async fn main() {
    let hello_message = message::create_hello_message("client_id");
    println!("Created hello message: {}", hello_message);

    let incoming_message = r#"{\"type\":\"chat\",\"text\":\"Hello, server!\"}"#;
    handler::handle_incoming_message(incoming_message);
}
