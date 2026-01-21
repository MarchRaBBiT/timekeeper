use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::password_reset::PasswordReset;
use crate::types::UserId;

pub async fn create_password_reset(
    pool: &PgPool,
    user_id: UserId,
    token: &str,
) -> Result<PasswordReset, AppError> {
    let token_hash = hash_token(token);
    let expires_at = Utc::now() + Duration::hours(1);

    let reset_id = Uuid::new_v4();

    let record = sqlx::query_as::<_, PasswordReset>(
        r#"
        INSERT INTO password_resets (id, user_id, token_hash, expires_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id, user_id, token_hash, expires_at, created_at, used_at
        "#,
    )
    .bind(reset_id.to_string())
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(record)
}

pub async fn find_valid_reset_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<PasswordReset>, AppError> {
    let token_hash = hash_token(token);
    let now = Utc::now();

    let record = sqlx::query_as::<_, PasswordReset>(
        r#"
        SELECT id, user_id, token_hash, expires_at, created_at, used_at
        FROM password_resets
        WHERE token_hash = $1
        AND expires_at > $2
        AND used_at IS NULL
        "#,
    )
    .bind(&token_hash)
    .bind(now)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

pub async fn mark_token_as_used(pool: &PgPool, reset_id: &str) -> Result<(), AppError> {
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE password_resets
        SET used_at = $1
        WHERE id = $2
        "#,
    )
    .bind(now)
    .bind(reset_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_expired_tokens(pool: &PgPool) -> Result<u64, AppError> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        DELETE FROM password_resets
        WHERE expires_at < $1
        "#,
    )
    .bind(now)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_token() {
        let token = "test-token-123";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);
        assert_eq!(hash1, hash2);
        assert_ne!(hash_token("different-token"), hash1);
    }
}
