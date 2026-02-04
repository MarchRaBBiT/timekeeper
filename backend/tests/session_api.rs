use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::{sessions, admin::sessions as admin_sessions},
    middleware::request_id::RequestId,
    models::user::{User, UserRole},
    services::audit_log::{AuditLogService, AuditLogServiceTrait},
    state::AppState,
    utils::jwt::Claims,
};
use tower::ServiceExt;
use std::sync::Arc;
use chrono::Utc;

mod support;

use support::{
    create_test_token, seed_user, seed_active_session, test_config, test_pool,
};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD.get_or_init(|| tokio::sync::Mutex::new(())).lock().await
}

fn create_test_claims(user_id: timekeeper_backend::types::UserId, jti: &str) -> Claims {
    Claims {
        sub: user_id.to_string(),
        username: "testuser".to_string(),
        role: "employee".to_string(),
        exp: Utc::now().timestamp() + 3600,
        iat: Utc::now().timestamp(),
        jti: jti.to_string(),
    }
}

fn test_router_user_sessions(pool: PgPool, user: User, claims: Claims) -> Router {
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let audit_log_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool));
    Router::new()
        .route("/api/sessions", axum::routing::get(sessions::list_sessions))
        .route("/api/sessions/{id}", axum::routing::delete(sessions::revoke_session))
        .layer(Extension(user))
        .layer(Extension(claims))
        .layer(Extension(RequestId("test-request-id".to_string())))
        .layer(Extension(audit_log_service))
        .with_state(state)
}

fn test_router_admin_sessions(pool: PgPool, user: User, claims: Claims) -> Router {
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let audit_log_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool));
    Router::new()
        .route("/api/admin/users/{user_id}/sessions", axum::routing::get(admin_sessions::list_user_sessions))
        .route("/api/admin/sessions/{id}", axum::routing::delete(admin_sessions::revoke_session))
        .layer(Extension(user))
        .layer(Extension(claims))
        .layer(Extension(RequestId("test-request-id".to_string())))
        .layer(Extension(audit_log_service))
        .with_state(state)
}

