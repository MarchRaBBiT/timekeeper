use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::auth,
    models::user::{User, UserRole},
    state::AppState,
    utils::mfa::generate_totp_secret,
};
use tower::ServiceExt;

mod support;

use support::{
    create_test_token, seed_user, test_config, test_pool,
};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD.get_or_init(|| tokio::sync::Mutex::new(())).lock().await
}

fn test_router_with_state(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route("/api/auth/mfa", axum::routing::get(auth::mfa_status))
        .route("/api/auth/mfa", axum::routing::delete(auth::mfa_disable))
        .route("/api/auth/mfa/setup", axum::routing::post(auth::mfa_setup))
        .route("/api/auth/mfa/activate", axum::routing::post(auth::mfa_activate))
        .layer(Extension(user))
        .with_state(state)
}

fn generate_test_totp_code(secret: &str) -> String {
    use totp_rs::{Algorithm, TOTP};
    use base32::Alphabet::RFC4648;
    let cleaned = secret.trim().replace(' ', "").to_uppercase();
    let secret_bytes = base32::decode(RFC4648 { padding: false }, &cleaned).unwrap();
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("Test".to_string()),
        "user".to_string(),
    ).unwrap();
    totp.generate_current().unwrap()
}

const TEST_ORIGIN: &str = "http://localhost:8000";

#[tokio::test]
async fn test_mfa_setup_returns_secret_and_qr_url() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/mfa/setup")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["secret"].as_str().is_some());
    assert!(json["otpauth_url"].as_str().is_some());
}

#[tokio::test]
async fn test_mfa_setup_already_enabled_returns_error() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let secret = generate_totp_secret();
    employee.mfa_secret = Some(secret);
    employee.mfa_enabled_at = Some(chrono::Utc::now());
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/mfa/setup")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mfa_activate_with_valid_code_succeeds() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let secret = generate_totp_secret();
    employee.mfa_secret = Some(secret.clone());
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let code = generate_test_totp_code(&secret);
    let payload = json!({"code": code});
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/mfa/activate")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_mfa_activate_with_invalid_code_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let secret = generate_totp_secret();
    employee.mfa_secret = Some(secret);
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({"code": "000000"});
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/mfa/activate")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mfa_activate_without_setup_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({"code": "123456"});
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/mfa/activate")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mfa_disable_with_valid_code_succeeds() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let secret = generate_totp_secret();
    employee.mfa_secret = Some(secret.clone());
    employee.mfa_enabled_at = Some(chrono::Utc::now());
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let code = generate_test_totp_code(&secret);
    let payload = json!({"code": code});
    let request = Request::builder()
        .method("DELETE")
        .uri("/api/auth/mfa")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_mfa_disable_with_invalid_code_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let secret = generate_totp_secret();
    employee.mfa_secret = Some(secret);
    employee.mfa_enabled_at = Some(chrono::Utc::now());
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({"code": "000000"});
    let request = Request::builder()
        .method("DELETE")
        .uri("/api/auth/mfa")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mfa_disable_without_mfa_enabled_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let payload = json!({"code": "123456"});
    let request = Request::builder()
        .method("DELETE")
        .uri("/api/auth/mfa")
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", TEST_ORIGIN)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mfa_status_returns_enabled_state() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let mut employee = seed_user(&pool, UserRole::Employee, false).await;
    let secret = generate_totp_secret();
    employee.mfa_secret = Some(secret);
    employee.mfa_enabled_at = Some(chrono::Utc::now());
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let request = Request::builder()
        .uri("/api/auth/mfa")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["enabled"], true);
}

#[tokio::test]
async fn test_mfa_status_returns_disabled_state() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());
    
    let request = Request::builder()
        .uri("/api/auth/mfa")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["enabled"], false);
}
