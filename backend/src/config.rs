use anyhow::anyhow;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::env;

use crate::utils::cookies::SameSite;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub read_database_url: Option<String>,
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub refresh_token_expiration_days: u64,
    pub audit_log_retention_days: i64,
    pub audit_log_retention_forever: bool,
    pub consent_log_retention_days: i64,
    pub consent_log_retention_forever: bool,
    pub aws_region: String,
    pub aws_kms_key_id: String,
    pub aws_audit_log_bucket: String,
    pub aws_cloudtrail_enabled: bool,
    pub cookie_secure: bool,
    pub cookie_same_site: SameSite,
    pub cors_allow_origins: Vec<String>,
    pub time_zone: Tz,
    pub mfa_issuer: String,
    pub rate_limit_ip_max_requests: u32,
    pub rate_limit_ip_window_seconds: u64,
    pub rate_limit_user_max_requests: u32,
    pub rate_limit_user_window_seconds: u64,
    pub redis_url: Option<String>,
    pub redis_pool_size: u32,
    pub redis_connect_timeout: u64,
    pub feature_redis_cache_enabled: bool,
    pub feature_read_replica_enabled: bool,
    pub password_min_length: usize,
    pub password_require_uppercase: bool,
    pub password_require_lowercase: bool,
    pub password_require_numbers: bool,
    pub password_require_symbols: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditLogRetentionPolicy {
    Disabled,
    Forever,
    Days(i64),
}

impl AuditLogRetentionPolicy {
    pub fn is_recording_enabled(&self) -> bool {
        !matches!(self, AuditLogRetentionPolicy::Disabled)
    }

    pub fn retention_days(&self) -> Option<i64> {
        match self {
            AuditLogRetentionPolicy::Days(days) => Some(*days),
            _ => None,
        }
    }

