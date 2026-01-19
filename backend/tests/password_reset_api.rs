use chrono::Utc;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::auth_repo,
    models::user::{User, UserRole},
    repositories::password_reset as password_reset_repo,
    utils::{password::hash_password, security::generate_token},
};

mod support;

#[tokio::test]
async fn test_password_reset_full_flow() {
    let pool = support::test_pool().await;

    let email = "test@example.com";
    let initial_password = "OldPassword123!";
    let new_password = "NewPassword456!";

    let user = create_test_user(&pool, email, initial_password).await;

    let token = generate_token(32);
    let reset_record = password_reset_repo::create_password_reset(&pool, user.id, &token)
        .await
        .expect("create password reset");

    assert_eq!(reset_record.user_id, user.id);
    assert!(reset_record.used_at.is_none());
    assert!(reset_record.expires_at > Utc::now());

    let found_reset = password_reset_repo::find_valid_reset_by_token(&pool, &token)
        .await
        .expect("find reset")
        .expect("reset should exist");

    assert_eq!(found_reset.id, reset_record.id);
    assert_eq!(found_reset.user_id, user.id);

    let new_hash = hash_password(new_password).expect("hash new password");
    let updated_user = auth_repo::update_user_password(&pool, user.id, &new_hash)
        .await
        .expect("update password");

    assert_ne!(updated_user.password_hash, user.password_hash);

    password_reset_repo::mark_token_as_used(&pool, reset_record.id)
        .await
        .expect("mark token used");

    let used_reset =
        sqlx::query_as::<_, timekeeper_backend::models::password_reset::PasswordReset>(
            "SELECT * FROM password_resets WHERE id = $1",
        )
        .bind(reset_record.id)
        .fetch_one(&pool)
        .await
        .expect("fetch used reset");

    assert!(used_reset.used_at.is_some());

    let invalid_search = password_reset_repo::find_valid_reset_by_token(&pool, &token)
        .await
        .expect("search should succeed");

    assert!(invalid_search.is_none());
}

#[tokio::test]
async fn test_expired_token_cleanup() {
    let pool = support::test_pool().await;

    let user = create_test_user(&pool, "cleanup@example.com", "Pass123!").await;

    sqlx::query(
        "INSERT INTO password_resets (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(user.id)
    .bind("expired_token_hash")
    .bind(Utc::now() - chrono::Duration::hours(2))
    .execute(&pool)
    .await
    .expect("insert expired token");

    let deleted = password_reset_repo::delete_expired_tokens(&pool)
        .await
        .expect("cleanup");

    assert!(deleted >= 1);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM password_resets WHERE token_hash = $1")
            .bind("expired_token_hash")
            .fetch_one(&pool)
            .await
            .expect("count");

    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_invalid_token_returns_none() {
    let pool = support::test_pool().await;

    let result = password_reset_repo::find_valid_reset_by_token(&pool, "invalid_token")
        .await
        .expect("query should succeed");

    assert!(result.is_none());
}

async fn create_test_user(pool: &PgPool, email: &str, password: &str) -> User {
    let password_hash = hash_password(password).expect("hash password");

    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, password_hash, full_name, email, role, is_system_admin)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, 
        mfa_secret, mfa_enabled_at, created_at, updated_at
        "#,
    )
    .bind(format!("user_{}", email))
    .bind(password_hash)
    .bind("Test User")
    .bind(email)
    .bind(UserRole::Employee.as_str())
    .bind(false)
    .fetch_one(pool)
    .await
    .expect("create test user")
}
