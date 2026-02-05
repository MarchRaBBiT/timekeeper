use chrono::{Duration as ChronoDuration, Utc};
use std::sync::OnceLock;
use tokio::sync::Mutex;
use uuid::Uuid;

use timekeeper_backend::{models::user::UserRole, repositories::transaction};

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn reset_tables(pool: &sqlx::PgPool) {
    sqlx::query("TRUNCATE users RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .expect("truncate users");
}

#[tokio::test]
async fn transaction_commit_persists_changes() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let user_id = Uuid::new_v4().to_string();
    let mut tx = transaction::begin_transaction(&pool)
        .await
        .expect("begin transaction");

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, email, role, is_system_admin, \
         mfa_secret, mfa_enabled_at, password_changed_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NULL, NULL, $8, $9, $9)",
    )
    .bind(&user_id)
    .bind(format!("tx-user-{}", Uuid::new_v4()))
    .bind("hash")
    .bind("Tx User")
    .bind(format!("tx-user-{}@example.com", Uuid::new_v4()))
    .bind(UserRole::Employee.as_str())
    .bind(false)
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .expect("insert user in transaction");

    transaction::commit_transaction(tx)
        .await
        .expect("commit transaction");

    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .expect("check committed row");
    assert!(exists);
}

#[tokio::test]
async fn transaction_rollback_discards_changes() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_tables(&pool).await;

    let user_id = Uuid::new_v4().to_string();
    let mut tx = transaction::begin_transaction(&pool)
        .await
        .expect("begin transaction");

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, email, role, is_system_admin, \
         mfa_secret, mfa_enabled_at, password_changed_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NULL, NULL, $8, $9, $9)",
    )
    .bind(&user_id)
    .bind(format!("tx-rollback-{}", Uuid::new_v4()))
    .bind("hash")
    .bind("Tx Rollback User")
    .bind(format!("tx-rollback-{}@example.com", Uuid::new_v4()))
    .bind(UserRole::Admin.as_str())
    .bind(true)
    .bind(Utc::now() + ChronoDuration::minutes(1))
    .bind(Utc::now() + ChronoDuration::minutes(1))
    .execute(&mut *tx)
    .await
    .expect("insert user in transaction");

    transaction::rollback_transaction(tx)
        .await
        .expect("rollback transaction");

    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .expect("check rolled-back row");
    assert!(!exists);
}