#[tokio::test]
async fn test_list_sessions_returns_user_sessions() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let refresh_token_id = uuid::Uuid::new_v4().to_string();
    let access_jti = uuid::Uuid::new_v4().to_string();
    let _session_id =
        seed_active_session(&pool, employee.id, &refresh_token_id, Some(&access_jti)).await;
    
    let claims = create_test_claims(employee.id, &access_jti);
    let app = test_router_user_sessions(pool.clone(), employee.clone(), claims);
    
    let request = Request::builder()
        .uri("/api/sessions")
        .header("Authorization", format!("Bearer {}", create_test_token(employee.id, employee.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_revoke_other_session_succeeds() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let refresh_token_id = uuid::Uuid::new_v4().to_string();
    let access_jti = uuid::Uuid::new_v4().to_string();
    let session_id =
        seed_active_session(&pool, employee.id, &refresh_token_id, Some(&access_jti)).await;
    
    let current_jti = uuid::Uuid::new_v4().to_string();
    let claims = create_test_claims(employee.id, &current_jti);
    let app = test_router_user_sessions(pool.clone(), employee.clone(), claims);
    
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/sessions/{}", session_id))
        .header("Authorization", format!("Bearer {}", create_test_token(employee.id, employee.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_revoke_current_session_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let refresh_token_id = uuid::Uuid::new_v4().to_string();
    let access_jti = uuid::Uuid::new_v4().to_string();
    let session_id =
        seed_active_session(&pool, employee.id, &refresh_token_id, Some(&access_jti)).await;
    
    let claims = create_test_claims(employee.id, &access_jti);
    let app = test_router_user_sessions(pool.clone(), employee.clone(), claims);
    
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/sessions/{}", session_id))
        .header("Authorization", format!("Bearer {}", create_test_token(employee.id, employee.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_revoke_other_user_session_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee1 = seed_user(&pool, UserRole::Employee, false).await;
    let employee2 = seed_user(&pool, UserRole::Employee, false).await;
    let refresh_token_id = uuid::Uuid::new_v4().to_string();
    let access_jti = uuid::Uuid::new_v4().to_string();
    let session_id =
        seed_active_session(&pool, employee2.id, &refresh_token_id, Some(&access_jti)).await;
    
    let current_jti = uuid::Uuid::new_v4().to_string();
    let claims = create_test_claims(employee1.id, &current_jti);
    let app = test_router_user_sessions(pool.clone(), employee1.clone(), claims);
    
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/sessions/{}", session_id))
        .header("Authorization", format!("Bearer {}", create_test_token(employee1.id, employee1.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_revoke_nonexistent_session_returns_not_found() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let current_jti = uuid::Uuid::new_v4().to_string();
    let claims = create_test_claims(employee.id, &current_jti);
    let app = test_router_user_sessions(pool.clone(), employee.clone(), claims);
    
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/sessions/{}", uuid::Uuid::new_v4()))
        .header("Authorization", format!("Bearer {}", create_test_token(employee.id, employee.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_can_list_user_sessions() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let refresh_token_id = uuid::Uuid::new_v4().to_string();
    let access_jti = uuid::Uuid::new_v4().to_string();
    let _session_id =
        seed_active_session(&pool, employee.id, &refresh_token_id, Some(&access_jti)).await;
    
    let current_jti = uuid::Uuid::new_v4().to_string();
    let claims = create_test_claims(admin.id, &current_jti);
    let app = test_router_admin_sessions(pool.clone(), admin.clone(), claims);
    
    let request = Request::builder()
        .uri(format!("/api/admin/users/{}/sessions", employee.id))
        .header("Authorization", format!("Bearer {}", create_test_token(admin.id, admin.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_admin_can_revoke_any_session() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let refresh_token_id = uuid::Uuid::new_v4().to_string();
    let access_jti = uuid::Uuid::new_v4().to_string();
    let session_id =
        seed_active_session(&pool, employee.id, &refresh_token_id, Some(&access_jti)).await;
    
    let current_jti = uuid::Uuid::new_v4().to_string();
    let claims = create_test_claims(admin.id, &current_jti);
    let app = test_router_admin_sessions(pool.clone(), admin.clone(), claims);
    
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/admin/sessions/{}", session_id))
        .header("Authorization", format!("Bearer {}", create_test_token(admin.id, admin.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_employee_cannot_list_other_user_sessions() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee1 = seed_user(&pool, UserRole::Employee, false).await;
    let employee2 = seed_user(&pool, UserRole::Employee, false).await;
    
    let current_jti = uuid::Uuid::new_v4().to_string();
    let claims = create_test_claims(employee1.id, &current_jti);
    let app = test_router_admin_sessions(pool.clone(), employee1.clone(), claims);
    
    let request = Request::builder()
        .uri(format!("/api/admin/users/{}/sessions", employee2.id))
        .header("Authorization", format!("Bearer {}", create_test_token(employee1.id, employee1.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_sessions_empty_when_no_sessions() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let current_jti = uuid::Uuid::new_v4().to_string();
    let claims = create_test_claims(employee.id, &current_jti);
    let app = test_router_user_sessions(pool.clone(), employee.clone(), claims);
    
    let request = Request::builder()
        .uri("/api/sessions")
        .header("Authorization", format!("Bearer {}", create_test_token(employee.id, employee.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_revoke_session_with_empty_id_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let current_jti = uuid::Uuid::new_v4().to_string();
    let claims = create_test_claims(employee.id, &current_jti);
    let app = test_router_user_sessions(pool.clone(), employee.clone(), claims);
    
    let request = Request::builder()
        .method("DELETE")
        .uri("/api/sessions/%20")
        .header("Authorization", format!("Bearer {}", create_test_token(employee.id, employee.role.clone())))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
