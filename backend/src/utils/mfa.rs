use anyhow::{anyhow, Result};
use base32::Alphabet::RFC4648;
use rand::{rngs::OsRng, RngCore};
use totp_rs::{Algorithm, TOTP};

const SECRET_BYTE_LENGTH: usize = 20;
const CODE_DIGITS: usize = 6;
const STEP_SECONDS: u64 = 30;
const ALLOWED_SKEW: u8 = 1;

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
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn secret_round_trip_verification() {
        let secret = generate_totp_secret();
        let totp = build_totp(&secret).expect("totp build");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_secs();
        let current = totp.generate(now).expect("code");
        assert!(verify_totp_code(&secret, &current).unwrap());
    }
}
