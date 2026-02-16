use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

use crate::config::Config;

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    Ok(password_hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid password hash: {}", e))?;

    let argon2 = Argon2::default();
    let result = argon2.verify_password(password.as_bytes(), &parsed_hash);

    match result {
        Ok(_) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(anyhow::anyhow!("Password verification error: {}", e)),
    }
}

pub fn password_matches_any(password: &str, hashes: &[String]) -> anyhow::Result<bool> {
    for hash in hashes {
        if verify_password(password, hash)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn validate_password_complexity(password: &str, config: &Config) -> anyhow::Result<()> {
    if password.len() < config.password_min_length {
        return Err(anyhow::anyhow!(
            "Password must be at least {} characters long",
            config.password_min_length
        ));
    }

    if config.password_require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        return Err(anyhow::anyhow!(
            "Password must contain at least one uppercase letter"
        ));
    }

    if config.password_require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
        return Err(anyhow::anyhow!(
            "Password must contain at least one lowercase letter"
        ));
    }

    if config.password_require_numbers && !password.chars().any(|c| c.is_numeric()) {
        return Err(anyhow::anyhow!("Password must contain at least one number"));
    }

    if config.password_require_symbols && !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err(anyhow::anyhow!("Password must contain at least one symbol"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::cookies::SameSite;
    use chrono_tz::UTC;

    fn test_config() -> Config {
        Config {
            database_url: "".to_string(),
            read_database_url: None,
            jwt_secret: "".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            max_concurrent_sessions: 3,
            audit_log_retention_days: 0,
            audit_log_retention_forever: false,
            audit_log_export_max_rows: 10_000,
            consent_log_retention_days: 0,
            consent_log_retention_forever: false,
            aws_region: "".to_string(),
            aws_kms_key_id: "".to_string(),
            aws_audit_log_bucket: "".to_string(),
            aws_cloudtrail_enabled: false,
            cookie_secure: false,
            cookie_same_site: SameSite::Lax,
            cors_allow_origins: vec![],
            time_zone: UTC,
            mfa_issuer: "".to_string(),
            rate_limit_ip_max_requests: 0,
            rate_limit_ip_window_seconds: 0,
            rate_limit_user_max_requests: 0,
            rate_limit_user_window_seconds: 0,
            redis_url: None,
            redis_pool_size: 0,
            redis_connect_timeout: 0,
            feature_redis_cache_enabled: false,
            feature_read_replica_enabled: false,
            password_min_length: 8,
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
    fn validate_password_complexity_success() {
        let config = test_config();
        assert!(validate_password_complexity("StrongP@ss1", &config).is_ok());
    }

    #[test]
    fn validate_password_complexity_fails_length() {
        let config = test_config();
        let err = validate_password_complexity("Short1!", &config).unwrap_err();
        assert!(err.to_string().contains("at least 8 characters"));
    }

    #[test]
    fn validate_password_complexity_fails_uppercase() {
        let config = test_config();
        let err = validate_password_complexity("weakp@ss1", &config).unwrap_err();
        assert!(err.to_string().contains("uppercase"));
    }

    #[test]
    fn validate_password_complexity_fails_lowercase() {
        let config = test_config();
        let err = validate_password_complexity("WEAKP@SS1", &config).unwrap_err();
        assert!(err.to_string().contains("lowercase"));
    }

    #[test]
    fn validate_password_complexity_fails_numbers() {
        let config = test_config();
        let err = validate_password_complexity("NoNumbers!", &config).unwrap_err();
        assert!(err.to_string().contains("number"));
    }

    #[test]
    fn validate_password_complexity_fails_symbols() {
        let config = test_config();
        let err = validate_password_complexity("NoSymbols1", &config).unwrap_err();
        assert!(err.to_string().contains("symbol"));
    }

    #[test]
    fn hash_and_verify_roundtrip() {
        let pw = "S3cr3t!";
        let hash = hash_password(pw).expect("hash should succeed");
        assert!(verify_password(pw, &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }
}
