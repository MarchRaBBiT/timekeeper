use chrono::{Duration as ChronoDuration, Utc};
use std::sync::OnceLock;
use timekeeper_backend::utils::encryption::{encrypt_pii, hash_email};
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
    let email = format!("tx-user-{}@example.com", Uuid::new_v4());
    let now = Utc::now();
    let config = support::test_config();
    let mut tx = transaction::begin_transaction(&pool)
        .await
        .expect("begin transaction");

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name_enc, email_enc, email_hash, role, is_system_admin, \
         mfa_secret_enc, mfa_enabled_at, password_changed_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, NULL, $9, $10, $10)",
    )
    .bind(&user_id)
    .bind(format!("tx-user-{}", Uuid::new_v4()))
    .bind("hash")
    .bind(encrypt_pii("Tx User", &config).expect("encrypt full_name"))
    .bind(encrypt_pii(&email, &config).expect("encrypt email"))
    .bind(hash_email(&email, &config))
    .bind(UserRole::Employee.as_str())
    .bind(false)
    .bind(now)
    .bind(now)
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
    let email = format!("tx-rollback-{}@example.com", Uuid::new_v4());
    let now = Utc::now() + ChronoDuration::minutes(1);
    let config = support::test_config();
    let mut tx = transaction::begin_transaction(&pool)
        .await
        .expect("begin transaction");

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name_enc, email_enc, email_hash, role, is_system_admin, \
         mfa_secret_enc, mfa_enabled_at, password_changed_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, NULL, $9, $10, $10)",
    )
    .bind(&user_id)
    .bind(format!("tx-rollback-{}", Uuid::new_v4()))
    .bind("hash")
    .bind(encrypt_pii("Tx Rollback User", &config).expect("encrypt full_name"))
    .bind(encrypt_pii(&email, &config).expect("encrypt email"))
    .bind(hash_email(&email, &config))
    .bind(UserRole::Admin.as_str())
    .bind(true)
    .bind(now)
    .bind(now)
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
