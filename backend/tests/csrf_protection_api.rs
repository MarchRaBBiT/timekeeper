/// CSRF protection regression tests.
///
/// Verifies that cookie-authenticated mutation endpoints reject requests
/// that lack a valid Origin/Referer, while Bearer-token clients pass through.
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware as axum_middleware,
    routing::post,
    Extension, Router,
};
use serde_json::json;
use timekeeper_backend::{
    middleware::{csrf::csrf_check, request_id::RequestId},
    models::user::{User, UserRole},
    services::audit_log::{AuditLogService, AuditLogServiceTrait},
    state::AppState,
    utils::jwt::Claims,
};
use tower::ServiceExt;

mod support;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn csrf_test_router(pool: sqlx::PgPool) -> Router {
    // A minimal handler that always returns 200 OK; we only care about the
    // CSRF middleware verdict, not what the handler itself does.
    async fn ok_handler(Extension(_user): Extension<User>) -> StatusCode {
        StatusCode::OK
    }

    let config = support::test_config();
    let state = AppState::new(pool.clone(), None, None, None, config);

    Router::new()
        .route("/api/test/mutation", post(ok_handler))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            csrf_check,
        ))
        .layer(Extension(dummy_user()))
        .with_state(state)
}

fn dummy_user() -> User {
    use chrono::Utc;
    User {
        id: timekeeper_backend::types::UserId::new(),
        username: "testuser".into(),
        password_hash: "hash".into(),
        full_name: "Test User".into(),
        email: "test@example.com".into(),
        role: UserRole::Employee,
        is_system_admin: false,
        mfa_secret: None,
        mfa_enabled_at: None,
        password_changed_at: Utc::now(),
        failed_login_attempts: 0,
        locked_until: None,
        lock_reason: None,
        lockout_count: 0,
        department_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn dummy_claims(user: &User) -> Claims {
    use chrono::Utc;
    Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: "employee".into(),
        jti: uuid::Uuid::new_v4().to_string(),
        exp: (Utc::now() + chrono::Duration::hours(1)).timestamp(),
        iat: Utc::now().timestamp(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Cookie-auth POST with no Origin/Referer header must be rejected (403).
#[tokio::test]
async fn csrf_blocks_cookie_post_without_origin() {
    let pool = support::test_pool().await;
    let app = csrf_test_router(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/test/mutation")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Cookie-auth POST with wrong origin must be rejected (403).
#[tokio::test]
async fn csrf_blocks_cookie_post_with_wrong_origin() {
    let pool = support::test_pool().await;
    let app = csrf_test_router(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/test/mutation")
                .header("Origin", "http://evil.example.com")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Cookie-auth POST with the allowed origin must pass (200).
#[tokio::test]
async fn csrf_allows_cookie_post_with_correct_origin() {
    let pool = support::test_pool().await;
    let app = csrf_test_router(pool);

    // test_config() sets cors_allow_origins = ["http://localhost:8000"]
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/test/mutation")
                .header("Origin", "http://localhost:8000")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// Bearer-token POST with no Origin must pass (200); programmatic clients
/// are not subject to cookie-based CSRF.
#[tokio::test]
async fn csrf_allows_bearer_post_without_origin() {
    let pool = support::test_pool().await;
    let app = csrf_test_router(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/test/mutation")
                .header(header::AUTHORIZATION, "Bearer some.jwt.token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// GET requests must never be CSRF-blocked (safe method).
#[tokio::test]
async fn csrf_allows_get_without_origin() {
    async fn ok_get() -> StatusCode {
        StatusCode::OK
    }

    let pool = support::test_pool().await;
    let config = support::test_config();
    let state = AppState::new(pool.clone(), None, None, None, config);

    let app = Router::new()
        .route("/api/test/read", axum::routing::get(ok_get))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            csrf_check,
        ))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/test/read")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// Referer header is accepted as a fallback when Origin is absent.
#[tokio::test]
async fn csrf_allows_cookie_post_with_referer_fallback() {
    let pool = support::test_pool().await;
    let app = csrf_test_router(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/test/mutation")
                .header("Referer", "http://localhost:8000/some/page")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Integration: previously-unprotected endpoints now require origin
// ---------------------------------------------------------------------------

fn logout_router_with_csrf(pool: sqlx::PgPool, user: User, claims: Claims) -> Router {
    use timekeeper_backend::handlers::auth;
    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let audit_log_service: std::sync::Arc<dyn AuditLogServiceTrait> =
        std::sync::Arc::new(AuditLogService::new(pool));
    Router::new()
        .route("/api/auth/logout", post(auth::logout))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            csrf_check,
        ))
        .layer(Extension(user))
        .layer(Extension(claims))
        .layer(Extension(RequestId("test".to_string())))
        .layer(Extension(audit_log_service))
        .with_state(state)
}

/// POST /api/auth/logout without Origin should be CSRF-rejected (403).
#[tokio::test]
async fn logout_csrf_rejected_without_origin() {
    let pool = support::test_pool().await;
    let user = dummy_user();
    let claims = dummy_claims(&user);
    let app = logout_router_with_csrf(pool, user, claims);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"all": false}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// POST /api/auth/logout with correct Origin should pass CSRF and reach handler (not 403).
#[tokio::test]
async fn logout_csrf_passes_with_correct_origin() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let claims = dummy_claims(&user);
    let app = logout_router_with_csrf(pool, user, claims);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header("Origin", "http://localhost:8000")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"all": false}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Not 403 (CSRF) – handler runs and returns 200 or other business status
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}
