use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::audit_logs,
    models::user::{User, UserRole},
    state::AppState,
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
            "/api/admin/audit-logs",
            axum::routing::get(audit_logs::list_audit_logs),
        )
        .route(
            "/api/admin/audit-logs/{id}",
            axum::routing::get(audit_logs::get_audit_log_detail),
        )
        .route(
            "/api/admin/audit-logs/export",
            axum::routing::get(audit_logs::export_audit_logs),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn seed_audit_log(pool: &PgPool, actor_id: timekeeper_backend::types::UserId) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO audit_logs (id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result)
        VALUES ($1, NOW(), $2, 'user', 'test_event', 'test_target', 'test-id', 'success')
        "#
    )
    .bind(&id)
    .bind(actor_id.to_string())
    .execute(pool)
    .await
    .expect("insert audit log");
    id
}

#[tokio::test]
async fn test_system_admin_can_list_audit_logs() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_audit_log(&pool, employee.id).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .uri("/api/admin/audit-logs")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!json["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_regular_admin_cannot_list_audit_logs_without_permission() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let request = Request::builder()
        .uri("/api/admin/audit-logs")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_employee_cannot_list_audit_logs() {
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
        .uri("/api/admin/audit-logs")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_system_admin_can_export_audit_logs() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_audit_log(&pool, employee.id).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .uri("/api/admin/audit-logs/export?from=2024-01-01&to=2024-01-31")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_export_with_excessive_date_range_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .uri("/api/admin/audit-logs/export?from=2024-01-01&to=2024-06-01")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_audit_logs_with_actor_filter() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_audit_log(&pool, employee.id).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .uri(format!("/api/admin/audit-logs?actor_id={}", employee.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_audit_logs_with_event_type_filter() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_audit_log(&pool, employee.id).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .uri("/api/admin/audit-logs?event_type=test_event")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_audit_logs_with_result_filter() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_audit_log(&pool, employee.id).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .uri("/api/admin/audit-logs?result=success")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invalid_result_filter_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .uri("/api/admin/audit-logs?result=invalid")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_system_admin_can_get_audit_log_detail() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let log_id = seed_audit_log(&pool, employee.id).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .uri(format!("/api/admin/audit-logs/{}", log_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_nonexistent_audit_log_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .uri(format!("/api/admin/audit-logs/{}", uuid::Uuid::new_v4()))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
