use timekeeper_backend::{config::Config, utils::cookies::SameSite};

#[test]
fn loads_time_zone_string_without_db() {
    // Ensure Config struct stays default-constructible via explicit values.
    let cfg = Config {
        database_url: "postgres://example".into(),
        read_database_url: None,
        jwt_secret: "secret".into(),
        jwt_expiration_hours: 1,
        refresh_token_expiration_days: 1,
        max_concurrent_sessions: 3,
        audit_log_retention_days: 1825,
        audit_log_retention_forever: false,
        consent_log_retention_days: 1825,
        consent_log_retention_forever: false,
        aws_region: "ap-northeast-1".into(),
        aws_kms_key_id: "alias/timekeeper-test".into(),
        aws_audit_log_bucket: "timekeeper-audit-logs".into(),
        aws_cloudtrail_enabled: true,
        cookie_secure: false,
        cookie_same_site: SameSite::Lax,
        cors_allow_origins: vec!["http://localhost:8000".into()],
        time_zone: chrono_tz::Asia::Tokyo,
        mfa_issuer: "Timekeeper".into(),
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
    };
    assert_eq!(cfg.time_zone, chrono_tz::Asia::Tokyo);
}
