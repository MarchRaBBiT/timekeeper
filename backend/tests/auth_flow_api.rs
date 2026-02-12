use axum::{
    body::Body,
    http::{header, HeaderMap, Request, StatusCode},
    routing::{get, post, put},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use timekeeper_backend::{
    config::Config,
    handlers::auth,
    middleware::request_id::RequestId,
    models::user::{User, UserRole},
    repositories::auth as auth_repo,
    services::audit_log::{AuditLogService, AuditLogServiceTrait},
    state::AppState,
    utils::{
        cookies::{ACCESS_COOKIE_NAME, REFRESH_COOKIE_NAME},
        jwt::{decode_refresh_token, verify_access_token, Claims},
    },
};
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
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn auth_router_with_config(pool: PgPool, config: Config) -> Router {
    let state = AppState::new(pool.clone(), None, None, None, config);
    let audit_log_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool));
    Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/refresh", post(auth::refresh))
        .layer(Extension(RequestId("test-request-id".to_string())))
        .layer(Extension(audit_log_service))
        .with_state(state)
}

fn auth_router(pool: PgPool) -> Router {
    auth_router_with_config(pool, support::test_config())
}

fn logout_router(pool: PgPool, user: User, claims: Claims) -> Router {
    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let audit_log_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool));
    Router::new()
        .route("/api/auth/logout", post(auth::logout))
        .layer(Extension(user))
        .layer(Extension(claims))
        .layer(Extension(RequestId("test-request-id".to_string())))
        .layer(Extension(audit_log_service))
        .with_state(state)
}

fn me_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, support::test_config());
    Router::new()
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/me", put(auth::update_profile))
        .layer(Extension(user))
        .with_state(state)
}

fn extract_set_cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .find_map(|value| {
            let value = value.to_str().ok()?;
            let token = value.strip_prefix(&prefix)?.split(';').next()?.trim();
            if token.is_empty() {
                None
            } else {
                Some(token.to_string())
            }
        })
}

async fn count_refresh_tokens(pool: &PgPool, user_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("count refresh tokens")
}

async fn count_active_access_tokens(pool: &PgPool, user_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM active_access_tokens WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("count active access tokens")
}

async fn count_active_sessions(pool: &PgPool, user_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM active_sessions WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("count active sessions")
}

async fn refresh_token_exists(pool: &PgPool, token_id: &str) -> bool {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM refresh_tokens WHERE id = $1)")
        .bind(token_id)
        .fetch_one(pool)
        .await
        .expect("refresh token exists")
}

