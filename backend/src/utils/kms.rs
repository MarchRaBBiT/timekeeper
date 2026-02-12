use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::env;

use crate::config::Config;

const NONCE_LENGTH: usize = 12;
const ENVELOPE_SCHEME: &str = "kms";
const ENVELOPE_VERSION: &str = "v1";
const PSEUDO_PROVIDER_ID: &str = "pseudo";
const AWS_PROVIDER_ID: &str = "aws";
const GCP_PROVIDER_ID: &str = "gcp";

pub struct KmsEnvelope {
    pub provider_id: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl KmsEnvelope {
    pub fn encode(&self) -> Result<String> {
        if self.nonce.len() != NONCE_LENGTH {
            return Err(anyhow!("Invalid nonce length"));
        }
        Ok(format!(
            "{}:{}:{}:{}:{}",
            ENVELOPE_SCHEME,
            ENVELOPE_VERSION,
            self.provider_id,
            STANDARD_NO_PAD.encode(&self.nonce),
            STANDARD_NO_PAD.encode(&self.ciphertext)
        ))
    }

    pub fn parse(stored: &str) -> Result<Option<Self>> {
        if !stored.starts_with("kms:v1:") {
            return Ok(None);
        }

        let parts: Vec<&str> = stored.split(':').collect();
        let (provider_id, nonce_part, cipher_part) = match parts.as_slice() {
            // Backward compatibility: kms:v1:<nonce>:<ciphertext>
            ["kms", "v1", nonce, cipher] => (PSEUDO_PROVIDER_ID.to_string(), *nonce, *cipher),
            ["kms", "v1", provider, nonce, cipher] => (provider.to_string(), *nonce, *cipher),
            _ => return Err(anyhow!("Invalid KMS envelope format")),
        };

        let nonce = STANDARD_NO_PAD
            .decode(nonce_part)
            .map_err(|_| anyhow!("Invalid nonce encoding"))?;
        if nonce.len() != NONCE_LENGTH {
            return Err(anyhow!("Invalid nonce length"));
        }

        let ciphertext = STANDARD_NO_PAD
            .decode(cipher_part)
            .map_err(|_| anyhow!("Invalid ciphertext encoding"))?;

        Ok(Some(Self {
            provider_id,
            nonce,
            ciphertext,
        }))
    }
}

pub trait KmsProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn encrypt(&self, plaintext: &[u8]) -> Result<KmsEnvelope>;
    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>>;
}

struct LocalAeadProvider {
    key: [u8; 32],
}

impl LocalAeadProvider {
    fn from_context(config: &Config, context: &[&str]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(config.jwt_secret.as_bytes());
        for part in context {
            hasher.update(b"|");
            hasher.update(part.as_bytes());
        }
        let digest = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&digest);
        Self { key }
    }

    fn encrypt(&self, provider_id: &'static str, plaintext: &[u8]) -> Result<KmsEnvelope> {
        let mut nonce = [0u8; NONCE_LENGTH];
        OsRng.fill_bytes(&mut nonce);

        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| anyhow!("Invalid key"))?;
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|_| anyhow!("Encryption failed"))?;

        Ok(KmsEnvelope {
            provider_id: provider_id.to_string(),
            nonce: nonce.to_vec(),
            ciphertext,
        })
    }

    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if nonce.len() != NONCE_LENGTH {
            return Err(anyhow!("Invalid nonce length"));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| anyhow!("Invalid key"))?;
        cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| anyhow!("Decryption failed"))
    }
}

pub struct PseudoKmsProvider {
    crypto: LocalAeadProvider,
}

impl PseudoKmsProvider {
    pub fn from_config(config: &Config) -> Self {
        let crypto = LocalAeadProvider::from_context(
            config,
            &[
                PSEUDO_PROVIDER_ID,
                &config.aws_region,
                &config.aws_kms_key_id,
            ],
        );
        Self { crypto }
    }
}

impl KmsProvider for PseudoKmsProvider {
    fn provider_id(&self) -> &'static str {
        PSEUDO_PROVIDER_ID
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<KmsEnvelope> {
        self.crypto.encrypt(self.provider_id(), plaintext)
    }

    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        self.crypto.decrypt(nonce, ciphertext)
    }
}

pub struct AwsKmsProvider {
    crypto: LocalAeadProvider,
}

impl AwsKmsProvider {
    pub fn from_config(config: &Config) -> Self {
        let crypto = LocalAeadProvider::from_context(
            config,
            &[AWS_PROVIDER_ID, &config.aws_region, &config.aws_kms_key_id],
        );
        Self { crypto }
    }
}

impl KmsProvider for AwsKmsProvider {
    fn provider_id(&self) -> &'static str {
        AWS_PROVIDER_ID
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<KmsEnvelope> {
        self.crypto.encrypt(self.provider_id(), plaintext)
    }

    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        self.crypto.decrypt(nonce, ciphertext)
    }
}

pub struct GcpKmsProvider {
    crypto: LocalAeadProvider,
}

impl GcpKmsProvider {
    pub fn from_config(config: &Config) -> Self {
        let crypto =
            LocalAeadProvider::from_context(config, &[GCP_PROVIDER_ID, &config.aws_kms_key_id]);
        Self { crypto }
    }
}

impl KmsProvider for GcpKmsProvider {
    fn provider_id(&self) -> &'static str {
        GCP_PROVIDER_ID
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<KmsEnvelope> {
        self.crypto.encrypt(self.provider_id(), plaintext)
    }

    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        self.crypto.decrypt(nonce, ciphertext)
    }
}

pub fn active_provider(config: &Config) -> Box<dyn KmsProvider> {
    match env::var("KMS_PROVIDER").ok().as_deref() {
        Some(AWS_PROVIDER_ID) => Box::new(AwsKmsProvider::from_config(config)),
        Some(GCP_PROVIDER_ID) => Box::new(GcpKmsProvider::from_config(config)),
        _ => Box::new(PseudoKmsProvider::from_config(config)),
    }
}

pub fn provider_by_id(provider_id: &str, config: &Config) -> Result<Box<dyn KmsProvider>> {
    match provider_id {
        PSEUDO_PROVIDER_ID => Ok(Box::new(PseudoKmsProvider::from_config(config))),
        AWS_PROVIDER_ID => Ok(Box::new(AwsKmsProvider::from_config(config))),
        GCP_PROVIDER_ID => Ok(Box::new(GcpKmsProvider::from_config(config))),
        _ => Err(anyhow!("Unsupported KMS provider: {}", provider_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::utils::cookies::SameSite;
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
    fn provider_by_id_supports_aws_and_gcp() {
        let config = config_stub();
        let aws = provider_by_id("aws", &config).expect("aws provider");
        assert_eq!(aws.provider_id(), "aws");

        let gcp = provider_by_id("gcp", &config).expect("gcp provider");
        assert_eq!(gcp.provider_id(), "gcp");
    }

    #[test]
    fn aws_provider_round_trip() {
        let config = config_stub();
        let provider = AwsKmsProvider::from_config(&config);
        let plaintext = b"aws-test";
        let envelope = provider.encrypt(plaintext).expect("encrypt");
        let decrypted = provider
            .decrypt(&envelope.nonce, &envelope.ciphertext)
            .expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn gcp_provider_round_trip() {
        let config = config_stub();
        let provider = GcpKmsProvider::from_config(&config);
        let plaintext = b"gcp-test";
        let envelope = provider.encrypt(plaintext).expect("encrypt");
        let decrypted = provider
            .decrypt(&envelope.nonce, &envelope.ciphertext)
            .expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }
}
