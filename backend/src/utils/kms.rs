use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_kms::primitives::Blob;
use base64::{
    engine::general_purpose::STANDARD, engine::general_purpose::STANDARD_NO_PAD, Engine as _,
};
use rand::{rngs::OsRng, RngCore};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env, future::Future, thread};

use crate::config::Config;

const NONCE_LENGTH: usize = 12;
const ENVELOPE_SCHEME: &str = "kms";
const ENVELOPE_VERSION: &str = "v1";
const DEFAULT_KEY_VERSION: u16 = 1;
const PSEUDO_PROVIDER_ID: &str = "pseudo";
const AWS_PROVIDER_ID: &str = "aws";
const GCP_PROVIDER_ID: &str = "gcp";

pub struct KmsEnvelope {
    pub provider_id: String,
    pub key_version: u16,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl KmsEnvelope {
    pub fn encode(&self) -> Result<String> {
        if self.nonce.len() != NONCE_LENGTH {
            return Err(anyhow!("Invalid nonce length"));
        }
        if self.key_version == 0 {
            return Err(anyhow!("Invalid key version"));
        }

        Ok(format!(
            "{}:{}:{}:{}:{}:{}",
            ENVELOPE_SCHEME,
            ENVELOPE_VERSION,
            self.provider_id,
            self.key_version,
            STANDARD_NO_PAD.encode(&self.nonce),
            STANDARD_NO_PAD.encode(&self.ciphertext)
        ))
    }

    pub fn parse(stored: &str) -> Result<Option<Self>> {
        if !stored.starts_with("kms:v1:") {
            return Ok(None);
        }

        let parts: Vec<&str> = stored.split(':').collect();
        let (provider_id, key_version, nonce_part, cipher_part) = match parts.as_slice() {
            // Backward compatibility: kms:v1:<nonce>:<ciphertext>
            ["kms", "v1", nonce, cipher] => (
                PSEUDO_PROVIDER_ID.to_string(),
                DEFAULT_KEY_VERSION,
                *nonce,
                *cipher,
            ),
            // Backward compatibility: kms:v1:<provider>:<nonce>:<ciphertext>
            ["kms", "v1", provider, nonce, cipher] => {
                (provider.to_string(), DEFAULT_KEY_VERSION, *nonce, *cipher)
            }
            ["kms", "v1", provider, version, nonce, cipher] => {
                let parsed_version = version
                    .parse::<u16>()
                    .map_err(|_| anyhow!("Invalid key version"))?;
                if parsed_version == 0 {
                    return Err(anyhow!("Invalid key version"));
                }
                (provider.to_string(), parsed_version, *nonce, *cipher)
            }
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
            key_version,
            nonce,
            ciphertext,
        }))
    }
}

pub trait KmsProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn key_version(&self) -> u16;
    fn encrypt(&self, plaintext: &[u8]) -> Result<KmsEnvelope>;
    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>>;
}

pub fn active_key_version() -> u16 {
    env::var("KMS_ACTIVE_KEY_VERSION")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|version| *version > 0)
        .unwrap_or(DEFAULT_KEY_VERSION)
}