async fn fetch_lock_state(
    pool: &PgPool,
    user_id: &str,
) -> (i32, Option<chrono::DateTime<chrono::Utc>>, i32) {
    sqlx::query_as::<_, (i32, Option<chrono::DateTime<chrono::Utc>>, i32)>(
        "SELECT failed_login_attempts, locked_until, lockout_count FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("fetch lock state")
}

#[tokio::test]
async fn login_sets_cookies_and_persists_tokens() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let password = "Password123!";
    let user = support::seed_user_with_password(&pool, UserRole::Employee, false, password).await;
    let app = auth_router(pool.clone());

    let payload = json!({
        "username": user.username.clone(),
        "password": password,
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .expect("build login request"),
        )
        .await
        .expect("login request");

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    let access_token =
        extract_set_cookie_value(headers, ACCESS_COOKIE_NAME).expect("access cookie");
    let refresh_token =
        extract_set_cookie_value(headers, REFRESH_COOKIE_NAME).expect("refresh cookie");
    assert!(!access_token.is_empty());
    assert!(!refresh_token.is_empty());

    let user_id = user.id.to_string();
    assert_eq!(count_refresh_tokens(&pool, &user_id).await, 1);
    assert_eq!(count_active_access_tokens(&pool, &user_id).await, 1);
    assert_eq!(count_active_sessions(&pool, &user_id).await, 1);
}

#[tokio::test]
async fn login_rejects_invalid_password() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let user =
        support::seed_user_with_password(&pool, UserRole::Employee, false, "Correct123!").await;
    let app = auth_router(pool.clone());

    let payload = json!({
        "username": user.username.clone(),
        "password": "Wrong123!",
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .expect("build login request"),
        )
        .await
        .expect("login request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let user_id = user.id.to_string();
    assert_eq!(count_refresh_tokens(&pool, &user_id).await, 0);
    assert_eq!(count_active_access_tokens(&pool, &user_id).await, 0);
    assert_eq!(count_active_sessions(&pool, &user_id).await, 0);
}

#[tokio::test]
async fn login_locks_account_after_reaching_failure_threshold() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let password = "Correct123!";
    let user = support::seed_user_with_password(&pool, UserRole::Employee, false, password).await;
    let mut config = support::test_config();
    config.account_lockout_threshold = 3;
    config.account_lockout_duration_minutes = 15;
    config.account_lockout_backoff_enabled = true;
    config.account_lockout_max_duration_hours = 24;
    let app = auth_router_with_config(pool.clone(), config);

    for _ in 0..3 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "username": user.username.clone(),
                            "password": "Wrong123!",
                        })
                        .to_string(),
                    ))
                    .expect("build login request"),
            )
            .await
            .expect("login request");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let (failed_attempts, locked_until, lockout_count) =
        fetch_lock_state(&pool, &user.id.to_string()).await;
    assert_eq!(failed_attempts, 0);
    assert!(locked_until.is_some());
    assert_eq!(lockout_count, 1);

    let locked_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": user.username.clone(),
                        "password": password,
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");
    assert_eq!(locked_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_success_clears_login_failure_state() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let password = "Correct123!";
    let user = support::seed_user_with_password(&pool, UserRole::Employee, false, password).await;
    let app = auth_router(pool.clone());

    let failed_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": user.username.clone(),
                        "password": "Wrong123!",
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");
    assert_eq!(failed_response.status(), StatusCode::UNAUTHORIZED);

    let success_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": user.username.clone(),
                        "password": password,
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");
    assert_eq!(success_response.status(), StatusCode::OK);

    let (failed_attempts, locked_until, lockout_count) =
        fetch_lock_state(&pool, &user.id.to_string()).await;
    assert_eq!(failed_attempts, 0);
    assert!(locked_until.is_none());
    assert_eq!(lockout_count, 0);
}

#[tokio::test]
async fn login_enforces_session_limit() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let password = "Password123!";
    let user = support::seed_user_with_password(&pool, UserRole::Employee, false, password).await;

    let refresh_token_id_1 = Uuid::new_v4().to_string();
    let refresh_token_id_2 = Uuid::new_v4().to_string();
    support::seed_active_session(&pool, user.id, &refresh_token_id_1, None).await;
    support::seed_active_session(&pool, user.id, &refresh_token_id_2, None).await;

    let mut config = support::test_config();
    config.max_concurrent_sessions = 2;

    let response = auth_router_with_config(pool.clone(), config)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": user.username.clone(),
                        "password": password,
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    let refresh_token =
        extract_set_cookie_value(headers, REFRESH_COOKIE_NAME).expect("refresh cookie");
    let (new_refresh_id, _) =
        decode_refresh_token(&refresh_token).expect("decode new refresh token");

    assert_eq!(count_active_sessions(&pool, &user.id.to_string()).await, 2);
    assert_eq!(count_refresh_tokens(&pool, &user.id.to_string()).await, 2);
    assert!(refresh_token_exists(&pool, &new_refresh_id).await);

    let mut remaining_seeded = 0;
    if refresh_token_exists(&pool, &refresh_token_id_1).await {
        remaining_seeded += 1;
    }
    if refresh_token_exists(&pool, &refresh_token_id_2).await {
        remaining_seeded += 1;
    }
    assert_eq!(remaining_seeded, 1);
}