    pub fn cleanup_cutoff(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.retention_days()
            .map(|days| now - ChronoDuration::days(days))
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://timekeeper:timekeeper@localhost:5432/timekeeper".to_string()
        });

        let read_database_url = env::var("READ_DATABASE_URL").ok();

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| anyhow!("JWT_SECRET must be set and at least 32 characters long"))?;
        if jwt_secret.len() < 32 {
            return Err(anyhow!(
                "JWT_SECRET must be at least 32 characters long (current length: {})",
                jwt_secret.len()
            ));
        }

        let jwt_expiration_hours = env::var("JWT_EXPIRATION_HOURS")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        let refresh_token_expiration_days = env::var("REFRESH_TOKEN_EXPIRATION_DAYS")
            .unwrap_or_else(|_| "7".to_string())
            .parse()
            .unwrap_or(7);

        let audit_log_retention_days = env::var("AUDIT_LOG_RETENTION_DAYS")
            .unwrap_or_else(|_| "1825".to_string())
            .parse::<i64>()
            .unwrap_or(1825)
            .max(0);

        let audit_log_retention_forever = env::var("AUDIT_LOG_RETENTION_FOREVER")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let consent_log_retention_days = env::var("CONSENT_LOG_RETENTION_DAYS")
            .unwrap_or_else(|_| "1825".to_string())
            .parse::<i64>()
            .unwrap_or(1825)
            .max(0);

        let consent_log_retention_forever = env::var("CONSENT_LOG_RETENTION_FOREVER")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let aws_region = env::var("AWS_REGION").unwrap_or_else(|_| "ap-northeast-1".to_string());
        let aws_kms_key_id = env::var("AWS_KMS_KEY_ID").unwrap_or_default();
        let aws_audit_log_bucket = env::var("AWS_AUDIT_LOG_BUCKET").unwrap_or_default();
        let aws_cloudtrail_enabled = env::var("AWS_CLOUDTRAIL_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let cookie_secure = env::var("COOKIE_SECURE")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let cookie_same_site =
            parse_same_site(env::var("COOKIE_SAMESITE").unwrap_or_else(|_| "Lax".to_string()))?;

        let cors_allow_origins = env::var("CORS_ALLOW_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:8000".to_string())
            .split(',')
            .map(|origin| origin.trim().to_string())
            .filter(|origin| !origin.is_empty())
            .collect::<Vec<_>>();

        let time_zone_name = env::var("APP_TIMEZONE").unwrap_or_else(|_| "UTC".to_string());
        let time_zone: Tz = time_zone_name
            .parse()
            .map_err(|_| anyhow!("Invalid APP_TIMEZONE value: {}", time_zone_name))?;

        let mfa_issuer = env::var("MFA_ISSUER").unwrap_or_else(|_| "Timekeeper".to_string());

        let rate_limit_ip_max_requests = env::var("RATE_LIMIT_IP_MAX_REQUESTS")
            .unwrap_or_else(|_| "15".to_string())
            .parse()
            .unwrap_or(15);

        let rate_limit_ip_window_seconds = env::var("RATE_LIMIT_IP_WINDOW_SECONDS")
            .unwrap_or_else(|_| "900".to_string())
            .parse()
            .unwrap_or(900);

        let rate_limit_user_max_requests = env::var("RATE_LIMIT_USER_MAX_REQUESTS")
            .unwrap_or_else(|_| "20".to_string())
            .parse()
            .unwrap_or(20);

        let rate_limit_user_window_seconds = env::var("RATE_LIMIT_USER_WINDOW_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .unwrap_or(3600);

        let redis_url = env::var("REDIS_URL").ok();
        let redis_pool_size = env::var("REDIS_POOL_SIZE")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10);
        let redis_connect_timeout = env::var("REDIS_CONNECT_TIMEOUT")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        let feature_redis_cache_enabled = env::var("FEATURE_REDIS_CACHE_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let feature_read_replica_enabled = env::var("FEATURE_READ_REPLICA_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let password_min_length = env::var("PASSWORD_MIN_LENGTH")
            .unwrap_or_else(|_| "12".to_string())
            .parse()
            .unwrap_or(12);

        let password_require_uppercase = env::var("PASSWORD_REQUIRE_UPPERCASE")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let password_require_lowercase = env::var("PASSWORD_REQUIRE_LOWERCASE")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let password_require_numbers = env::var("PASSWORD_REQUIRE_NUMBERS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let password_require_symbols = env::var("PASSWORD_REQUIRE_SYMBOLS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        Ok(Self {
            database_url,
            read_database_url,
            jwt_secret,
            jwt_expiration_hours,
            refresh_token_expiration_days,
            audit_log_retention_days,
            audit_log_retention_forever,
            consent_log_retention_days,
            consent_log_retention_forever,
            aws_region,
            aws_kms_key_id,
            aws_audit_log_bucket,
            aws_cloudtrail_enabled,
            cookie_secure,
            cookie_same_site,
            cors_allow_origins,
            time_zone,
            mfa_issuer,
            rate_limit_ip_max_requests,
            rate_limit_ip_window_seconds,
            rate_limit_user_max_requests,
            rate_limit_user_window_seconds,
            redis_url,
            redis_pool_size,
            redis_connect_timeout,
            feature_redis_cache_enabled,
            feature_read_replica_enabled,
            password_min_length,
            password_require_uppercase,
            password_require_lowercase,
            password_require_numbers,
            password_require_symbols,
        })
    }

    pub fn audit_log_retention_policy(&self) -> AuditLogRetentionPolicy {
        if self.audit_log_retention_forever {
            AuditLogRetentionPolicy::Forever
        } else if self.audit_log_retention_days == 0 {
            AuditLogRetentionPolicy::Disabled
        } else {
            AuditLogRetentionPolicy::Days(self.audit_log_retention_days)
        }
    }

    pub fn consent_log_retention_policy(&self) -> AuditLogRetentionPolicy {
        if self.consent_log_retention_forever {
            AuditLogRetentionPolicy::Forever
        } else if self.consent_log_retention_days == 0 {
            AuditLogRetentionPolicy::Disabled
        } else {
            AuditLogRetentionPolicy::Days(self.consent_log_retention_days)
        }
    }
}

fn parse_same_site(raw: String) -> anyhow::Result<SameSite> {
    match raw.to_ascii_lowercase().as_str() {
        "lax" => Ok(SameSite::Lax),
        "strict" => Ok(SameSite::Strict),
        "none" => Ok(SameSite::None),
        _ => Err(anyhow!("Invalid COOKIE_SAMESITE value: {}", raw)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::UTC;
    use std::sync::{Mutex, OnceLock};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("lock env")
    }

    fn snapshot_env(keys: &[&str]) -> Vec<Option<String>> {
        keys.iter().map(|key| env::var(key).ok()).collect()
    }

    fn restore_env(keys: &[&str], values: Vec<Option<String>>) {
        for (key, value) in keys.iter().zip(values.into_iter()) {
            match value {
                Some(value) => env::set_var(key, value),
                None => env::remove_var(key),
            }
        }
    }

    fn set_optional_aws_env() {
        env::remove_var("AWS_KMS_KEY_ID");
        env::remove_var("AWS_AUDIT_LOG_BUCKET");
    }

    fn base_config() -> Config {
        Config {
            database_url: "postgres://test".to_string(),
            read_database_url: None,
            jwt_secret: "test-jwt-secret-32-chars-minimum!".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            audit_log_retention_days: 1825,
            audit_log_retention_forever: false,
            consent_log_retention_days: 1825,
            consent_log_retention_forever: false,
            aws_region: "ap-northeast-1".to_string(),
            aws_kms_key_id: "alias/timekeeper-test".to_string(),
            aws_audit_log_bucket: "timekeeper-audit-logs".to_string(),
            aws_cloudtrail_enabled: true,
            cookie_secure: false,
            cookie_same_site: SameSite::Lax,
            cors_allow_origins: Vec::new(),
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
        }
    }

    #[test]
    fn config_loads_audit_log_retention_defaults() {
        let _guard = env_guard();
        let keys = [
            "JWT_SECRET",
            "AUDIT_LOG_RETENTION_DAYS",
            "AUDIT_LOG_RETENTION_FOREVER",
            "CONSENT_LOG_RETENTION_DAYS",
            "CONSENT_LOG_RETENTION_FOREVER",
            "AWS_KMS_KEY_ID",
            "AWS_AUDIT_LOG_BUCKET",
            "AWS_REGION",
            "AWS_CLOUDTRAIL_ENABLED",
        ];
        let original = snapshot_env(&keys);

        env::set_var("JWT_SECRET", "a_secure_token_that_is_long_enough_123");
        env::remove_var("AUDIT_LOG_RETENTION_DAYS");
        env::remove_var("AUDIT_LOG_RETENTION_FOREVER");
        env::remove_var("CONSENT_LOG_RETENTION_DAYS");
        env::remove_var("CONSENT_LOG_RETENTION_FOREVER");
        env::remove_var("AWS_REGION");
        env::remove_var("AWS_CLOUDTRAIL_ENABLED");
        set_optional_aws_env();

        let config = Config::load().expect("load config");

        assert_eq!(config.audit_log_retention_days, 1825);
        assert!(!config.audit_log_retention_forever);
        assert_eq!(config.consent_log_retention_days, 1825);
        assert!(!config.consent_log_retention_forever);
        assert_eq!(config.aws_kms_key_id, "");
        assert_eq!(config.aws_audit_log_bucket, "");

        restore_env(&keys, original);
    }

    #[test]
    fn config_loads_aws_defaults() {
        let _guard = env_guard();
        let keys = [
            "JWT_SECRET",
            "AWS_KMS_KEY_ID",
            "AWS_AUDIT_LOG_BUCKET",
            "AWS_REGION",
            "AWS_CLOUDTRAIL_ENABLED",
            "CONSENT_LOG_RETENTION_DAYS",
            "CONSENT_LOG_RETENTION_FOREVER",
        ];
        let original = snapshot_env(&keys);

        env::set_var("JWT_SECRET", "a_secure_token_that_is_long_enough_123");
        env::remove_var("AWS_REGION");
        env::remove_var("AWS_CLOUDTRAIL_ENABLED");
        set_optional_aws_env();

        let config = Config::load().expect("load config");

        assert_eq!(config.aws_region, "ap-northeast-1");
        assert!(config.aws_cloudtrail_enabled);
        assert_eq!(config.aws_kms_key_id, "");
        assert_eq!(config.aws_audit_log_bucket, "");

        restore_env(&keys, original);
    }

    #[test]
    fn config_aws_kms_key_id_is_optional() {
        let _guard = env_guard();
        let keys = ["JWT_SECRET", "AWS_KMS_KEY_ID", "AWS_AUDIT_LOG_BUCKET"];
        let original = snapshot_env(&keys);

        env::set_var("JWT_SECRET", "a_secure_token_that_is_long_enough_123");
        env::remove_var("AWS_KMS_KEY_ID");
        env::set_var("AWS_AUDIT_LOG_BUCKET", "timekeeper-audit-logs");

        let config = Config::load().expect("config should load without kms key");
        assert_eq!(config.aws_kms_key_id, "");
        assert_eq!(config.aws_audit_log_bucket, "timekeeper-audit-logs");

        restore_env(&keys, original);
    }

    #[test]
    fn config_aws_audit_log_bucket_is_optional() {
        let _guard = env_guard();
        let keys = ["JWT_SECRET", "AWS_KMS_KEY_ID", "AWS_AUDIT_LOG_BUCKET"];
        let original = snapshot_env(&keys);

        env::set_var("JWT_SECRET", "a_secure_token_that_is_long_enough_123");
        env::set_var("AWS_KMS_KEY_ID", "alias/timekeeper-test");
        env::remove_var("AWS_AUDIT_LOG_BUCKET");

        let config = Config::load().expect("config should load without audit bucket");
        assert_eq!(config.aws_kms_key_id, "alias/timekeeper-test");
        assert_eq!(config.aws_audit_log_bucket, "");

        restore_env(&keys, original);
    }

    #[test]
    fn audit_log_retention_policy_prioritizes_forever() {
        let mut config = base_config();
        config.audit_log_retention_days = 0;
        config.audit_log_retention_forever = true;

        let policy = config.audit_log_retention_policy();

        assert_eq!(policy, AuditLogRetentionPolicy::Forever);
        assert!(policy.is_recording_enabled());
    }

    #[test]
    fn audit_log_retention_policy_disables_when_days_zero() {
        let mut config = base_config();
        config.audit_log_retention_days = 0;
        config.audit_log_retention_forever = false;

        let policy = config.audit_log_retention_policy();

        assert_eq!(policy, AuditLogRetentionPolicy::Disabled);
        assert!(!policy.is_recording_enabled());
    }

    #[test]
    fn audit_log_retention_policy_returns_days() {
        let mut config = base_config();
        config.audit_log_retention_days = 30;
        config.audit_log_retention_forever = false;

        let policy = config.audit_log_retention_policy();

        assert_eq!(policy, AuditLogRetentionPolicy::Days(30));
        assert_eq!(policy.retention_days(), Some(30));
    }
}
