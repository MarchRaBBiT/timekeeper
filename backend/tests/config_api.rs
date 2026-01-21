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
        feature_read_replica_enabled: true,
    };
    assert_eq!(cfg.time_zone, chrono_tz::Asia::Tokyo);
}
