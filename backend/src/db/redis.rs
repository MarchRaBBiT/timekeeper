use crate::config::Config;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use std::time::Duration;

pub type RedisPool = Pool<RedisConnectionManager>;

pub async fn create_redis_pool(config: &Config) -> anyhow::Result<Option<RedisPool>> {
    let Some(url) = &config.redis_url else {
        tracing::info!("Redis URL not set, caching disabled");
        return Ok(None);
    };

    let manager = RedisConnectionManager::new(url.clone())?;
    let pool = Pool::builder()
        .max_size(config.redis_pool_size)
        .connection_timeout(Duration::from_secs(config.redis_connect_timeout))
        .build(manager)
        .await?;

    tracing::info!(
        "Redis connection pool created (size: {})",
        config.redis_pool_size
    );
    Ok(Some(pool))
}
