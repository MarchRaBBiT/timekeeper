use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::active_session::ActiveSession;
use crate::types::UserId;

pub async fn create_active_session(
    pool: &PgPool,
    user_id: UserId,
    refresh_token_id: &str,
    device_label: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<ActiveSession, sqlx::Error> {
    let session_id = Uuid::new_v4().to_string();
    let last_seen_at = Utc::now();

    sqlx::query_as::<_, ActiveSession>(
        r#"
        INSERT INTO active_sessions
            (id, user_id, refresh_token_id, device_label, expires_at, last_seen_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, user_id, refresh_token_id, device_label, created_at, last_seen_at, expires_at
        "#,
    )
    .bind(&session_id)
    .bind(user_id)
    .bind(refresh_token_id)
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
        SELECT id, user_id, refresh_token_id, device_label, created_at, last_seen_at, expires_at
        FROM active_sessions
        WHERE user_id = $1
        ORDER BY last_seen_at DESC NULLS LAST, created_at DESC, id DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn touch_active_session(
    pool: &PgPool,
    session_id: &str,
    last_seen_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE active_sessions
        SET last_seen_at = $1
        WHERE id = $2
        "#,
    )
    .bind(last_seen_at)
    .bind(session_id)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn delete_active_session_by_refresh_token_id(
    pool: &PgPool,
    refresh_token_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_sessions WHERE refresh_token_id = $1")
        .bind(refresh_token_id)
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

pub async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM active_sessions WHERE expires_at <= NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
