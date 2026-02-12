use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use base32::Alphabet::RFC4648;
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use totp_rs::{Algorithm, TOTP};

use crate::config::Config;

const SECRET_BYTE_LENGTH: usize = 20;
const CODE_DIGITS: usize = 6;
const STEP_SECONDS: u64 = 30;
const ALLOWED_SKEW: u8 = 1;
const NONCE_LENGTH: usize = 12;
const ENCRYPTED_PREFIX: &str = "enc:v1";

/// Generates a random base32-encoded secret suitable for RFC6238 TOTP.
pub fn generate_totp_secret() -> String {
    let mut bytes = [0u8; SECRET_BYTE_LENGTH];
    OsRng.fill_bytes(&mut bytes);
    base32::encode(RFC4648 { padding: false }, &bytes)
}

/// Produces an `otpauth://` URI that OTP clients like Authy can import.
pub fn generate_otpauth_uri(issuer: &str, account_name: &str, secret: &str) -> Result<String> {
    if issuer.contains(':') {
        return Err(anyhow!("Issuer must not contain ':'"));
    }
    let sanitized_account = account_name.trim();
    if sanitized_account.contains(':') {
        return Err(anyhow!("Account name must not contain ':'"));
    }
    let totp = build_totp_with_labels(secret, Some(issuer), sanitized_account)?;
    Ok(totp.get_url())
}

/// Validates the submitted TOTP code against the stored secret.
pub fn verify_totp_code(secret: &str, code: &str) -> Result<bool> {
    let sanitized_code = code.trim();
    if sanitized_code.len() != CODE_DIGITS || !sanitized_code.chars().all(|c| c.is_ascii_digit()) {
        return Ok(false);
    }
    let totp = build_totp(secret)?;
    totp.check_current(sanitized_code)
        .map_err(|e| anyhow!("Failed to verify TOTP code: {}", e))
}

/// Encrypts a TOTP secret before persisting it.
pub fn protect_totp_secret(secret: &str, config: &Config) -> Result<String> {
    let mut nonce_bytes = [0u8; NONCE_LENGTH];
    OsRng.fill_bytes(&mut nonce_bytes);

    let key = derive_mfa_key(config);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow!("Invalid MFA key"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, secret.as_bytes())
        .map_err(|_| anyhow!("Failed to encrypt MFA secret"))?;

    Ok(format!(
        "{}:{}:{}",
        ENCRYPTED_PREFIX,
        STANDARD_NO_PAD.encode(nonce_bytes),
        STANDARD_NO_PAD.encode(ciphertext)
    ))
}

/// Decrypts a stored TOTP secret.
/// For backward compatibility, non-prefixed values are treated as legacy plain text.
pub fn recover_totp_secret(stored: &str, config: &Config) -> Result<String> {
    let mut parts = stored.splitn(3, ':');
    let prefix = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    let remainder = parts.next().unwrap_or_default();

    if prefix != "enc" || version != "v1" || remainder.is_empty() {
        return Ok(stored.to_string());
    }

    let mut payload = remainder.splitn(2, ':');
    let nonce_part = payload.next().unwrap_or_default();
    let cipher_part = payload.next().unwrap_or_default();

    if nonce_part.is_empty() || cipher_part.is_empty() {
        return Err(anyhow!("Invalid encrypted MFA secret format"));
    }

    let nonce_bytes = STANDARD_NO_PAD
        .decode(nonce_part)
        .map_err(|_| anyhow!("Invalid nonce encoding"))?;
    if nonce_bytes.len() != NONCE_LENGTH {
        return Err(anyhow!("Invalid nonce length"));
    }
    let ciphertext = STANDARD_NO_PAD
        .decode(cipher_part)
        .map_err(|_| anyhow!("Invalid ciphertext encoding"))?;

    let key = derive_mfa_key(config);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow!("Invalid MFA key"))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|_| anyhow!("Failed to decrypt MFA secret"))?;

    String::from_utf8(plaintext).map_err(|_| anyhow!("Invalid UTF-8 in decrypted MFA secret"))
}

fn derive_mfa_key(config: &Config) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(config.jwt_secret.as_bytes());
    hasher.update(b"|");
    hasher.update(config.aws_region.as_bytes());
    hasher.update(b"|");
    hasher.update(config.aws_kms_key_id.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

fn build_totp(secret: &str) -> Result<TOTP> {
    build_totp_with_labels(secret, None, "")
}

fn build_totp_with_labels(secret: &str, issuer: Option<&str>, account_name: &str) -> Result<TOTP> {
    let secret_bytes = decode_secret(secret)?;
    TOTP::new(
        Algorithm::SHA1,
        CODE_DIGITS,
        ALLOWED_SKEW,
        STEP_SECONDS,
        secret_bytes,
        issuer.map(|value| value.to_string()),
        account_name.to_string(),
    )
    .map_err(|e| anyhow!("Failed to configure TOTP: {}", e))
}

fn decode_secret(secret: &str) -> Result<Vec<u8>> {
    let cleaned = secret.trim().replace(' ', "").to_uppercase();
    base32::decode(RFC4648 { padding: false }, cleaned.as_str())
        .ok_or_else(|| anyhow!("Invalid base32 secret"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::cookies::SameSite;
    use chrono_tz::UTC;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_config() -> Config {
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
            cookie_same_site: SameSite::Lax,
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
    fn secret_round_trip_verification() {
        let secret = generate_totp_secret();
        let totp = build_totp(&secret).expect("totp build");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_secs();
        let current = totp.generate(now);
        assert!(verify_totp_code(&secret, &current).unwrap());
    }

    #[test]
    fn protect_and_recover_totp_secret_round_trip() {
        let config = test_config();
        let secret = generate_totp_secret();
        let encrypted = protect_totp_secret(&secret, &config).expect("encrypt");
        assert!(encrypted.starts_with("enc:v1:"));

        let decrypted = recover_totp_secret(&encrypted, &config).expect("decrypt");
        assert_eq!(decrypted, secret);
    }

    #[test]
    fn recover_totp_secret_accepts_legacy_plaintext() {
        let config = test_config();
        let secret = generate_totp_secret();
        let recovered = recover_totp_secret(&secret, &config).expect("recover");
        assert_eq!(recovered, secret);
    }
}
