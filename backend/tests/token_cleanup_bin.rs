use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::process::Command;
use timekeeper_backend::models::user::UserRole;
use uuid::Uuid;

mod support;

async fn migrate_db(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("run migrations");
}

#[tokio::test]
async fn token_cleanup_binary_removes_expired_records() {
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let expired = Utc::now() - Duration::hours(2);

    sqlx::query("INSERT INTO active_access_tokens (jti, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(Uuid::new_v4().to_string())
        .bind(user.id.to_string())
        .bind(expired)
        .execute(&pool)
        .await
        .expect("insert active access token");

    let refresh_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&refresh_id)
    .bind(user.id.to_string())
    .bind("expired-token-hash")
    .bind(expired)
    .execute(&pool)
    .await
    .expect("insert refresh token");

    sqlx::query(
        "INSERT INTO active_sessions \
         (id, user_id, refresh_token_id, access_jti, device_label, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user.id.to_string())
    .bind(&refresh_id)
    .bind(None::<String>)
    .bind(Some("test-device".to_string()))
    .bind(expired)
    .execute(&pool)
    .await
    .expect("insert active session");

    sqlx::query(
        "INSERT INTO password_resets (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user.id.to_string())
    .bind("expired-reset-hash")
    .bind(expired)
    .execute(&pool)
    .await
    .expect("insert password reset");

    let bin = env!("CARGO_BIN_EXE_token_cleanup");
    let db_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL");

    let status = Command::new(bin)
        .env("DATABASE_URL", db_url)
        .env(
            "JWT_SECRET",
            "0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .status()
        .expect("run token_cleanup");
    assert!(status.success());

    let access_tokens: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM active_access_tokens")
        .fetch_one(&pool)
        .await
        .expect("count active access tokens");
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM active_sessions")
        .fetch_one(&pool)
        .await
        .expect("count active sessions");
    let resets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM password_resets")
        .fetch_one(&pool)
        .await
        .expect("count password resets");

    assert_eq!(access_tokens, 0);
    assert_eq!(sessions, 0);
    assert_eq!(resets, 0);
}
