use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::requests as admin_requests,
    handlers::requests as user_requests,
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

fn test_router_admin(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/requests",
            axum::routing::get(admin_requests::list_requests),
        )
        .route(
            "/api/admin/requests/{id}/approve",
            axum::routing::post(admin_requests::approve_request),
        )
        .route(
            "/api/admin/requests/{id}/reject",
            axum::routing::post(admin_requests::reject_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

fn test_router_user(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/requests/leave",
            axum::routing::post(user_requests::create_leave_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

#[tokio::test]
async fn test_admin_can_list_all_requests() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let employee_token = create_test_token(employee.id, employee.role.clone());
    let user_app = test_router_user(pool.clone(), employee.clone());

    let payload = json!({
        "leave_type": "annual",
        "start_date": "2024-07-15",
        "end_date": "2024-07-17",
        "reason": "Summer vacation"
    });
    let request_create = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", employee_token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    user_app.oneshot(request_create).await.unwrap();

    let admin_token = create_test_token(admin.id, admin.role.clone());
    let admin_app = test_router_admin(pool.clone(), admin.clone());

    let request = Request::builder()
        .uri("/api/admin/requests")
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let response = admin_app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_employee_cannot_list_all_requests() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_admin(pool.clone(), employee.clone());

    let request = Request::builder()
        .uri("/api/admin/requests")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_can_approve_leave_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let employee_token = create_test_token(employee.id, employee.role.clone());
    let user_app = test_router_user(pool.clone(), employee.clone());

    let payload = json!({
        "leave_type": "annual",
        "start_date": "2024-08-01",
        "end_date": "2024-08-03",
        "reason": "Personal time"
    });
    let request_create = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", employee_token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response_create = user_app.oneshot(request_create).await.unwrap();

    let body = axum::body::to_bytes(response_create.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let request_id = json["id"].as_str().unwrap();

    let admin_token = create_test_token(admin.id, admin.role.clone());
    let admin_app = test_router_admin(pool.clone(), admin.clone());

    let approve_payload = json!({
        "comment": "Approved for summer break"
    });
    let request_approve = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/requests/{}/approve", request_id))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(approve_payload.to_string()))
        .unwrap();

    let response = admin_app.oneshot(request_approve).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_can_reject_leave_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let employee_token = create_test_token(employee.id, employee.role.clone());
    let user_app = test_router_user(pool.clone(), employee.clone());

    let payload = json!({
        "leave_type": "sick",
        "start_date": "2024-09-01",
        "end_date": "2024-09-02",
        "reason": "Not feeling well"
    });
    let request_create = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", employee_token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response_create = user_app.oneshot(request_create).await.unwrap();

    let body = axum::body::to_bytes(response_create.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let request_id = json["id"].as_str().unwrap();

    let admin_token = create_test_token(admin.id, admin.role.clone());
    let admin_app = test_router_admin(pool.clone(), admin.clone());

    let reject_payload = json!({
        "comment": "Insufficient staffing during this period"
    });
    let request_reject = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/requests/{}/reject", request_id))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(reject_payload.to_string()))
        .unwrap();

    let response = admin_app.oneshot(request_reject).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_approve_already_processed_request_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let employee_token = create_test_token(employee.id, employee.role.clone());
    let user_app = test_router_user(pool.clone(), employee.clone());

    let payload = json!({
        "leave_type": "personal",
        "start_date": "2024-10-01",
        "end_date": "2024-10-01",
        "reason": "Personal matter"
    });
    let request_create = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", employee_token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response_create = user_app.oneshot(request_create).await.unwrap();

    let body = axum::body::to_bytes(response_create.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let request_id = json["id"].as_str().unwrap();

    let admin_token = create_test_token(admin.id, admin.role.clone());
    let admin_app = test_router_admin(pool.clone(), admin.clone());

    let approve_payload = json!({
        "comment": "Approved"
    });
    let request_approve = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/requests/{}/approve", request_id))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(approve_payload.to_string()))
        .unwrap();
    admin_app.clone().oneshot(request_approve).await.unwrap();

    let request_approve_again = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/requests/{}/approve", request_id))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(approve_payload.to_string()))
        .unwrap();
    let response = admin_app.oneshot(request_approve_again).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_employee_cannot_approve_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee1 = seed_user(&pool, UserRole::Employee, false).await;
    let employee2 = seed_user(&pool, UserRole::Employee, false).await;

    let employee2_token = create_test_token(employee2.id, employee2.role.clone());
    let user_app = test_router_user(pool.clone(), employee2.clone());

    let payload = json!({
        "leave_type": "annual",
        "start_date": "2024-11-01",
        "end_date": "2024-11-03",
        "reason": "Vacation"
    });
    let request_create = Request::builder()
        .method("POST")
        .uri("/api/requests/leave")
        .header("Authorization", format!("Bearer {}", employee2_token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response_create = user_app.oneshot(request_create).await.unwrap();

    let body = axum::body::to_bytes(response_create.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let request_id = json["id"].as_str().unwrap();

    let employee1_token = create_test_token(employee1.id, employee1.role.clone());
    let admin_app = test_router_admin(pool.clone(), employee1.clone());

    let approve_payload = json!({
        "comment": "Trying to approve"
    });
    let request_approve = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/requests/{}/approve", request_id))
        .header("Authorization", format!("Bearer {}", employee1_token))
        .header("Content-Type", "application/json")
        .body(Body::from(approve_payload.to_string()))
        .unwrap();

    let response = admin_app.oneshot(request_approve).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
