#[tokio::main]
async fn main() -> anyhow::Result<()> {
    timekeeper_backend::platform::runtime::run().await
}
