use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::requests,
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
            "/api/requests/leave",
            axum::routing::post(requests::create_leave_request),
        )
        .route(
            "/api/requests/me",
            axum::routing::get(requests::get_my_requests),
        )
        .route(
            "/api/requests/{id}",
            axum::routing::delete(requests::cancel_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

#[tokio::test]
async fn test_create_leave_request_succeeds() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload = json!({
        "leave_type": "annual",
        "start_date": "2024-07-15",
        "end_date": "2024-07-17",
        "reason": "Summer vacation"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_leave_request_with_invalid_date_range_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload = json!({
        "leave_type": "annual",
        "start_date": "2024-07-20",
        "end_date": "2024-07-15",
        "reason": "Invalid dates"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_leave_request_without_reason_succeeds() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload = json!({
        "leave_type": "sick",
        "start_date": "2024-07-15",
        "end_date": "2024-07-15"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_my_requests_returns_list() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload = json!({
        "leave_type": "annual",
        "start_date": "2024-07-15",
        "end_date": "2024-07-17",
        "reason": "Summer vacation"
    });
    let request_create = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    app.clone().oneshot(request_create).await.unwrap();

    let request_list = Request::builder()
        .uri("/api/requests/me")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request_list).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["leave_requests"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_cancel_leave_request_succeeds() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload = json!({
        "leave_type": "annual",
        "start_date": "2024-08-01",
        "end_date": "2024-08-03",
        "reason": "Personal"
    });
    let request_create = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response_create = app.clone().oneshot(request_create).await.unwrap();
    assert_eq!(response_create.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response_create.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let request_id = json["id"].as_str().unwrap();

    let request_cancel = Request::builder()
        .method("DELETE")
        .uri(format!("/api/requests/{}", request_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response_cancel = app.oneshot(request_cancel).await.unwrap();
    assert_eq!(response_cancel.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cancel_nonexistent_request_returns_not_found() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let request_id = timekeeper_backend::types::LeaveRequestId::new();
    let request_cancel = Request::builder()
        .method("DELETE")
        .uri(format!("/api/requests/{}", request_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request_cancel).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
