use crate::{
    config::Config, db::connection::DbPool, db::redis::RedisPool,
    services::token_cache::TokenCacheServiceTrait,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub write_pool: DbPool,
    pub read_pool: Option<DbPool>,
    pub redis_pool: Option<RedisPool>,
    pub token_cache: Option<Arc<dyn TokenCacheServiceTrait>>,
    pub config: Config,
}

impl AppState {
    pub fn new(
        write_pool: DbPool,
        read_pool: Option<DbPool>,
        redis_pool: Option<RedisPool>,
        token_cache: Option<Arc<dyn TokenCacheServiceTrait>>,
        config: Config,
    ) -> Self {
        Self {
            write_pool,
            read_pool,
            redis_pool,
            token_cache,
            config,
        }
    }

    /// Returns the read pool if configured, otherwise falls back to the write pool.
    /// Use this for SELECT queries that don't require read-after-write consistency.
    pub fn read_pool(&self) -> &DbPool {
        if self.config.feature_read_replica_enabled {
            self.read_pool.as_ref().unwrap_or(&self.write_pool)
        } else {
            &self.write_pool
        }
    }

    /// Returns the Redis pool if configured and enabled.
    pub fn redis(&self) -> Option<&RedisPool> {
        if self.config.feature_redis_cache_enabled {
            self.redis_pool.as_ref()
        } else {
            None
        }
    }

    /// For backward compatibility: returns (write_pool, config) tuple.
    /// Deprecated - use AppState directly instead.
    pub fn into_parts(self) -> (DbPool, Config) {
        (self.write_pool, self.config)
    }

    /// For backward compatibility: returns (write_pool, config) tuple.
    /// Deprecated - use AppState directly instead.
    pub fn as_tuple(&self) -> (DbPool, Config) {
        (self.write_pool.clone(), self.config.clone())
    }
}

impl From<AppState> for (DbPool, Config) {
    fn from(state: AppState) -> Self {
        state.into_parts()
    }
}

impl From<(DbPool, Config)> for AppState {
    fn from((db_pool, config): (DbPool, Config)) -> Self {
        Self::new(db_pool, None, None, None, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::UTC;
    use sqlx::postgres::PgPoolOptions;

    fn base_config() -> Config {
        Config {
            database_url: "postgres://test".to_string(),
            read_database_url: None,
            jwt_secret: "a_secure_token_that_is_long_enough_123".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            max_concurrent_sessions: 3,
            audit_log_retention_days: 1825,
            audit_log_retention_forever: false,
            consent_log_retention_days: 1825,
            consent_log_retention_forever: false,
            aws_region: "ap-northeast-1".to_string(),
            aws_kms_key_id: "alias/timekeeper-test".to_string(),
            aws_audit_log_bucket: "timekeeper-audit-logs".to_string(),
            aws_cloudtrail_enabled: true,
            cookie_secure: false,
            cookie_same_site: crate::utils::cookies::SameSite::Lax,
            cors_allow_origins: vec!["http://localhost:8000".to_string()],
            time_zone: UTC,
            mfa_issuer: "Timekeeper".to_string(),
            rate_limit_ip_max_requests: 15,
            rate_limit_ip_window_seconds: 900,
            rate_limit_user_max_requests: 20,
            rate_limit_user_window_seconds: 3600,
            redis_url: None,
            redis_pool_size: 10,
            redis_connect_timeout: 5,
            feature_redis_cache_enabled: true,
            feature_read_replica_enabled: true,
            password_min_length: 12,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_numbers: true,
            password_require_symbols: true,
            password_expiration_days: 90,
            password_history_count: 5,
            production_mode: false,
        }
    }

    fn lazy_pool(port: u16) -> DbPool {
        PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&format!("postgres://127.0.0.1:{port}/timekeeper"))
            .expect("create lazy pool")
    }

    #[tokio::test]
    async fn read_pool_uses_replica_when_feature_enabled() {
        let write_pool = lazy_pool(15432);
        let read_pool = lazy_pool(25432);
        let state = AppState::new(
            write_pool.clone(),
            Some(read_pool.clone()),
            None,
            None,
            base_config(),
        );

        assert_eq!(
            state.read_pool().connect_options().get_port(),
            read_pool.connect_options().get_port()
        );
    }

    #[tokio::test]
    async fn read_pool_falls_back_to_write_when_replica_disabled_or_missing() {
        let write_pool = lazy_pool(15432);
        let read_pool = lazy_pool(25432);

        let mut config_disabled = base_config();
        config_disabled.feature_read_replica_enabled = false;
        let state_disabled = AppState::new(
            write_pool.clone(),
            Some(read_pool),
            None,
            None,
            config_disabled,
        );
        assert_eq!(
            state_disabled.read_pool().connect_options().get_port(),
            write_pool.connect_options().get_port()
        );

        let state_missing = AppState::new(write_pool.clone(), None, None, None, base_config());
        assert_eq!(
            state_missing.read_pool().connect_options().get_port(),
            write_pool.connect_options().get_port()
        );
    }

    #[tokio::test]
    async fn redis_returns_none_for_disabled_and_missing_pool() {
        let write_pool = lazy_pool(15432);
        let state_enabled = AppState::new(write_pool.clone(), None, None, None, base_config());
        assert!(state_enabled.redis().is_none());

        let mut config_disabled = base_config();
        config_disabled.feature_redis_cache_enabled = false;
        let state_disabled = AppState::new(write_pool, None, None, None, config_disabled);
        assert!(state_disabled.redis().is_none());
    }

    #[tokio::test]
    async fn into_parts_as_tuple_and_from_tuple_roundtrip() {
        let write_pool = lazy_pool(15432);
        let config = base_config();
        let state = AppState::new(write_pool.clone(), None, None, None, config.clone());

        let (pool_from_ref, config_from_ref) = state.as_tuple();
        assert_eq!(
            pool_from_ref.connect_options().get_database(),
            write_pool.connect_options().get_database()
        );
        assert_eq!(config_from_ref.database_url, config.database_url);

        let (pool_from_move, config_from_move) = state.into_parts();
        assert_eq!(
            pool_from_move.connect_options().get_database(),
            write_pool.connect_options().get_database()
        );
        assert_eq!(config_from_move.database_url, config.database_url);

        let rebuilt = AppState::from((write_pool.clone(), config.clone()));
        assert_eq!(
            rebuilt.write_pool.connect_options().get_database(),
            write_pool.connect_options().get_database()
        );
        assert_eq!(rebuilt.config.database_url, config.database_url);
    }
}
