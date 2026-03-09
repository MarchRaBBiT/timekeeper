use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::post,
    Router,
};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use std::{env, sync::OnceLock};
use timekeeper_backend::{
    handlers,
    models::user::{User, UserRole},
    repositories::{auth as auth_repo, password_reset as password_reset_repo},
    state::AppState,
    utils::{
        encryption::{decrypt_pii, encrypt_pii, hash_email},
        password::{hash_password, verify_password},
        security::generate_token,
    },
};
use tokio::sync::Mutex;
use tower::ServiceExt;
use uuid::Uuid;

mod support;

async fn migrate_db(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("run migrations");
}

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

fn configure_email_skip() {
    static EMAIL_SKIP: OnceLock<()> = OnceLock::new();
    EMAIL_SKIP.get_or_init(|| {
        env::set_var("SMTP_SKIP_SEND", "true");
    });
}

async fn reset_password_resets(pool: &PgPool) {
    sqlx::query("TRUNCATE password_resets")
        .execute(pool)
        .await
        .expect("truncate password_resets");
}

#[tokio::test]
async fn test_password_reset_full_flow() {
    let _guard = integration_guard().await;
    configure_email_skip();
    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    reset_password_resets(&pool).await;

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
    let updated_user =
        auth_repo::update_user_password(&pool, user.id, &new_hash, &user.password_hash, 5)
            .await
            .expect("update password");

    assert_ne!(updated_user.password_hash, user.password_hash);

    password_reset_repo::mark_token_as_used(&pool, &reset_record.id)
        .await
        .expect("mark token used");

    let used_reset =
        sqlx::query_as::<_, timekeeper_backend::models::password_reset::PasswordReset>(
            "SELECT * FROM password_resets WHERE id = $1",
        )
        .bind(&reset_record.id)
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
    let _guard = integration_guard().await;
    configure_email_skip();
    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    reset_password_resets(&pool).await;

    let user = create_test_user(&pool, "cleanup@example.com", "Pass123!").await;

    sqlx::query(
        "INSERT INTO password_resets (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4().to_string())
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
    let _guard = integration_guard().await;
    configure_email_skip();
    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    reset_password_resets(&pool).await;

    let result = password_reset_repo::find_valid_reset_by_token(&pool, "invalid_token")
        .await
        .expect("query should succeed");

    assert!(result.is_none());
}

async fn create_test_user(pool: &PgPool, email: &str, password: &str) -> User {
    let password_hash = hash_password(password).expect("hash password");
    let config = support::test_config();

    let user_id = Uuid::new_v4();
    let (local, domain) = email.split_once('@').unwrap_or((email, "example.com"));
    let unique_email = format!("{}+{}@{}", local, user_id, domain);
    let username = format!("user_{}", unique_email);
    let mut user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, username, password_hash, full_name_enc, email_enc, email_hash, role, is_system_admin)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, username, password_hash, full_name_enc as full_name, email_enc as email, LOWER(role) as role, is_system_admin,
        mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(username)
    .bind(password_hash)
    .bind(encrypt_pii("Test User", &config).expect("encrypt full_name"))
    .bind(encrypt_pii(&unique_email, &config).expect("encrypt email"))
    .bind(hash_email(&unique_email, &config))
    .bind(UserRole::Employee.as_str())
    .bind(false)
    .fetch_one(pool)
    .await
    .expect("create test user");
    user.full_name = decrypt_pii(&user.full_name, &config).expect("decrypt full_name");
    user.email = decrypt_pii(&user.email, &config).expect("decrypt email");
    user
}

