use chrono::Utc;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    config::{AuditLogRetentionPolicy, Config},
    db::{connection::create_pools, redis::create_redis_pool},
    platform::app::{build_app, AppServices},
    services::{
        audit_log::{AuditLogService, AuditLogServiceTrait},
        consent_log::ConsentLogService,
        holiday::{HolidayService, HolidayServiceTrait},
        holiday_exception::{HolidayExceptionService, HolidayExceptionServiceTrait},
        token_cache::{TokenCacheService, TokenCacheServiceTrait},
    },
    AppState,
};

pub struct ServerRuntime {
    app: axum::Router,
    addr: SocketAddr,
}

impl ServerRuntime {
    pub async fn initialize(config: Config) -> anyhow::Result<Self> {
        log_config(&config);

        let (write_pool, read_pool) =
            create_pools(&config.database_url, config.read_database_url.as_deref()).await?;
        sqlx::migrate!("./migrations").run(&write_pool).await?;

        let audit_log_service: Arc<dyn AuditLogServiceTrait> =
            Arc::new(AuditLogService::new(write_pool.clone()));
        let consent_log_service = Arc::new(ConsentLogService::new(write_pool.clone()));
        let holiday_service: Arc<dyn HolidayServiceTrait> =
            Arc::new(HolidayService::new(write_pool.clone()));
        let holiday_exception_service: Arc<dyn HolidayExceptionServiceTrait> =
            Arc::new(HolidayExceptionService::new(write_pool.clone()));

        let redis_pool = create_redis_pool(&config).await?;
        let token_cache: Option<Arc<dyn TokenCacheServiceTrait>> =
            redis_pool.as_ref().map(|pool| {
                Arc::new(TokenCacheService::new(pool.clone())) as Arc<dyn TokenCacheServiceTrait>
            });

        let state = AppState::new(
            write_pool,
            read_pool,
            redis_pool,
            token_cache,
            config.clone(),
        );
        let services = AppServices {
            audit_log_service: audit_log_service.clone(),
            holiday_service,
            holiday_exception_service,
        };

        spawn_audit_log_cleanup(audit_log_service, config.audit_log_retention_policy());
        spawn_consent_log_cleanup(consent_log_service, config.consent_log_retention_policy());

        Ok(Self {
            app: build_app(state, services),
            addr: SocketAddr::from(([0, 0, 0, 0], 3000)),
        })
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        tracing::info!("Server listening on {}", self.addr);
        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(
            listener,
            self.app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;
        Ok(())
    }
}

pub async fn run() -> anyhow::Result<()> {
    init_tracing();
    let config = Config::load()?;
    let runtime = ServerRuntime::initialize(config).await?;
    runtime.serve().await
}

pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "timekeeper_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

pub fn log_config(config: &Config) {
    tracing::info!(
        "Database URL: {}",
        crate::utils::security::mask_database_url(&config.database_url)
    );
    if let Some(url) = &config.read_database_url {
        tracing::info!(
            "Read Database URL: {}",
            crate::utils::security::mask_database_url(url)
        );
    } else {
        tracing::info!("Read Database URL: None");
    }
    tracing::info!("JWT Expiration: {} hours", config.jwt_expiration_hours);
    tracing::info!("Time Zone: {}", config.time_zone);
    tracing::info!("CORS Allowed Origins: {:?}", config.cors_allow_origins);

    if config.cors_allow_origins.iter().any(|origin| origin == "*") {
        if config.production_mode {
            tracing::error!("SECURITY ERROR: CORS is configured to allow all origins ('*') in PRODUCTION MODE. This is a severe security risk. The server will refuse to start.");
            panic!("Refusing to start due to insecure CORS configuration in production mode.");
        }
        tracing::warn!("SECURITY WARNING: CORS is configured to allow all origins ('*'). This is dangerous for production!");
    }
}

pub fn spawn_audit_log_cleanup(
    audit_log_service: Arc<dyn AuditLogServiceTrait>,
    retention_policy: AuditLogRetentionPolicy,
) {
    if !retention_policy.is_recording_enabled() {
        return;
    }

    tracing::info!(
        retention_days = retention_policy.retention_days(),
        "Starting daily audit log cleanup task"
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            let Some(cutoff) = retention_policy.cleanup_cutoff(Utc::now()) else {
                continue;
            };
            match audit_log_service.delete_logs_before(cutoff).await {
                Ok(deleted) => {
                    tracing::info!(deleted, "Audit log cleanup completed");
                }
                Err(err) => {
                    tracing::warn!(error = ?err, "Audit log cleanup failed");
                }
            }
        }
    });
}

pub fn spawn_consent_log_cleanup(
    consent_log_service: Arc<ConsentLogService>,
    retention_policy: AuditLogRetentionPolicy,
) {
    if !retention_policy.is_recording_enabled() {
        return;
    }

    tracing::info!(
        retention_days = retention_policy.retention_days(),
        "Starting daily consent log cleanup task"
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            let Some(cutoff) = retention_policy.cleanup_cutoff(Utc::now()) else {
                continue;
            };
            match consent_log_service.delete_logs_before(cutoff).await {
                Ok(deleted) => {
                    tracing::info!(deleted, "Consent log cleanup completed");
                }
                Err(err) => {
                    tracing::warn!(error = ?err, "Consent log cleanup failed");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::UTC;
    use sqlx::postgres::PgPoolOptions;

    fn test_config(cors_allow_origins: Vec<String>) -> Config {
        Config {
            database_url: "postgres://test".to_string(),
            read_database_url: None,
            jwt_secret: "test-jwt-secret-32-chars-minimum!".to_string(),
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
            cors_allow_origins,
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
            account_lockout_threshold: 5,
            account_lockout_duration_minutes: 15,
            account_lockout_backoff_enabled: true,
            account_lockout_max_duration_hours: 24,
            production_mode: false,
        }
    }

    #[test]
    #[should_panic(
        expected = "Refusing to start due to insecure CORS configuration in production mode"
    )]
    fn test_production_mode_wildcard_cors_panics() {
        let mut config = test_config(vec!["*".to_string()]);
        config.production_mode = true;
        log_config(&config);
    }

    #[test]
    fn test_production_mode_specific_cors_allows() {
        let mut config = test_config(vec!["https://example.com".to_string()]);
        config.production_mode = true;
        log_config(&config);
    }

    #[test]
    fn test_log_config_with_read_database_and_wildcard_in_non_production() {
        let mut config = test_config(vec!["*".to_string()]);
        config.read_database_url = Some("postgres://read-db".to_string());
        config.production_mode = false;
        log_config(&config);
    }

    #[tokio::test]
    async fn test_spawn_cleanup_skips_when_retention_disabled() {
        let config = test_config(vec!["*".to_string()]);
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&config.database_url)
            .expect("create lazy pool");
        let audit_service: Arc<dyn AuditLogServiceTrait> =
            Arc::new(AuditLogService::new(pool.clone()));
        let consent_service = Arc::new(ConsentLogService::new(pool));

        spawn_audit_log_cleanup(audit_service, AuditLogRetentionPolicy::Disabled);
        spawn_consent_log_cleanup(consent_service, AuditLogRetentionPolicy::Disabled);
    }
}
