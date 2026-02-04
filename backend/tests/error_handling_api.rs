use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware as axum_middleware,
    Extension, Router,
};
use serde_json::json;
use timekeeper_backend::{
    middleware,
    models::user::UserRole,
    state::AppState,
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

#[tokio::test]
async fn test_invalid_json_payload_returns_bad_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let state = AppState::new(pool, None, None, None, test_config());
    let app = Router::new()
        .route("/api/requests/leave", axum::routing::post(timekeeper_backend::handlers::requests::create_leave_request))
        .layer(Extension(employee))
        .with_state(state);
    
    let request = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from("not valid json"))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_missing_required_field_returns_bad_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(employee.id, employee.role.clone());
    let state = AppState::new(pool, None, None, None, test_config());
    let app = Router::new()
        .route("/api/requests/leave", axum::routing::post(timekeeper_backend::handlers::requests::create_leave_request))
        .layer(Extension(employee))
        .with_state(state);
    
    let payload = json!({
        "leave_type": "annual"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_invalid_uuid_format_returns_bad_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let state = AppState::new(pool, None, None, None, test_config());
    let app = Router::new()
        .route("/api/admin/users/{id}", axum::routing::put(timekeeper_backend::handlers::admin::users::update_user))
        .layer(Extension(sysadmin))
        .with_state(state);
    
    let request = Request::builder()
        .method("PUT")
        .uri("/api/admin/users/not-a-uuid")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_out_of_range_pagination_params() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = Router::new()
        .route("/api/admin/audit-logs", axum::routing::get(timekeeper_backend::handlers::admin::audit_logs::list_audit_logs))
        .layer(Extension(sysadmin))
        .with_state(AppState::new(pool, None, None, None, test_config()));
    
    let request = Request::builder()
        .uri("/api/admin/audit-logs?page=-1&per_page=1000")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invalid_date_format_returns_bad_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = Router::new()
        .route("/api/admin/audit-logs", axum::routing::get(timekeeper_backend::handlers::admin::audit_logs::list_audit_logs))
        .layer(Extension(sysadmin))
        .with_state(AppState::new(pool, None, None, None, test_config()));
    
    let request = Request::builder()
        .uri("/api/admin/audit-logs?from=invalid-date")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_from_after_to_date_returns_bad_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = Router::new()
        .route("/api/admin/audit-logs", axum::routing::get(timekeeper_backend::handlers::admin::audit_logs::list_audit_logs))
        .layer(Extension(sysadmin))
        .with_state(AppState::new(pool, None, None, None, test_config()));
    
    let request = Request::builder()
        .uri("/api/admin/audit-logs?from=2024-12-31&to=2024-01-01")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_invalid_enum_value_returns_bad_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = Router::new()
        .route("/api/admin/audit-logs", axum::routing::get(timekeeper_backend::handlers::admin::audit_logs::list_audit_logs))
        .layer(Extension(sysadmin))
        .with_state(AppState::new(pool, None, None, None, test_config()));
    
    let request = Request::builder()
        .uri("/api/admin/audit-logs?result=invalid_value")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_empty_authorization_header_returns_unauthorized() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let state = AppState::new(pool, None, None, None, test_config());
    let app = Router::new()
        .route("/api/attendance/me", axum::routing::get(timekeeper_backend::handlers::attendance::get_my_attendance))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth,
        ))
        .with_state(state);
    
    let request = Request::builder()
        .uri("/api/attendance/me")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_malformed_jwt_token_returns_unauthorized() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let state = AppState::new(pool, None, None, None, test_config());
    let app = Router::new()
        .route("/api/attendance/me", axum::routing::get(timekeeper_backend::handlers::attendance::get_my_attendance))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth,
        ))
        .with_state(state);
    
    let request = Request::builder()
        .uri("/api/attendance/me")
        .header("Authorization", "Bearer invalid-token")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_expired_jwt_token_returns_unauthorized() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    // Create a user that will be associated with the token
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    // Create an expired token (expired in the past)
    use chrono::{Duration, Utc};
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use timekeeper_backend::utils::jwt::Claims;
    
    let past_time = Utc::now() - Duration::hours(2);
    let claims = Claims {
        sub: employee.id.to_string(),
        username: "testuser".to_string(),
        role: "Employee".to_string(),
        exp: past_time.timestamp(),  // Expired 2 hours ago
        iat: (past_time - Duration::hours(1)).timestamp(),
        jti: uuid::Uuid::new_v4().to_string(),
    };
    
    let secret = test_config().jwt_secret;
    let expired_token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ).expect("encode expired token");
    
    // Insert the expired token into active_access_tokens
    use timekeeper_backend::repositories::auth::{insert_active_access_token, ActiveAccessToken};
    let expired_at = Utc::now() - Duration::hours(1);
    let access_token = ActiveAccessToken {
        jti: &claims.jti,
        user_id: employee.id,
        expires_at: expired_at,
        context: None,
    };
    insert_active_access_token(&pool, &access_token).await.expect("insert expired token");
    
    let state = AppState::new(pool, None, None, None, test_config());
    let app = Router::new()
        .route("/api/attendance/me", axum::routing::get(timekeeper_backend::handlers::attendance::get_my_attendance))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth,
        ))
        .with_state(state);
    
    let request = Request::builder()
        .uri("/api/attendance/me")
        .header("Authorization", format!("Bearer {}", expired_token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
