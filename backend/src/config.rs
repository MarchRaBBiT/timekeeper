use anyhow::anyhow;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::env;

use crate::utils::cookies::SameSite;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub refresh_token_expiration_days: u64,
    pub audit_log_retention_days: i64,
    pub audit_log_retention_forever: bool,
    pub cookie_secure: bool,
    pub cookie_same_site: SameSite,
    pub cors_allow_origins: Vec<String>,
    pub time_zone: Tz,
    pub mfa_issuer: String,
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
            .unwrap_or_else(|_| "365".to_string())
            .parse::<i64>()
            .unwrap_or(365)
            .max(0);

        let audit_log_retention_forever = env::var("AUDIT_LOG_RETENTION_FOREVER")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

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

        Ok(Config {
            database_url,
            jwt_secret,
            jwt_expiration_hours,
            refresh_token_expiration_days,
            audit_log_retention_days,
            audit_log_retention_forever,
            cookie_secure,
            cookie_same_site,
            cors_allow_origins,
            time_zone,
            mfa_issuer,
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

    fn base_config() -> Config {
        Config {
            database_url: "postgres://test".to_string(),
            jwt_secret: "test-jwt-secret-32-chars-minimum!".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            audit_log_retention_days: 365,
            audit_log_retention_forever: false,
            cookie_secure: false,
            cookie_same_site: SameSite::Lax,
            cors_allow_origins: Vec::new(),
            time_zone: UTC,
            mfa_issuer: "Timekeeper".to_string(),
        }
    }

    #[test]
    fn config_loads_audit_log_retention_defaults() {
        let _guard = env_guard();
        let keys = [
            "JWT_SECRET",
            "AUDIT_LOG_RETENTION_DAYS",
            "AUDIT_LOG_RETENTION_FOREVER",
        ];
        let original = snapshot_env(&keys);

        env::set_var("JWT_SECRET", "a_secure_token_that_is_long_enough_123");
        env::remove_var("AUDIT_LOG_RETENTION_DAYS");
        env::remove_var("AUDIT_LOG_RETENTION_FOREVER");

        let config = Config::load().expect("load config");

        assert_eq!(config.audit_log_retention_days, 365);
        assert!(!config.audit_log_retention_forever);

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