fn resolve_versioned_env(base_name: &str, key_version: u16) -> Option<String> {
    if key_version > DEFAULT_KEY_VERSION {
        let versioned = format!("{}_V{}", base_name, key_version);
        if let Ok(value) = env::var(versioned) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    env::var(base_name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn random_nonce() -> [u8; NONCE_LENGTH] {
    let mut nonce = [0u8; NONCE_LENGTH];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

fn with_nonce_prefix(nonce: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(nonce.len() + plaintext.len());
    out.extend_from_slice(nonce);
    out.extend_from_slice(plaintext);
    out
}

fn split_nonce_prefixed_plaintext(bytes: &[u8]) -> Result<(&[u8], &[u8])> {
    if bytes.len() < NONCE_LENGTH {
        return Err(anyhow!("Decrypted payload is too short"));
    }
    let (nonce, plaintext) = bytes.split_at(NONCE_LENGTH);
    Ok((nonce, plaintext))
}

fn block_on_thread<F, T>(future: F) -> Result<T>
where
    F: Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let handle = thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to build runtime for KMS call")?;
        runtime.block_on(future)
    });

    handle
        .join()
        .map_err(|_| anyhow!("KMS worker thread panicked"))?
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

    fn encrypt(
        &self,
        provider_id: &'static str,
        key_version: u16,
        plaintext: &[u8],
    ) -> Result<KmsEnvelope> {
        let nonce = random_nonce();

        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| anyhow!("Invalid key"))?;
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|_| anyhow!("Encryption failed"))?;

        Ok(KmsEnvelope {
            provider_id: provider_id.to_string(),
            key_version,
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
    key_version: u16,
}

impl PseudoKmsProvider {
    pub fn from_config(config: &Config, key_version: u16) -> Self {
        let key_version_str = key_version.to_string();
        let crypto = LocalAeadProvider::from_context(
            config,
            &[
                PSEUDO_PROVIDER_ID,
                &key_version_str,
                &config.aws_region,
                &config.aws_kms_key_id,
            ],
        );
        Self {
            crypto,
            key_version,
        }
    }
}

impl KmsProvider for PseudoKmsProvider {
    fn provider_id(&self) -> &'static str {
        PSEUDO_PROVIDER_ID
    }

    fn key_version(&self) -> u16 {
        self.key_version
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<KmsEnvelope> {
        self.crypto
            .encrypt(self.provider_id(), self.key_version(), plaintext)
    }

    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        self.crypto.decrypt(nonce, ciphertext)
    }
}

pub struct AwsKmsProvider {
    region: String,
    key_id: String,
    key_version: u16,
}

impl AwsKmsProvider {
    pub fn from_config(config: &Config, key_version: u16) -> Self {
        let key_id = resolve_versioned_env("AWS_KMS_KEY_ID", key_version)
            .unwrap_or_else(|| config.aws_kms_key_id.clone());

        Self {
            region: config.aws_region.clone(),
            key_id,
            key_version,
        }
    }
}

impl KmsProvider for AwsKmsProvider {
    fn provider_id(&self) -> &'static str {
        AWS_PROVIDER_ID
    }

    fn key_version(&self) -> u16 {
        self.key_version
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<KmsEnvelope> {
        if self.key_id.trim().is_empty() {
            return Err(anyhow!("AWS KMS key id is not configured"));
        }

        let nonce = random_nonce();
        let payload = with_nonce_prefix(&nonce, plaintext);
        let region = self.region.clone();
        let key_id = self.key_id.clone();

        let ciphertext = block_on_thread(async move {
            let aws_config = aws_config::defaults(BehaviorVersion::latest())
                .region(aws_config::Region::new(region))
                .load()
                .await;
            let client = aws_sdk_kms::Client::new(&aws_config);
            let response = client
                .encrypt()
                .key_id(key_id)
                .plaintext(Blob::new(payload))
                .send()
                .await
                .context("AWS KMS Encrypt API call failed")?;

            response
                .ciphertext_blob
                .map(|blob| blob.into_inner())
                .ok_or_else(|| anyhow!("AWS KMS Encrypt returned empty ciphertext"))
        })?;

        Ok(KmsEnvelope {
            provider_id: self.provider_id().to_string(),
            key_version: self.key_version(),
            nonce: nonce.to_vec(),
            ciphertext,
        })
    }

    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if self.key_id.trim().is_empty() {
            return Err(anyhow!("AWS KMS key id is not configured"));
        }

        if nonce.len() != NONCE_LENGTH {
            return Err(anyhow!("Invalid nonce length"));
        }

        let region = self.region.clone();
        let ciphertext_blob = ciphertext.to_vec();
        let expected_nonce = nonce.to_vec();

        let decrypted = block_on_thread(async move {
            let aws_config = aws_config::defaults(BehaviorVersion::latest())
                .region(aws_config::Region::new(region))
                .load()
                .await;
            let client = aws_sdk_kms::Client::new(&aws_config);
            let response = client
                .decrypt()
                .ciphertext_blob(Blob::new(ciphertext_blob))
                .send()
                .await
                .context("AWS KMS Decrypt API call failed")?;

            response
                .plaintext
                .map(|blob| blob.into_inner())
                .ok_or_else(|| anyhow!("AWS KMS Decrypt returned empty plaintext"))
        })?;

        let (embedded_nonce, plaintext) = split_nonce_prefixed_plaintext(&decrypted)?;
        if embedded_nonce != expected_nonce.as_slice() {
            return Err(anyhow!("KMS envelope nonce mismatch"));
        }

        Ok(plaintext.to_vec())
    }
}

