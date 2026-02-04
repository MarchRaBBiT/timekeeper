use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::subject_requests,
    models::{
        subject_request::DataSubjectRequestType,
        user::{User, UserRole},
    },
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
            "/api/admin/subject-requests",
            axum::routing::get(subject_requests::list_subject_requests),
        )
        .route(
            "/api/admin/subject-requests/{id}/approve",
            axum::routing::post(subject_requests::approve_subject_request),
        )
        .route(
            "/api/admin/subject-requests/{id}/reject",
            axum::routing::post(subject_requests::reject_subject_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn seed_subject_request(
    pool: &PgPool,
    user_id: timekeeper_backend::types::UserId,
    request_type: DataSubjectRequestType,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO subject_requests (id, user_id, request_type, status, details, created_at, updated_at)
        VALUES ($1, $2, $3, 'pending', 'Test request', NOW(), NOW())
        "#
    )
    .bind(&id)
    .bind(user_id.to_string())
    .bind(request_type.db_value())
    .execute(pool)
    .await
    .expect("insert subject request");
    id
}

#[tokio::test]
async fn test_admin_can_list_subject_requests() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_subject_request(&pool, employee.id, DataSubjectRequestType::Access).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let request = Request::builder()
        .uri("/api/admin/subject-requests")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["items"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_employee_cannot_list_subject_requests() {
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
        .uri("/api/admin/subject-requests")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_can_approve_subject_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let request_id = seed_subject_request(&pool, employee.id, DataSubjectRequestType::Access).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let payload = json!({"comment": "Approved for data access"});
    let request = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/admin/subject-requests/{}/approve",
            request_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_can_reject_subject_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let request_id = seed_subject_request(&pool, employee.id, DataSubjectRequestType::Delete).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let payload = json!({"comment": "Cannot delete due to legal retention"});
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/admin/subject-requests/{}/reject", request_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
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
    let request_id = seed_subject_request(&pool, employee.id, DataSubjectRequestType::Access).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let payload = json!({"comment": "Approved"});
    let request_approve = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/admin/subject-requests/{}/approve",
            request_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    app.clone().oneshot(request_approve).await.unwrap();

    let request_approve_again = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/admin/subject-requests/{}/approve",
            request_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.oneshot(request_approve_again).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_subject_requests_with_status_filter() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_subject_request(&pool, employee.id, DataSubjectRequestType::Access).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let request = Request::builder()
        .uri("/api/admin/subject-requests?status=pending")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["items"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_list_subject_requests_with_type_filter() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_subject_request(&pool, employee.id, DataSubjectRequestType::Access).await;

    let token = create_test_token(admin.id, admin.role.clone());
    let app = test_router_with_state(pool.clone(), admin.clone());

    let request = Request::builder()
        .uri("/api/admin/subject-requests?type=access")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_subject_requests_with_invalid_status_fails() {
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
        .uri("/api/admin/subject-requests?status=invalid")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_employee_cannot_approve_subject_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee1 = seed_user(&pool, UserRole::Employee, false).await;
    let employee2 = seed_user(&pool, UserRole::Employee, false).await;
    let request_id =
        seed_subject_request(&pool, employee2.id, DataSubjectRequestType::Access).await;

    let token = create_test_token(employee1.id, employee1.role.clone());
    let app = test_router_with_state(pool.clone(), employee1.clone());

    let payload = json!({"comment": "Trying to approve"});
    let request = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/admin/subject-requests/{}/approve",
            request_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