#[tokio::test]
async fn request_password_reset_creates_token_record() {
    let _guard = integration_guard().await;
    configure_email_skip();
    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    reset_password_resets(&pool).await;

    let user = create_test_user(&pool, "reset-request@example.com", "Pass123!").await;
    let state = AppState::new(pool.clone(), None, None, None, support::test_config());

    let app = Router::new()
        .route(
            "/api/auth/request-password-reset",
            post(handlers::auth::request_password_reset),
        )
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/request-password-reset")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "email": user.email }).to_string()))
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("parse response");
    assert!(payload.get("message").and_then(|v| v.as_str()).is_some());

    let token_hash: String = sqlx::query_scalar(
        "SELECT token_hash FROM password_resets WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .expect("fetch token hash");

    assert_eq!(token_hash.len(), 64);
}

#[tokio::test]
async fn create_password_reset_invalidates_previous_unused_tokens() {
    let _guard = integration_guard().await;
    configure_email_skip();
    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    reset_password_resets(&pool).await;

    let user = create_test_user(&pool, "reset-rotate@example.com", "Pass123!").await;
    let first_token = generate_token(32);
    let second_token = generate_token(32);

    let first_record = password_reset_repo::create_password_reset(&pool, user.id, &first_token)
        .await
        .expect("create first reset token");
    let second_record = password_reset_repo::create_password_reset(&pool, user.id, &second_token)
        .await
        .expect("create second reset token");

    let first_used_at: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT used_at FROM password_resets WHERE id = $1")
            .bind(&first_record.id)
            .fetch_one(&pool)
            .await
            .expect("fetch first used_at");
    let second_used_at: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT used_at FROM password_resets WHERE id = $1")
            .bind(&second_record.id)
            .fetch_one(&pool)
            .await
            .expect("fetch second used_at");
    let unused_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM password_resets WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .expect("count unused tokens");

    assert!(first_used_at.is_some());
    assert!(second_used_at.is_none());
    assert_eq!(unused_count, 1);
    assert!(
        password_reset_repo::find_valid_reset_by_token(&pool, &first_token)
            .await
            .expect("query first token")
            .is_none()
    );
    assert!(
        password_reset_repo::find_valid_reset_by_token(&pool, &second_token)
            .await
            .expect("query second token")
            .is_some()
    );
}

#[tokio::test]
async fn reset_password_endpoint_marks_token_used_and_rejects_reuse() {
    let _guard = integration_guard().await;
    configure_email_skip();
    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    reset_password_resets(&pool).await;

    let user = create_test_user(&pool, "reset-endpoint@example.com", "Pass123!").await;
    let token = generate_token(32);
    let reset_record = password_reset_repo::create_password_reset(&pool, user.id, &token)
        .await
        .expect("create reset record");

    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let app = Router::new()
        .route(
            "/api/auth/reset-password",
            post(handlers::auth::reset_password),
        )
        .with_state(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/reset-password")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "token": token, "new_password": "NewPassword123!" }).to_string(),
                ))
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);

    let used_at: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT used_at FROM password_resets WHERE id = $1")
            .bind(&reset_record.id)
            .fetch_one(&pool)
            .await
            .expect("fetch used_at");
    assert!(used_at.is_some());

    let updated_user = auth_repo::find_user_by_id(&pool, user.id)
        .await
        .expect("fetch user")
        .expect("user exists");
    let matches =
        verify_password("NewPassword123!", &updated_user.password_hash).expect("verify password");
    assert!(matches);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/reset-password")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "token": token, "new_password": "OtherPassword123!" }).to_string(),
                ))
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reset_password_endpoint_allows_only_one_concurrent_success() {
    let _guard = integration_guard().await;
    configure_email_skip();
    let pool = support::test_pool().await;
    migrate_db(&pool).await;
    reset_password_resets(&pool).await;

    let user = create_test_user(&pool, "reset-race@example.com", "Pass123!").await;
    let token = generate_token(32);
    password_reset_repo::create_password_reset(&pool, user.id, &token)
        .await
        .expect("create reset record");

    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let app = Router::new()
        .route(
            "/api/auth/reset-password",
            post(handlers::auth::reset_password),
        )
        .with_state(state);

    let request_1 = Request::builder()
        .method("POST")
        .uri("/api/auth/reset-password")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "token": token, "new_password": "NewPassword123!" }).to_string(),
        ))
        .expect("build request 1");
    let request_2 = Request::builder()
        .method("POST")
        .uri("/api/auth/reset-password")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "token": token, "new_password": "OtherPassword123!" }).to_string(),
        ))
        .expect("build request 2");

    let (response_1, response_2) =
        tokio::join!(app.clone().oneshot(request_1), app.oneshot(request_2));
    let statuses = [
        response_1.expect("response 1").status(),
        response_2.expect("response 2").status(),
    ];
    let success_count = statuses
        .iter()
        .filter(|status| **status == StatusCode::OK)
        .count();
    let failure_count = statuses
        .iter()
        .filter(|status| **status == StatusCode::BAD_REQUEST)
        .count();

    assert_eq!(success_count, 1);
    assert_eq!(failure_count, 1);

    let updated_user = auth_repo::find_user_by_id(&pool, user.id)
        .await
        .expect("fetch user")
        .expect("user exists");
    let matches_first = verify_password("NewPassword123!", &updated_user.password_hash)
        .expect("verify first password");
    let matches_second = verify_password("OtherPassword123!", &updated_user.password_hash)
        .expect("verify second password");

    assert_ne!(matches_first, matches_second);
}