#[derive(Serialize)]
struct GcpEncryptRequest {
    plaintext: String,
}

#[derive(Deserialize)]
struct GcpEncryptResponse {
    ciphertext: String,
}

#[derive(Serialize)]
struct GcpDecryptRequest {
    ciphertext: String,
}

#[derive(Deserialize)]
struct GcpDecryptResponse {
    plaintext: String,
}

pub struct GcpKmsProvider {
    key_name: String,
    access_token: Option<String>,
    http_client: Client,
    key_version: u16,
}

impl GcpKmsProvider {
    pub fn from_config(config: &Config, key_version: u16) -> Self {
        let key_name = resolve_versioned_env("GCP_KMS_KEY_NAME", key_version)
            .or_else(|| resolve_versioned_env("AWS_KMS_KEY_ID", key_version))
            .unwrap_or_else(|| config.aws_kms_key_id.clone());

        let access_token = env::var("GCP_ACCESS_TOKEN")
            .ok()
            .or_else(|| env::var("GOOGLE_OAUTH_ACCESS_TOKEN").ok());

        Self {
            key_name,
            access_token,
            http_client: Client::new(),
            key_version,
        }
    }

    fn access_token(&self) -> Result<&str> {
        self.access_token
            .as_deref()
            .ok_or_else(|| anyhow!("GCP access token is not configured"))
    }

    fn encrypt_url(&self) -> String {
        format!(
            "https://cloudkms.googleapis.com/v1/{}:encrypt",
            self.key_name
        )
    }

    fn decrypt_url(&self) -> String {
        format!(
            "https://cloudkms.googleapis.com/v1/{}:decrypt",
            self.key_name
        )
    }
}

impl KmsProvider for GcpKmsProvider {
    fn provider_id(&self) -> &'static str {
        GCP_PROVIDER_ID
    }

    fn key_version(&self) -> u16 {
        self.key_version
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<KmsEnvelope> {
        if self.key_name.trim().is_empty() {
            return Err(anyhow!("GCP KMS key name is not configured"));
        }

        let nonce = random_nonce();
        let payload = with_nonce_prefix(&nonce, plaintext);
        let request = GcpEncryptRequest {
            plaintext: STANDARD.encode(payload),
        };
        let token = self.access_token()?;

        let response = self
            .http_client
            .post(self.encrypt_url())
            .bearer_auth(token)
            .json(&request)
            .send()
            .context("GCP KMS Encrypt API call failed")?
            .error_for_status()
            .context("GCP KMS Encrypt returned error status")?;

        let body: GcpEncryptResponse = response
            .json()
            .context("Failed to parse GCP KMS Encrypt response")?;

        let ciphertext = STANDARD
            .decode(body.ciphertext)
            .map_err(|_| anyhow!("Invalid GCP ciphertext encoding"))?;

        Ok(KmsEnvelope {
            provider_id: self.provider_id().to_string(),
            key_version: self.key_version(),
            nonce: nonce.to_vec(),
            ciphertext,
        })
    }

    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if self.key_name.trim().is_empty() {
            return Err(anyhow!("GCP KMS key name is not configured"));
        }

        if nonce.len() != NONCE_LENGTH {
            return Err(anyhow!("Invalid nonce length"));
        }

        let request = GcpDecryptRequest {
            ciphertext: STANDARD.encode(ciphertext),
        };
        let token = self.access_token()?;

        let response = self
            .http_client
            .post(self.decrypt_url())
            .bearer_auth(token)
            .json(&request)
            .send()
            .context("GCP KMS Decrypt API call failed")?
            .error_for_status()
            .context("GCP KMS Decrypt returned error status")?;

        let body: GcpDecryptResponse = response
            .json()
            .context("Failed to parse GCP KMS Decrypt response")?;

        let decrypted_payload = STANDARD
            .decode(body.plaintext)
            .map_err(|_| anyhow!("Invalid GCP plaintext encoding"))?;

        let (embedded_nonce, plaintext) = split_nonce_prefixed_plaintext(&decrypted_payload)?;
        if embedded_nonce != nonce {
            return Err(anyhow!("KMS envelope nonce mismatch"));
        }

        Ok(plaintext.to_vec())
    }
}

