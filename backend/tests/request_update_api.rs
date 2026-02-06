use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use chrono::{NaiveDate, Utc};
use serde_json::json;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::requests,
    models::{
        leave_request::RequestStatus,
        user::{User, UserRole},
    },
    repositories::leave_request::{LeaveRequestRepository, LeaveRequestRepositoryTrait},
    state::AppState,
};
use tower::ServiceExt;

mod support;

use support::{
    create_test_token, seed_leave_request, seed_overtime_request, seed_user, test_config, test_pool,
};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn requests_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/requests/{id}",
            axum::routing::put(requests::update_request).delete(requests::cancel_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn migrate(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("run migrations");
}

#[tokio::test]
async fn update_request_rejects_invalid_id_format() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    migrate(&pool).await;

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let token = create_test_token(employee.id, employee.role.clone());
    let app = requests_router(pool, employee);

    let request = Request::builder()
        .method("PUT")
        .uri("/api/requests/not-a-uuid")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({ "reason": "x" }).to_string()))
        .expect("build request");

    let response = app.oneshot(request).await.expect("call update");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_leave_request_validates_pending_and_date_window() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    migrate(&pool).await;

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let leave = seed_leave_request(
        &pool,
        employee.id,
        timekeeper_backend::models::leave_request::LeaveType::Annual,
        NaiveDate::from_ymd_opt(2026, 2, 10).expect("valid date"),
        NaiveDate::from_ymd_opt(2026, 2, 11).expect("valid date"),
    )
    .await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = requests_router(pool.clone(), employee);

    let invalid_window = Request::builder()
        .method("PUT")
        .uri(format!("/api/requests/{}", leave.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "start_date": "2026-02-12",
                "end_date": "2026-02-10"
            })
            .to_string(),
        ))
        .expect("build invalid window request");
    let invalid_window_response = app
        .clone()
        .oneshot(invalid_window)
        .await
        .expect("call invalid window");
    assert_eq!(invalid_window_response.status(), StatusCode::BAD_REQUEST);

    let repo = LeaveRequestRepository::new();
    let affected = repo
        .approve(&pool, leave.id, admin.id, "approved", Utc::now())
        .await
        .expect("approve leave");
    assert_eq!(affected, 1);

    let non_pending = Request::builder()
        .method("PUT")
        .uri(format!("/api/requests/{}", leave.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({ "reason": "cannot update" }).to_string()))
        .expect("build non-pending request");
    let non_pending_response = app
        .oneshot(non_pending)
        .await
        .expect("call non-pending update");
    assert_eq!(non_pending_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_leave_request_succeeds_for_pending_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    migrate(&pool).await;

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let leave = seed_leave_request(
        &pool,
        employee.id,
        timekeeper_backend::models::leave_request::LeaveType::Personal,
        NaiveDate::from_ymd_opt(2026, 2, 20).expect("valid date"),
        NaiveDate::from_ymd_opt(2026, 2, 20).expect("valid date"),
    )
    .await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = requests_router(pool.clone(), employee);

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/requests/{}", leave.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "leave_type": "sick",
                "start_date": "2026-02-21",
                "end_date": "2026-02-21",
                "reason": "updated reason"
            })
            .to_string(),
        ))
        .expect("build update request");
    let response = app.oneshot(request).await.expect("call update");
    assert_eq!(response.status(), StatusCode::OK);

    let updated: (String, String, Option<String>) =
        sqlx::query_as("SELECT leave_type, status, reason FROM leave_requests WHERE id = $1")
            .bind(leave.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("fetch updated leave");
    assert_eq!(updated.0, "sick");
    assert_eq!(updated.1, RequestStatus::Pending.db_value());
    assert_eq!(updated.2.as_deref(), Some("updated reason"));
}

#[tokio::test]
async fn update_overtime_request_validates_payload_and_updates() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    migrate(&pool).await;

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let overtime = seed_overtime_request(
        &pool,
        employee.id,
        NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid date"),
        2.0,
    )
    .await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = requests_router(pool.clone(), employee);

    let invalid_hours = Request::builder()
        .method("PUT")
        .uri(format!("/api/requests/{}", overtime.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({ "planned_hours": 0.0 }).to_string()))
        .expect("build invalid hours request");
    let invalid_hours_response = app
        .clone()
        .oneshot(invalid_hours)
        .await
        .expect("call invalid hours");
    assert_eq!(invalid_hours_response.status(), StatusCode::BAD_REQUEST);

    let invalid_payload = Request::builder()
        .method("PUT")
        .uri(format!("/api/requests/{}", overtime.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({ "date": 123 }).to_string()))
        .expect("build invalid payload request");
    let invalid_payload_response = app
        .clone()
        .oneshot(invalid_payload)
        .await
        .expect("call invalid payload");
    assert_eq!(invalid_payload_response.status(), StatusCode::BAD_REQUEST);

    let update = Request::builder()
        .method("PUT")
        .uri(format!("/api/requests/{}", overtime.id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "date": "2026-03-02",
                "planned_hours": 3.5,
                "reason": "updated overtime"
            })
            .to_string(),
        ))
        .expect("build update request");
    let update_response = app.oneshot(update).await.expect("call overtime update");
    assert_eq!(update_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn cancel_request_handles_invalid_and_not_cancellable_states() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    migrate(&pool).await;

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let admin = seed_user(&pool, UserRole::Admin, false).await;
    let leave = seed_leave_request(
        &pool,
        employee.id,
        timekeeper_backend::models::leave_request::LeaveType::Annual,
        NaiveDate::from_ymd_opt(2026, 4, 10).expect("valid date"),
        NaiveDate::from_ymd_opt(2026, 4, 10).expect("valid date"),
    )
    .await;

    let repo = LeaveRequestRepository::new();
    let affected = repo
        .approve(&pool, leave.id, admin.id, "approved", Utc::now())
        .await
        .expect("approve leave");
    assert_eq!(affected, 1);

    let token = create_test_token(employee.id, employee.role.clone());
    let app = requests_router(pool, employee);

    let invalid_id = Request::builder()
        .method("DELETE")
        .uri("/api/requests/not-a-uuid")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build invalid id cancel request");
    let invalid_id_response = app
        .clone()
        .oneshot(invalid_id)
        .await
        .expect("call invalid id cancel");
    assert_eq!(invalid_id_response.status(), StatusCode::BAD_REQUEST);

    let not_cancellable = Request::builder()
        .method("DELETE")
        .uri(format!("/api/requests/{}", leave.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build non-cancellable request");
    let not_cancellable_response = app
        .oneshot(not_cancellable)
        .await
        .expect("call non-cancellable request");
    assert_eq!(not_cancellable_response.status(), StatusCode::NOT_FOUND);
}
