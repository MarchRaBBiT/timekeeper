use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::active_session::ActiveSession;
use crate::types::UserId;

pub async fn create_active_session(
    pool: &PgPool,
    user_id: UserId,
    refresh_token_id: &str,
    access_jti: &str,
    device_label: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<ActiveSession, sqlx::Error> {
    let session_id = Uuid::new_v4().to_string();
    let last_seen_at = Utc::now();

    sqlx::query_as::<_, ActiveSession>(
        r#"
        INSERT INTO active_sessions
            (id, user_id, refresh_token_id, access_jti, device_label, expires_at, last_seen_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, user_id, refresh_token_id, access_jti, device_label, created_at, last_seen_at, expires_at
        "#,
    )
    .bind(&session_id)
    .bind(user_id)
    .bind(refresh_token_id)
    .bind(access_jti)
    .bind(device_label)
    .bind(expires_at)
    .bind(last_seen_at)
    .fetch_one(pool)
    .await
}

pub async fn list_active_sessions_for_user(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<ActiveSession>, sqlx::Error> {
    sqlx::query_as::<_, ActiveSession>(
        r#"
        SELECT id, user_id, refresh_token_id, access_jti, device_label, created_at, last_seen_at, expires_at
        FROM active_sessions
        WHERE user_id = $1
        ORDER BY last_seen_at DESC NULLS LAST, created_at DESC, id DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn find_active_session_by_id(
    pool: &PgPool,
    session_id: &str,
) -> Result<Option<ActiveSession>, sqlx::Error> {
    sqlx::query_as::<_, ActiveSession>(
        r#"
        SELECT id, user_id, refresh_token_id, access_jti, device_label, created_at, last_seen_at, expires_at
        FROM active_sessions
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
}

pub async fn find_active_session_by_refresh_token_id(
    pool: &PgPool,
    refresh_token_id: &str,
) -> Result<Option<ActiveSession>, sqlx::Error> {
    sqlx::query_as::<_, ActiveSession>(
        r#"
        SELECT id, user_id, refresh_token_id, access_jti, device_label, created_at, last_seen_at, expires_at
        FROM active_sessions
        WHERE refresh_token_id = $1
        "#,
    )
    .bind(refresh_token_id)
    .fetch_optional(pool)
    .await
}

pub async fn find_active_session_by_access_jti(
    pool: &PgPool,
    access_jti: &str,
) -> Result<Option<ActiveSession>, sqlx::Error> {
    sqlx::query_as::<_, ActiveSession>(
        r#"
        SELECT id, user_id, refresh_token_id, access_jti, device_label, created_at, last_seen_at, expires_at
        FROM active_sessions
        WHERE access_jti = $1
        "#,
    )
    .bind(access_jti)
    .fetch_optional(pool)
    .await
}

pub async fn touch_active_session_by_access_jti(
    pool: &PgPool,
    access_jti: &str,
    last_seen_at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE active_sessions
        SET last_seen_at = $1
        WHERE access_jti = $2
        "#,
    )
    .bind(last_seen_at)
    .bind(access_jti)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_active_session_tokens(
    pool: &PgPool,
    current_refresh_token_id: &str,
    new_refresh_token_id: &str,
    new_access_jti: &str,
    last_seen_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE active_sessions
        SET refresh_token_id = $1,
            access_jti = $2,
            last_seen_at = $3,
            expires_at = $4
        WHERE refresh_token_id = $5
        "#,
    )
    .bind(new_refresh_token_id)
    .bind(new_access_jti)
    .bind(last_seen_at)
    .bind(expires_at)
    .bind(current_refresh_token_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_active_session_by_id(
    pool: &PgPool,
    session_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn delete_active_session_by_access_jti(
    pool: &PgPool,
    access_jti: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_sessions WHERE access_jti = $1")
        .bind(access_jti)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn delete_active_sessions_for_user(
    pool: &PgPool,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ())
}

#[allow(dead_code)]
pub async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM active_sessions WHERE expires_at <= NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