pub fn active_provider(config: &Config) -> Box<dyn KmsProvider> {
    let key_version = active_key_version();
    match env::var("KMS_PROVIDER").ok().as_deref() {
        Some(AWS_PROVIDER_ID) => Box::new(AwsKmsProvider::from_config(config, key_version)),
        Some(GCP_PROVIDER_ID) => Box::new(GcpKmsProvider::from_config(config, key_version)),
        _ => Box::new(PseudoKmsProvider::from_config(config, key_version)),
    }
}

pub fn provider_by_id_and_version(
    provider_id: &str,
    key_version: u16,
    config: &Config,
) -> Result<Box<dyn KmsProvider>> {
    if key_version == 0 {
        return Err(anyhow!("Invalid key version"));
    }

    match provider_id {
        PSEUDO_PROVIDER_ID => Ok(Box::new(PseudoKmsProvider::from_config(
            config,
            key_version,
        ))),
        AWS_PROVIDER_ID => Ok(Box::new(AwsKmsProvider::from_config(config, key_version))),
        GCP_PROVIDER_ID => Ok(Box::new(GcpKmsProvider::from_config(config, key_version))),
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
            audit_log_export_max_rows: 10_000,
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
        let aws = provider_by_id_and_version("aws", 1, &config).expect("aws provider");
        assert_eq!(aws.provider_id(), "aws");

        let gcp = provider_by_id_and_version("gcp", 1, &config).expect("gcp provider");
        assert_eq!(gcp.provider_id(), "gcp");
    }

    #[test]
    fn split_nonce_prefixed_plaintext_extracts_nonce_and_payload() {
        let nonce = [1u8; NONCE_LENGTH];
        let payload = with_nonce_prefix(&nonce, b"hello");
        let (decoded_nonce, plaintext) = split_nonce_prefixed_plaintext(&payload).expect("split");
        assert_eq!(decoded_nonce, nonce);
        assert_eq!(plaintext, b"hello");
    }

    #[test]
    fn envelope_parse_supports_new_and_legacy_formats() {
        let encoded = "kms:v1:pseudo:2:AAAAAAAAAAAAAAAA:Ym9keQ";
        let envelope = KmsEnvelope::parse(encoded)
            .expect("parse")
            .expect("envelope");
        assert_eq!(envelope.provider_id, "pseudo");
        assert_eq!(envelope.key_version, 2);

        let legacy = "kms:v1:AAAAAAAAAAAAAAAA:Ym9keQ";
        let envelope = KmsEnvelope::parse(legacy)
            .expect("parse")
            .expect("envelope");
        assert_eq!(envelope.provider_id, "pseudo");
        assert_eq!(envelope.key_version, 1);
    }

    #[test]
    fn pseudo_provider_key_version_changes_derived_key() {
        let config = config_stub();
        let provider_v1 = PseudoKmsProvider::from_config(&config, 1);
        let provider_v2 = PseudoKmsProvider::from_config(&config, 2);

        let envelope = provider_v1.encrypt(b"same-plaintext").expect("encrypt");
        let decrypted_v1 = provider_v1
            .decrypt(&envelope.nonce, &envelope.ciphertext)
            .expect("decrypt v1");
        assert_eq!(decrypted_v1, b"same-plaintext");

        let decrypt_v2 = provider_v2.decrypt(&envelope.nonce, &envelope.ciphertext);
        assert!(decrypt_v2.is_err());
    }
}
