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

    let user_id = Uuid::new_v4();
    let (local, domain) = email.split_once('@').unwrap_or((email, "example.com"));
    let unique_email = format!("{}+{}@{}", local, user_id, domain);
    let username = format!("user_{}", unique_email);
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, username, password_hash, full_name, email, role, is_system_admin)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, 
        mfa_secret, mfa_enabled_at, password_changed_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(username)
    .bind(password_hash)
    .bind("Test User")
    .bind(unique_email)
    .bind(UserRole::Employee.as_str())
    .bind(false)
    .fetch_one(pool)
    .await
    .expect("create test user")
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
