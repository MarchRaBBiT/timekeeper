use chrono::{Duration as ChronoDuration, Utc};
use std::sync::OnceLock;
use timekeeper_backend::{
    models::user::UserRole,
    repositories::active_session,
};
use tokio::sync::Mutex;
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn insert_refresh_token(
    pool: &sqlx::PgPool,
    user_id: &str,
    expires_at: chrono::DateTime<Utc>,
) -> String {
    let token_id = Uuid::new_v4().to_string();
    let token_hash = format!("hash-{}", token_id);
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token_id)
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("insert refresh token");
    token_id
}

fn assert_same_millis(left: chrono::DateTime<Utc>, right: chrono::DateTime<Utc>) {
    assert_eq!(left.timestamp_millis(), right.timestamp_millis());
}

#[tokio::test]
async fn active_session_repo_roundtrip() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE active_sessions, refresh_tokens")
        .execute(&pool)
        .await
        .expect("truncate session tables");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let expires_at = Utc::now() + ChronoDuration::hours(1);
    let refresh_token_id = insert_refresh_token(&pool, &user.id.to_string(), expires_at).await;
    let access_jti = Uuid::new_v4().to_string();

    let created = active_session::create_active_session(
        &pool,
        user.id,
        &refresh_token_id,
        &access_jti,
        Some("macbook-pro"),
        expires_at,
    )
    .await
    .expect("create active session");

    assert_eq!(created.user_id, user.id);
    assert_eq!(created.refresh_token_id, refresh_token_id);
    assert_eq!(created.access_jti.as_deref(), Some(access_jti.as_str()));
    assert_eq!(created.device_label.as_deref(), Some("macbook-pro"));
    assert_same_millis(created.expires_at, expires_at);
    assert!(created.last_seen_at.is_some());

    let sessions = active_session::list_active_sessions_for_user(&pool, user.id)
        .await
        .expect("list active sessions");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, created.id);

    let new_last_seen = Utc::now() + ChronoDuration::minutes(5);
    active_session::touch_active_session_by_access_jti(&pool, &access_jti, new_last_seen)
        .await
        .expect("touch active session");

    let sessions = active_session::list_active_sessions_for_user(&pool, user.id)
        .await
        .expect("list active sessions after touch");
    assert_same_millis(sessions[0].last_seen_at.expect("last_seen_at"), new_last_seen);

    sqlx::query("TRUNCATE active_sessions, refresh_tokens")
        .execute(&pool)
        .await
        .expect("truncate session tables");
}

#[tokio::test]
async fn active_session_repo_deletes_by_refresh_token() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE active_sessions, refresh_tokens")
        .execute(&pool)
        .await
        .expect("truncate session tables");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let expires_at = Utc::now() + ChronoDuration::hours(2);
    let refresh_token_id = insert_refresh_token(&pool, &user.id.to_string(), expires_at).await;
    let access_jti = Uuid::new_v4().to_string();

    let created = active_session::create_active_session(
        &pool,
        user.id,
        &refresh_token_id,
        &access_jti,
        None,
        expires_at,
    )
    .await
    .expect("create active session");

    active_session::delete_active_session_by_id(&pool, &created.id)
        .await
        .expect("delete by id");

    let sessions = active_session::list_active_sessions_for_user(&pool, user.id)
        .await
        .expect("list active sessions");
    assert!(sessions.is_empty());

    sqlx::query("TRUNCATE active_sessions, refresh_tokens")
        .execute(&pool)
        .await
        .expect("truncate session tables");
}

#[tokio::test]
async fn active_session_repo_cleans_expired_sessions() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE active_sessions, refresh_tokens")
        .execute(&pool)
        .await
        .expect("truncate session tables");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let expired_at = Utc::now() - ChronoDuration::hours(1);
    let valid_expires_at = Utc::now() + ChronoDuration::hours(1);
    let expired_refresh = insert_refresh_token(&pool, &user.id.to_string(), valid_expires_at).await;
    let valid_refresh = insert_refresh_token(&pool, &user.id.to_string(), valid_expires_at).await;
    let expired_access_jti = Uuid::new_v4().to_string();
    let valid_access_jti = Uuid::new_v4().to_string();

    active_session::create_active_session(
        &pool,
        user.id,
        &expired_refresh,
        &expired_access_jti,
        Some("expired"),
        expired_at,
    )
    .await
    .expect("create expired session");
    active_session::create_active_session(
        &pool,
        user.id,
        &valid_refresh,
        &valid_access_jti,
        Some("valid"),
        valid_expires_at,
    )
    .await
    .expect("create valid session");

    let deleted = active_session::cleanup_expired_sessions(&pool)
        .await
        .expect("cleanup expired sessions");
    assert_eq!(deleted, 1);

    let sessions = active_session::list_active_sessions_for_user(&pool, user.id)
        .await
        .expect("list remaining sessions");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].device_label.as_deref(), Some("valid"));

    sqlx::query("TRUNCATE active_sessions, refresh_tokens")
        .execute(&pool)
        .await
        .expect("truncate session tables");
}
