use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::{models::user::User, utils::jwt::RefreshToken};
use crate::types::UserId;

#[allow(dead_code)]
#[derive(Debug, FromRow)]
pub struct StoredRefreshToken {
    pub id: String,
    pub user_id: UserId,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ActiveAccessToken<'a> {
    pub jti: &'a str,
    pub user_id: UserId,
    pub expires_at: DateTime<Utc>,
    pub context: Option<&'a str>,
}

pub async fn find_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn find_user_by_id(pool: &PgPool, user_id: UserId) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
}

pub async fn insert_refresh_token(pool: &PgPool, token: &RefreshToken) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token.id)
    .bind(token.user_id.to_string())
    .bind(&token.token_hash)
    .bind(token.expires_at)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn delete_refresh_token_by_id(pool: &PgPool, token_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn delete_refresh_token_for_user(
    pool: &PgPool,
    token_id: &str,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE id = $1 AND user_id = $2")
        .bind(token_id)
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn delete_refresh_tokens_for_user(
    pool: &PgPool,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn fetch_valid_refresh_token(
    pool: &PgPool,
    token_id: &str,
    now: DateTime<Utc>,
) -> Result<Option<StoredRefreshToken>, sqlx::Error> {
    sqlx::query_as::<_, StoredRefreshToken>(
        "SELECT id, user_id, token_hash, expires_at FROM refresh_tokens \
         WHERE id = $1 AND expires_at > $2",
    )
    .bind(token_id)
    .bind(now)
    .fetch_optional(pool)
    .await
}

pub async fn insert_active_access_token(
    pool: &PgPool,
    token: &ActiveAccessToken<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO active_access_tokens (jti, user_id, expires_at, context) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(token.jti)
    .bind(token.user_id.to_string())
    .bind(token.expires_at)
    .bind(token.context)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn access_token_exists(pool: &PgPool, jti: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM active_access_tokens WHERE jti = $1 LIMIT 1)",
    )
    .bind(jti)
    .fetch_one(pool)
    .await
}

pub async fn delete_active_access_token_by_jti(
    pool: &PgPool,
    jti: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_access_tokens WHERE jti = $1")
        .bind(jti)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn delete_active_access_tokens_for_user(
    pool: &PgPool,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_access_tokens WHERE user_id = $1")
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn cleanup_expired_access_tokens(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_access_tokens WHERE expires_at <= NOW()")
        .execute(pool)
        .await
        .map(|_| ())
}
