use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub role: String,
    pub exp: i64,    // expiration time
    pub iat: i64,    // issued at
    pub jti: String, // JWT ID
}

#[derive(Debug)]
pub struct RefreshToken {
    pub id: String,
    pub user_id: String,
    pub secret: String,
    pub token_hash: String,
    pub expires_at: chrono::DateTime<Utc>,
}

impl Claims {
    pub fn new(user_id: String, username: String, role: String, expiration_hours: u64) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(expiration_hours as i64);

        Self {
            sub: user_id,
            username,
            role,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        }
    }
}

pub fn create_access_token(
    user_id: String,
    username: String,
    role: String,
    secret: &str,
    expiration_hours: u64,
) -> anyhow::Result<(String, Claims)> {
    let claims = Claims::new(user_id, username, role, expiration_hours);
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )?;

    Ok((token, claims))
}

impl RefreshToken {
    pub fn encoded(&self) -> String {
        format!("{}:{}", self.id, self.secret)
    }
}

pub fn create_refresh_token(user_id: String, expiration_days: u64) -> anyhow::Result<RefreshToken> {
    let secret = Uuid::new_v4().to_string();
    let token_hash = hash_refresh_token(&secret)?;
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(expiration_days as i64))
        .ok_or_else(|| anyhow::anyhow!("Refresh token expiration overflow"))?;

    Ok(RefreshToken {
        id: Uuid::new_v4().to_string(),
        user_id,
        secret,
        token_hash,
        expires_at,
    })
}

#[allow(dead_code)]
pub fn verify_access_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    Ok(token_data.claims)
}

pub fn hash_refresh_token(token: &str) -> anyhow::Result<String> {
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let token_hash = argon2
        .hash_password(token.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash refresh token: {}", e))?;

    Ok(token_hash.to_string())
}

pub fn verify_refresh_token(token: &str, hash: &str) -> anyhow::Result<bool> {
    use argon2::password_hash::PasswordHash;
    use argon2::{Argon2, PasswordVerifier};

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("Invalid refresh token hash: {}", e))?;

    let argon2 = Argon2::default();
    let result = argon2.verify_password(token.as_bytes(), &parsed_hash);

    match result {
        Ok(_) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(anyhow::anyhow!("Refresh token verification error: {}", e)),
    }
}

pub fn decode_refresh_token(compound: &str) -> anyhow::Result<(String, String)> {
    let mut parts = compound.splitn(2, ':');
    let id = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Malformed refresh token"))?;
    let secret = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Malformed refresh token"))?;

    if id.is_empty() || secret.is_empty() {
        return Err(anyhow::anyhow!("Malformed refresh token"));
    }

    Ok((id.to_string(), secret.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_verify_with_snake_case_role() {
        let (token, _) =
            create_access_token("user-123".into(), "bob".into(), "admin".into(), "secret", 1)
                .expect("create token");
        let claims = verify_access_token(&token, "secret").expect("verify token");
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.username, "bob");
        assert_eq!(claims.role, "admin");
    }

    #[test]
    fn decode_refresh_token_requires_two_parts() {
        assert!(decode_refresh_token("abc:def").is_ok());
        assert!(decode_refresh_token("abc").is_err());
        assert!(decode_refresh_token(":def").is_err());
    }
}
