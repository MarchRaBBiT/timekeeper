use std::sync::OnceLock;

use sqlx::Executor;
use timekeeper_backend::models::user::UserRole;
use timekeeper_backend::repositories::user as user_repo;
use tokio::sync::Mutex;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

#[tokio::test]
async fn user_exists_returns_expected_values() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    pool.execute("TRUNCATE users CASCADE")
        .await
        .expect("truncate users");

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let exists = user_repo::user_exists(&pool, &user.id)
        .await
        .expect("user_exists for existing user");
    assert!(exists);

    let missing = user_repo::user_exists(&pool, "00000000-0000-0000-0000-000000000000")
        .await
        .expect("user_exists for missing user");
    assert!(!missing);
}
