use std::time::Duration;

use timekeeper_backend::{
    config::Config,
    db::{connection::create_pool, redis::create_redis_pool},
    services::lockout_notification_worker::work_once,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let run_once = std::env::args().skip(1).any(|arg| arg == "--once");
    let config = Config::load()?;
    let pool = create_pool(&config.database_url).await?;
    let redis_pool = create_redis_pool(&config).await?.ok_or_else(|| {
        anyhow::anyhow!("REDIS_URL must be configured for lockout_notification_worker")
    })?;

    loop {
        match work_once(&pool, &redis_pool, &config).await? {
            Some(outcome) => tracing::info!(?outcome, "Processed lockout notification job"),
            None if run_once => break,
            None => tokio::time::sleep(Duration::from_millis(250)).await,
        }
    }

    Ok(())
}