#[tokio::test]
async fn me_returns_current_user() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let user_id = user.id.to_string();

    let response = me_router(pool, user.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .body(Body::empty())
                .expect("build me request"),
        )
        .await
        .expect("me request");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = support::response_json(response).await;
    assert_eq!(payload["id"].as_str(), Some(user_id.as_str()));
    assert_eq!(payload["username"].as_str(), Some(user.username.as_str()));
    assert_eq!(payload["full_name"].as_str(), Some(user.full_name.as_str()));
    assert_eq!(payload["email"].as_str(), Some(user.email.as_str()));
    assert_eq!(payload["role"].as_str(), Some(user.role.as_str()));
    assert_eq!(
        payload["is_system_admin"].as_bool(),
        Some(user.is_system_admin)
    );
    assert_eq!(payload["mfa_enabled"].as_bool(), Some(false));
}

#[tokio::test]
async fn update_profile_updates_full_name_and_email() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let new_full_name = "Updated User";
    let new_email = format!("updated-{}@example.com", Uuid::new_v4());

    let response = me_router(pool.clone(), user.clone())
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/auth/me")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "full_name": new_full_name,
                        "email": new_email,
                    })
                    .to_string(),
                ))
                .expect("build update profile request"),
        )
        .await
        .expect("update profile request");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = support::response_json(response).await;
    assert_eq!(payload["full_name"].as_str(), Some(new_full_name));
    assert_eq!(payload["email"].as_str(), Some(new_email.as_str()));

    let updated = auth_repo::find_user_by_id(&pool, user.id)
        .await
        .expect("fetch updated user")
        .expect("updated user");
    assert_eq!(updated.full_name, new_full_name);
    assert_eq!(updated.email, new_email);
}

#[tokio::test]
async fn update_profile_rejects_duplicate_email() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let other = support::seed_user(&pool, UserRole::Employee, false).await;

    let response = me_router(pool, user)
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/auth/me")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "email": other.email,
                    })
                    .to_string(),
                ))
                .expect("build update profile request"),
        )
        .await
        .expect("update profile request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn refresh_rotates_tokens_and_revokes_previous_access_token() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let password = "Password123!";
    let user = support::seed_user_with_password(&pool, UserRole::Employee, false, password).await;

    let login_response = auth_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": user.username.clone(),
                        "password": password,
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");

    assert_eq!(login_response.status(), StatusCode::OK);
    let login_headers = login_response.headers();
    let old_access_token =
        extract_set_cookie_value(login_headers, ACCESS_COOKIE_NAME).expect("access cookie");
    let old_refresh_token =
        extract_set_cookie_value(login_headers, REFRESH_COOKIE_NAME).expect("refresh cookie");
    let old_claims = verify_access_token(&old_access_token, &support::test_config().jwt_secret)
        .expect("decode old access token");
    let (old_refresh_id, _) =
        decode_refresh_token(&old_refresh_token).expect("decode old refresh token");

    let refresh_response = auth_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::COOKIE,
                    format!("{REFRESH_COOKIE_NAME}={old_refresh_token}"),
                )
                .body(Body::from("{}"))
                .expect("build refresh request"),
        )
        .await
        .expect("refresh request");

    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_headers = refresh_response.headers();
    let new_access_token =
        extract_set_cookie_value(refresh_headers, ACCESS_COOKIE_NAME).expect("access cookie");
    let new_refresh_token =
        extract_set_cookie_value(refresh_headers, REFRESH_COOKIE_NAME).expect("refresh cookie");
    assert_ne!(new_refresh_token, old_refresh_token);

    let new_claims = verify_access_token(&new_access_token, &support::test_config().jwt_secret)
        .expect("decode new access token");
    let (new_refresh_id, _) =
        decode_refresh_token(&new_refresh_token).expect("decode new refresh token");

    assert!(!refresh_token_exists(&pool, &old_refresh_id).await);
    assert!(refresh_token_exists(&pool, &new_refresh_id).await);

    let old_exists = auth_repo::access_token_exists(&pool, &old_claims.jti)
        .await
        .expect("check old access token");
    let new_exists = auth_repo::access_token_exists(&pool, &new_claims.jti)
        .await
        .expect("check new access token");
    assert!(!old_exists);
    assert!(new_exists);

    let user_id = user.id.to_string();
    assert_eq!(count_active_sessions(&pool, &user_id).await, 1);
}

