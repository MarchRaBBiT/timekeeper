use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Type alias so downstream code can reference the logical database pool in a single place.
pub type DbPool = PgPool;

/// Database pool configuration from environment variables
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 600;
const DEFAULT_MAX_LIFETIME_SECS: u64 = 1800;

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            acquire_timeout_secs: 30,
            idle_timeout_secs: DEFAULT_IDLE_TIMEOUT_SECS,
            max_lifetime_secs: DEFAULT_MAX_LIFETIME_SECS,
        }
    }
}

impl PoolConfig {
    pub fn from_env() -> Self {
        Self {
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            min_connections: std::env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            acquire_timeout_secs: std::env::var("DB_ACQUIRE_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            idle_timeout_secs: std::env::var("DB_IDLE_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS),
            max_lifetime_secs: std::env::var("DB_MAX_LIFETIME")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_MAX_LIFETIME_SECS),
        }
    }

    pub fn log(&self) {
        tracing::info!(
            max_connections = self.max_connections,
            min_connections = self.min_connections,
            acquire_timeout_secs = self.acquire_timeout_secs,
            idle_timeout_secs = self.idle_timeout_secs,
            max_lifetime_secs = self.max_lifetime_secs,
            "Database pool configuration"
        );
    }
}

pub async fn create_pool(database_url: &str) -> anyhow::Result<DbPool> {
    create_pool_with_config(database_url, PoolConfig::from_env()).await
}

pub async fn create_pool_with_config(
    database_url: &str,
    config: PoolConfig,
) -> anyhow::Result<DbPool> {
    config.log();

    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
        .connect(database_url)
        .await?;

    Ok(pool)
}
