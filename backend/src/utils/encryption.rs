use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::utils::kms::{active_provider, provider_by_id, KmsEnvelope};

pub fn encrypt_pii(plaintext: &str, config: &Config) -> Result<String> {
    let provider = active_provider(config);
    let envelope = provider.encrypt(plaintext.as_bytes())?;
    envelope.encode()
}

pub fn decrypt_pii(stored: &str, config: &Config) -> Result<String> {
    let Some(envelope) = KmsEnvelope::parse(stored)? else {
        return Ok(stored.to_string());
    };

    let provider = provider_by_id(&envelope.provider_id, config)?;
    let plaintext = provider.decrypt(&envelope.nonce, &envelope.ciphertext)?;
    String::from_utf8(plaintext).map_err(|_| anyhow!("Decrypted data is not UTF-8"))
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

pub fn hash_email(email: &str, config: &Config) -> String {
    let normalized = normalize_email(email);
    let mut hasher = Sha256::new();
    hasher.update(config.jwt_secret.as_bytes());
    hasher.update(b"|");
    hasher.update(normalized.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::UTC;

    fn config_stub() -> Config {
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
            account_lockout_threshold: 5,
            account_lockout_duration_minutes: 15,
            account_lockout_backoff_enabled: true,
            account_lockout_max_duration_hours: 24,
            production_mode: false,
        }
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let config = config_stub();
        let plain = "Alice Example";
        let encrypted = encrypt_pii(plain, &config).expect("encrypt");
        assert!(encrypted.starts_with("kms:v1:pseudo:"));
        let decrypted = decrypt_pii(&encrypted, &config).expect("decrypt");
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn decrypt_plaintext_for_backward_compatibility() {
        let config = config_stub();
        let plain = "legacy";
        let decrypted = decrypt_pii(plain, &config).expect("fallback");
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn decrypt_legacy_kms_envelope_without_provider_id() {
        let config = config_stub();
        let plain = "legacy-kms-envelope";
        let encrypted = encrypt_pii(plain, &config).expect("encrypt");
        let legacy = encrypted.replacen("kms:v1:pseudo:", "kms:v1:", 1);
        let decrypted = decrypt_pii(&legacy, &config).expect("decrypt legacy");
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn hash_email_normalizes_case_and_whitespace() {
        let config = config_stub();
        let a = hash_email(" Alice@Example.com ", &config);
        let b = hash_email("alice@example.com", &config);
        assert_eq!(a, b);
    }
}
