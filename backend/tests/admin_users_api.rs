use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::users,
    models::user::{CreateUser, UpdateUser, User, UserRole},
    state::AppState,
    types::UserId,
    utils::encryption::{encrypt_pii, hash_email},
};
use tower::ServiceExt;

mod support;

use support::{create_test_token, seed_user, test_config, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn test_router_with_state(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/users",
            get(users::get_users).post(users::create_user),
        )
        .route(
            "/api/admin/users/{id}",
            get(users::update_user).delete(users::delete_user),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn get_users_list(pool: &PgPool, user: &User) -> StatusCode {
    let token = create_test_token(user.id, user.role.clone());
    let app = test_router_with_state(pool.clone(), user.clone());

    let request = Request::builder()
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    response.status()
}

async fn get_users_payload(pool: &PgPool, user: &User) -> (String, serde_json::Value) {
    let token = create_test_token(user.id, user.role.clone());
    let app = test_router_with_state(pool.clone(), user.clone());

    let request = Request::builder()
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    if response.status() != StatusCode::OK {
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024 * 64)
            .await
            .expect("read error body");
        panic!(
            "unexpected status: {} body: {}",
            status,
            String::from_utf8_lossy(&body)
        );
    }
    let masked_header = response
        .headers()
        .get("x-pii-masked")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read response body");
    let payload = serde_json::from_slice(&body).expect("parse json");
    (masked_header, payload)
}

#[tokio::test]
async fn test_admin_can_list_all_users() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let _regular = seed_user(&pool, UserRole::Employee, false).await;

    let status = get_users_list(&pool, &admin).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_non_system_admin_user_list_masks_pii() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let _regular = seed_user(&pool, UserRole::Employee, false).await;

    let (masked_header, payload) = get_users_payload(&pool, &admin).await;
    let first = payload
        .as_array()
        .and_then(|items| items.first())
        .expect("first user");
    let full_name = first
        .get("full_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let email = first
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    assert_eq!(masked_header, "true");
    assert!(full_name.contains('*'));
    assert!(email.contains("***@"));
}

#[tokio::test]
async fn test_non_admin_cannot_list_users() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let status = get_users_list(&pool, &employee).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_system_admin_can_create_user() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;

    let new_user = CreateUser {
        username: "newemployee".to_string(),
        password: "SecureP@ssw0rd123".to_string(),
        full_name: "New Employee".to_string(),
        email: "new.employee@example.com".to_string(),
        role: UserRole::Employee,
        is_system_admin: false,
    };

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users", axum::routing::post(users::create_user))
        .layer(Extension(sysadmin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(new_user).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_regular_admin_cannot_create_user() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let new_user = CreateUser {
        username: "newemployee".to_string(),
        password: "SecureP@ssw0rd123".to_string(),
        full_name: "New Employee".to_string(),
        email: "new.employee@example.com".to_string(),
        role: UserRole::Employee,
        is_system_admin: false,
    };

    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users", axum::routing::post(users::create_user))
        .layer(Extension(admin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(new_user).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_cannot_create_user_with_duplicate_username() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let existing = seed_user(&pool, UserRole::Employee, false).await;
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;

    let new_user = CreateUser {
        username: existing.username.clone(),
        password: "SecureP@ssw0rd123".to_string(),
        full_name: "Another User".to_string(),
        email: "another@example.com".to_string(),
        role: UserRole::Employee,
        is_system_admin: false,
    };

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users", axum::routing::post(users::create_user))
        .layer(Extension(sysadmin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(new_user).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_user_rejects_password_without_required_symbol() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;

    let new_user = CreateUser {
        username: "nosymboluser".to_string(),
        password: "ValidPassword123".to_string(),
        full_name: "No Symbol".to_string(),
        email: "nosymbol@example.com".to_string(),
        role: UserRole::Employee,
        is_system_admin: false,
    };

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users", axum::routing::post(users::create_user))
        .layer(Extension(sysadmin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/users")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(new_user).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_system_admin_can_update_user() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let target = seed_user(&pool, UserRole::Employee, false).await;

    let update = UpdateUser {
        full_name: Some("Updated Name".to_string()),
        email: None,
        role: None,
        is_system_admin: None,
    };

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/users/{id}",
            axum::routing::put(users::update_user),
        )
        .layer(Extension(sysadmin))
        .with_state(state);

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/admin/users/{}", target.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(update).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cannot_update_with_duplicate_email() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let user1 = seed_user(&pool, UserRole::Employee, false).await;
    let user2 = seed_user(&pool, UserRole::Employee, false).await;

    let update = UpdateUser {
        full_name: None,
        email: Some(user1.email.clone()),
        role: None,
        is_system_admin: None,
    };

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/users/{id}",
            axum::routing::put(users::update_user),
        )
        .layer(Extension(sysadmin))
        .with_state(state);

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/admin/users/{}", user2.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(update).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cannot_delete_self() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/users/{id}",
            axum::routing::delete(users::delete_user),
        )
        .layer(Extension(sysadmin.clone()))
        .with_state(state);

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/admin/users/{}?hard=false", sysadmin.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_regular_admin_cannot_reset_mfa() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let _target = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/users/{id}/reset-mfa",
            axum::routing::post(users::reset_user_mfa),
        )
        .layer(Extension(admin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/users/{}/reset-mfa", UserId::new()))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_system_admin_reset_mfa_rejects_non_uuid_without_partial_update() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/users/{id}/reset-mfa",
            axum::routing::post(users::reset_user_mfa),
        )
        .layer(Extension(sysadmin))
        .with_state(state);

    let legacy_user_id = "legacy-user-id-302";
    let encrypted_full_name =
        encrypt_pii("Legacy User", &test_config()).expect("encrypt legacy full name");
    let encrypted_email =
        encrypt_pii("legacy302@example.com", &test_config()).expect("encrypt legacy email");
    let email_hash = hash_email("legacy302@example.com", &test_config());
    let now = Utc::now();
    let refresh_token_id = format!("rt-{}", uuid::Uuid::new_v4());

    sqlx::query(
        "INSERT INTO users \
         (id, username, password_hash, full_name_enc, email_enc, email_hash, role, is_system_admin, \
          mfa_secret_enc, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, \
          lock_reason, lockout_count, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 0, NULL, NULL, 0, $12, $13)",
    )
    .bind(legacy_user_id)
    .bind("legacy-302")
    .bind("dummy-password-hash")
    .bind(encrypted_full_name)
    .bind(encrypted_email)
    .bind(email_hash)
    .bind("employee")
    .bind(false)
    .bind("legacy-mfa-secret")
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert legacy user");

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&refresh_token_id)
    .bind(legacy_user_id)
    .bind("legacy-token-hash")
    .bind(now + Duration::days(7))
    .execute(&pool)
    .await
    .expect("insert refresh token");

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/users/{legacy_user_id}/reset-mfa"))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let remaining_mfa_secret =
        sqlx::query_scalar::<_, Option<String>>("SELECT mfa_secret_enc FROM users WHERE id = $1")
            .bind(legacy_user_id)
            .fetch_one(&pool)
            .await
            .expect("fetch mfa secret");
    assert_eq!(remaining_mfa_secret.as_deref(), Some("legacy-mfa-secret"));

    let refresh_token_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
            .bind(legacy_user_id)
            .fetch_one(&pool)
            .await
            .expect("count refresh tokens");
    assert_eq!(refresh_token_count, 1);
}

#[tokio::test]
async fn test_system_admin_can_reset_mfa_and_revoke_refresh_tokens() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let target = seed_user(&pool, UserRole::Employee, false).await;

    sqlx::query(
        "UPDATE users SET mfa_secret_enc = $1, mfa_enabled_at = NOW(), updated_at = NOW() WHERE id = $2",
    )
    .bind("dummy-secret")
    .bind(target.id.to_string())
    .execute(&pool)
    .await
    .expect("seed mfa state");

    let refresh_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, NOW() + INTERVAL '7 days')")
        .bind(&refresh_id)
        .bind(target.id.to_string())
        .bind("hash")
        .execute(&pool)
        .await
        .expect("seed refresh token");

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let app = Router::new()
        .route(
            "/api/admin/users/{id}/reset-mfa",
            axum::routing::post(users::reset_user_mfa),
        )
        .layer(Extension(sysadmin))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/users/{}/reset-mfa", target.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mfa_reset = sqlx::query_scalar::<_, bool>(
        "SELECT mfa_secret_enc IS NULL AND mfa_enabled_at IS NULL FROM users WHERE id = $1",
    )
    .bind(target.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("check mfa state");
    assert!(mfa_reset);

    let remaining_refresh_tokens =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
            .bind(target.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("count refresh tokens");
    assert_eq!(remaining_refresh_tokens, 0);
}

#[tokio::test]
async fn test_system_admin_can_unlock_user_account() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let target = seed_user(&pool, UserRole::Employee, false).await;

    sqlx::query(
        "UPDATE users \
         SET failed_login_attempts = 0, locked_until = NOW() + INTERVAL '15 minutes', \
             lock_reason = 'too_many_failed_attempts', lockout_count = 1 \
         WHERE id = $1",
    )
    .bind(target.id.to_string())
    .execute(&pool)
    .await
    .expect("lock target user");

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = Router::new()
        .route(
            "/api/admin/users/{id}/unlock",
            axum::routing::post(users::unlock_user_account),
        )
        .layer(Extension(sysadmin))
        .with_state(AppState::new(pool.clone(), None, None, None, test_config()));

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/users/{}/unlock", target.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let unlocked = sqlx::query_scalar::<_, bool>(
        "SELECT failed_login_attempts = 0 \
             AND locked_until IS NULL \
             AND lock_reason IS NULL \
             AND lockout_count = 0 \
         FROM users WHERE id = $1",
    )
    .bind(target.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("fetch unlocked status");
    assert!(unlocked);
}

#[tokio::test]
async fn test_regular_admin_cannot_unlock_user_account() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let target = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = Router::new()
        .route(
            "/api/admin/users/{id}/unlock",
            axum::routing::post(users::unlock_user_account),
        )
        .layer(Extension(admin))
        .with_state(AppState::new(pool, None, None, None, test_config()));

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/users/{}/unlock", target.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_user_rejects_invalid_id_and_missing_user() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = Router::new()
        .route(
            "/api/admin/users/{id}",
            axum::routing::delete(users::delete_user),
        )
        .layer(Extension(sysadmin))
        .with_state(AppState::new(pool, None, None, None, test_config()));

    let invalid = Request::builder()
        .method("DELETE")
        .uri("/api/admin/users/not-a-uuid?hard=true")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build invalid request");
    let invalid_response = app
        .clone()
        .oneshot(invalid)
        .await
        .expect("call invalid request");
    assert_eq!(invalid_response.status(), StatusCode::BAD_REQUEST);

    let missing = Request::builder()
        .method("DELETE")
        .uri(format!("/api/admin/users/{}?hard=true", UserId::new()))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build missing request");
    let missing_response = app.oneshot(missing).await.expect("call missing request");
    assert_eq!(missing_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_archived_user_endpoints_cover_not_found_conflict_and_forbidden() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let regular_admin = seed_user(&pool, UserRole::Admin, false).await;
    let target = seed_user(&pool, UserRole::Employee, false).await;
    let email_conflict_target = seed_user(&pool, UserRole::Employee, false).await;

    let app_sysadmin = Router::new()
        .route("/api/admin/users", axum::routing::post(users::create_user))
        .route(
            "/api/admin/users/{id}",
            axum::routing::delete(users::delete_user),
        )
        .route(
            "/api/admin/archived-users",
            axum::routing::get(users::get_archived_users),
        )
        .route(
            "/api/admin/archived-users/{id}/restore",
            axum::routing::post(users::restore_archived_user),
        )
        .route(
            "/api/admin/archived-users/{id}",
            axum::routing::delete(users::delete_archived_user),
        )
        .layer(Extension(sysadmin.clone()))
        .with_state(AppState::new(pool.clone(), None, None, None, test_config()));

    let sysadmin_token = create_test_token(sysadmin.id, sysadmin.role.clone());

    let soft_delete = Request::builder()
        .method("DELETE")
        .uri(format!("/api/admin/users/{}?hard=false", target.id))
        .header("Authorization", format!("Bearer {}", sysadmin_token))
        .body(Body::empty())
        .expect("build soft delete");
    let soft_delete_response = app_sysadmin
        .clone()
        .oneshot(soft_delete)
        .await
        .expect("call soft delete");
    assert_eq!(soft_delete_response.status(), StatusCode::OK);

    let soft_delete_email_conflict_target = Request::builder()
        .method("DELETE")
        .uri(format!(
            "/api/admin/users/{}?hard=false",
            email_conflict_target.id
        ))
        .header("Authorization", format!("Bearer {}", sysadmin_token))
        .body(Body::empty())
        .expect("build soft delete for email conflict target");
    let soft_delete_email_conflict_target_response = app_sysadmin
        .clone()
        .oneshot(soft_delete_email_conflict_target)
        .await
        .expect("call soft delete for email conflict target");
    assert_eq!(
        soft_delete_email_conflict_target_response.status(),
        StatusCode::OK
    );

    let conflict_email = format!("conflict-{}@example.com", UserId::new());
    let config = test_config();
    let inserted_conflict = sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name_enc, email_enc, email_hash, role, is_system_admin, \
         mfa_secret_enc, mfa_enabled_at, password_changed_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, NULL, NOW(), NOW(), NOW())",
    )
    .bind(UserId::new().to_string())
    .bind(&target.username)
    .bind("hash")
    .bind(encrypt_pii("Conflict User", &config).expect("encrypt full_name"))
    .bind(encrypt_pii(&conflict_email, &config).expect("encrypt email"))
    .bind(hash_email(&conflict_email, &config))
    .bind("employee")
    .bind(false)
    .execute(&pool)
    .await
    .expect("insert conflict user");
    assert_eq!(inserted_conflict.rows_affected(), 1);

    let restore_conflict = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/archived-users/{}/restore", target.id))
        .header("Authorization", format!("Bearer {}", sysadmin_token))
        .body(Body::empty())
        .expect("build restore conflict request");
    let restore_conflict_response = app_sysadmin
        .clone()
        .oneshot(restore_conflict)
        .await
        .expect("call restore conflict");
    assert_eq!(restore_conflict_response.status(), StatusCode::BAD_REQUEST);

    let inserted_email_conflict = sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name_enc, email_enc, email_hash, role, is_system_admin, \
         mfa_secret_enc, mfa_enabled_at, password_changed_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, NULL, NOW(), NOW(), NOW())",
    )
    .bind(UserId::new().to_string())
    .bind(format!("email-conflict-{}", UserId::new()))
    .bind("hash")
    .bind(encrypt_pii("Email Conflict User", &config).expect("encrypt full_name"))
    .bind(encrypt_pii(&email_conflict_target.email, &config).expect("encrypt email"))
    .bind(hash_email(&email_conflict_target.email, &config))
    .bind("employee")
    .bind(false)
    .execute(&pool)
    .await
    .expect("insert email conflict user");
    assert_eq!(inserted_email_conflict.rows_affected(), 1);

    let restore_email_conflict = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/admin/archived-users/{}/restore",
            email_conflict_target.id
        ))
        .header("Authorization", format!("Bearer {}", sysadmin_token))
        .body(Body::empty())
        .expect("build restore email conflict request");
    let restore_email_conflict_response = app_sysadmin
        .clone()
        .oneshot(restore_email_conflict)
        .await
        .expect("call restore email conflict");
    assert_eq!(
        restore_email_conflict_response.status(),
        StatusCode::BAD_REQUEST
    );

    let delete_missing_archived = Request::builder()
        .method("DELETE")
        .uri(format!("/api/admin/archived-users/{}", UserId::new()))
        .header("Authorization", format!("Bearer {}", sysadmin_token))
        .body(Body::empty())
        .expect("build delete missing archived request");
    let delete_missing_archived_response = app_sysadmin
        .clone()
        .oneshot(delete_missing_archived)
        .await
        .expect("call delete missing archived");
    assert_eq!(
        delete_missing_archived_response.status(),
        StatusCode::NOT_FOUND
    );

    let app_regular_admin = Router::new()
        .route(
            "/api/admin/archived-users",
            axum::routing::get(users::get_archived_users),
        )
        .layer(Extension(regular_admin.clone()))
        .with_state(AppState::new(pool, None, None, None, test_config()));
    let regular_token = create_test_token(regular_admin.id, regular_admin.role.clone());

    let forbidden_list = Request::builder()
        .uri("/api/admin/archived-users")
        .header("Authorization", format!("Bearer {}", regular_token))
        .body(Body::empty())
        .expect("build forbidden list request");
    let forbidden_list_response = app_regular_admin
        .oneshot(forbidden_list)
        .await
        .expect("call forbidden list");
    assert_eq!(forbidden_list_response.status(), StatusCode::FORBIDDEN);
}
