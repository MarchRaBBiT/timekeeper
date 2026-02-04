use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::attendance,
    models::user::{User, UserRole},
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

fn test_router_with_state(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route("/api/admin/attendance", axum::routing::get(attendance::get_all_attendance))
        .layer(Extension(user))
        .with_state(state)
}

fn test_router_with_upsert(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route("/api/admin/attendance", axum::routing::put(attendance::upsert_attendance))
        .layer(Extension(user))
        .with_state(state)
}

#[tokio::test]
async fn test_system_admin_can_list_all_attendance() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let _employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_state(pool.clone(), sysadmin.clone());
    
    let request = Request::builder()
        .uri("/api/admin/attendance")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_regular_admin_cannot_list_all_attendance() {
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
        .uri("/api/admin/attendance")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_employee_cannot_list_all_attendance() {
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
        .uri("/api/admin/attendance")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_system_admin_can_upsert_attendance() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_upsert(pool.clone(), sysadmin.clone());
    
    let payload = json!({
        "user_id": employee.id.to_string(),
        "date": "2024-07-15",
        "clock_in_time": "2024-07-15T09:00:00",
        "clock_out_time": "2024-07-15T18:00:00",
        "breaks": [
            {
                "break_start_time": "2024-07-15T12:00:00",
                "break_end_time": "2024-07-15T13:00:00"
            }
        ]
    });
    
    let request = Request::builder()
        .method("PUT")
        .uri("/api/admin/attendance")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_regular_admin_cannot_upsert_attendance() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_upsert(pool.clone(), admin.clone());
    
    let payload = json!({
        "user_id": employee.id.to_string(),
        "date": "2024-07-15",
        "clock_in_time": "2024-07-15T09:00:00",
        "clock_out_time": "2024-07-15T18:00:00"
    });
    
    let request = Request::builder()
        .method("PUT")
        .uri("/api/admin/attendance")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_upsert_attendance_with_invalid_date_format_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_upsert(pool.clone(), sysadmin.clone());
    
    let payload = json!({
        "user_id": employee.id.to_string(),
        "date": "invalid-date",
        "clock_in_time": "2024-07-15T09:00:00"
    });
    
    let request = Request::builder()
        .method("PUT")
        .uri("/api/admin/attendance")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