#[tokio::test]
async fn refresh_rejects_missing_or_malformed_token() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let response_missing = auth_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .expect("build refresh request"),
        )
        .await
        .expect("refresh request without token");

    assert_eq!(response_missing.status(), StatusCode::BAD_REQUEST);

    let response_invalid = auth_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, format!("{REFRESH_COOKIE_NAME}=not-a-token"))
                .body(Body::from("{}"))
                .expect("build refresh request"),
        )
        .await
        .expect("refresh request with malformed token");

    assert_eq!(response_invalid.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_revokes_current_session() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let password = "Password123!";
    let user = support::seed_user_with_password(&pool, UserRole::Employee, false, password).await;

    let login_response = auth_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": user.username.clone(),
                        "password": password,
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");

    assert_eq!(login_response.status(), StatusCode::OK);
    let headers = login_response.headers();
    let access_token =
        extract_set_cookie_value(headers, ACCESS_COOKIE_NAME).expect("access cookie");
    let refresh_token =
        extract_set_cookie_value(headers, REFRESH_COOKIE_NAME).expect("refresh cookie");
    let claims = verify_access_token(&access_token, &support::test_config().jwt_secret)
        .expect("decode access token");

    let response = logout_router(pool.clone(), user.clone(), claims.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "refresh_token": refresh_token,
                    })
                    .to_string(),
                ))
                .expect("build logout request"),
        )
        .await
        .expect("logout request");

    assert_eq!(response.status(), StatusCode::OK);

    let user_id = user.id.to_string();
    assert_eq!(count_refresh_tokens(&pool, &user_id).await, 0);
    assert_eq!(count_active_access_tokens(&pool, &user_id).await, 0);
    assert_eq!(count_active_sessions(&pool, &user_id).await, 0);
    let access_exists = auth_repo::access_token_exists(&pool, &claims.jti)
        .await
        .expect("check access token");
    assert!(!access_exists);
}

#[tokio::test]
async fn logout_all_revokes_all_sessions() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let password = "Password123!";
    let user = support::seed_user_with_password(&pool, UserRole::Employee, false, password).await;

    let login_response = auth_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": user.username.clone(),
                        "password": password,
                    })
                    .to_string(),
                ))
                .expect("build login request"),
        )
        .await
        .expect("login request");

    assert_eq!(login_response.status(), StatusCode::OK);
    let headers = login_response.headers();
    let access_token =
        extract_set_cookie_value(headers, ACCESS_COOKIE_NAME).expect("access cookie");
    let claims = verify_access_token(&access_token, &support::test_config().jwt_secret)
        .expect("decode access token");

    let extra_refresh_id = Uuid::new_v4().to_string();
    support::seed_active_session(&pool, user.id, &extra_refresh_id, None).await;

    let response = logout_router(pool.clone(), user.clone(), claims)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "all": true }).to_string()))
                .expect("build logout all request"),
        )
        .await
        .expect("logout all request");

    assert_eq!(response.status(), StatusCode::OK);

    let user_id = user.id.to_string();
    assert_eq!(count_refresh_tokens(&pool, &user_id).await, 0);
    assert_eq!(count_active_access_tokens(&pool, &user_id).await, 0);
    assert_eq!(count_active_sessions(&pool, &user_id).await, 0);
}
