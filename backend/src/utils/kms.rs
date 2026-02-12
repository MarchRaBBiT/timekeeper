use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};

use crate::config::Config;

const NONCE_LENGTH: usize = 12;
const ENVELOPE_SCHEME: &str = "kms";
const ENVELOPE_VERSION: &str = "v1";
const PSEUDO_PROVIDER_ID: &str = "pseudo";

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

pub struct PseudoKmsProvider {
    key: [u8; 32],
}

impl PseudoKmsProvider {
    pub fn from_config(config: &Config) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(config.jwt_secret.as_bytes());
        hasher.update(b"|");
        hasher.update(config.aws_region.as_bytes());
        hasher.update(b"|");
        hasher.update(config.aws_kms_key_id.as_bytes());
        let digest = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&digest);
        Self { key }
    }
}

impl KmsProvider for PseudoKmsProvider {
    fn provider_id(&self) -> &'static str {
        PSEUDO_PROVIDER_ID
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<KmsEnvelope> {
        let mut nonce = [0u8; NONCE_LENGTH];
        OsRng.fill_bytes(&mut nonce);

        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| anyhow!("Invalid key"))?;
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|_| anyhow!("Encryption failed"))?;

        Ok(KmsEnvelope {
            provider_id: self.provider_id().to_string(),
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

pub fn active_provider(config: &Config) -> Box<dyn KmsProvider> {
    Box::new(PseudoKmsProvider::from_config(config))
}

pub fn provider_by_id(provider_id: &str, config: &Config) -> Result<Box<dyn KmsProvider>> {
    match provider_id {
        PSEUDO_PROVIDER_ID => Ok(Box::new(PseudoKmsProvider::from_config(config))),
        _ => Err(anyhow!("Unsupported KMS provider: {}", provider_id)),
    }
}
