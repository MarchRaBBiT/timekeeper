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
            "/api/requests/overtime",
            axum::routing::post(requests::create_overtime_request),
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
async fn test_create_overtime_request_succeeds() {
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
        "date": "2024-07-15",
        "planned_hours": 3.5,
        "reason": "Project deadline"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/requests/overtime")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_overtime_request_with_zero_hours_fails() {
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
        "date": "2024-07-15",
        "planned_hours": 0.0,
        "reason": "Invalid"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/requests/overtime")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_overtime_request_with_excessive_hours_fails() {
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
        "date": "2024-07-15",
        "planned_hours": 25.0,
        "reason": "Too many hours"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/requests/overtime")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_my_requests_includes_overtime() {
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
        "date": "2024-08-01",
        "planned_hours": 2.0,
        "reason": "Urgent task"
    });
    let request_create = Request::builder()
        .method("POST")
        .uri("/api/requests/overtime")
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
    assert!(json["overtime_requests"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_cancel_overtime_request_succeeds() {
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
        "date": "2024-08-15",
        "planned_hours": 4.0,
        "reason": "Extra work"
    });
    let request_create = Request::builder()
        .method("POST")
        .uri("/api/requests/overtime")
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
async fn test_create_overtime_request_without_reason_succeeds() {
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
        "date": "2024-09-01",
        "planned_hours": 1.5
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/requests/overtime")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
