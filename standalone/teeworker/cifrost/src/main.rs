#[tokio::main]
async fn main() {
    ces_sanitized_logger::init_subscriber(false);
    cifrost::run().await;
}
