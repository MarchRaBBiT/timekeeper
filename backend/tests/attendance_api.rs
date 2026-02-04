use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use timekeeper_backend::{
    handlers::attendance,
    models::{
        attendance::{ClockInRequest, ClockOutRequest},
        user::{User, UserRole},
    },
    services::holiday::HolidayService,
    state::AppState,
    types::AttendanceId,
};
use tower::ServiceExt;

mod support;

use support::{create_test_token, seed_public_holiday, seed_user, test_config, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn test_router_with_state(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool.clone(), None, None, None, test_config());
    let holiday_service: Arc<dyn timekeeper_backend::services::holiday::HolidayServiceTrait> =
        Arc::new(HolidayService::new(pool));

    Router::new()
        .route(
            "/api/attendance/clock-in",
            axum::routing::post(attendance::clock_in),
        )
        .route(
            "/api/attendance/clock-out",
            axum::routing::post(attendance::clock_out),
        )
        .route(
            "/api/attendance/break-start",
            axum::routing::post(attendance::break_start),
        )
        .route(
            "/api/attendance/break-end",
            axum::routing::post(attendance::break_end),
        )
        .route(
            "/api/attendance/status",
            axum::routing::get(attendance::get_attendance_status),
        )
        .route(
            "/api/attendance/me",
            axum::routing::get(attendance::get_my_attendance),
        )
        .layer(Extension(user))
        .layer(Extension(holiday_service))
        .with_state(state)
}

#[tokio::test]
async fn test_clock_in_creates_attendance_record() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload = ClockInRequest { date: None };
    let request = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_clock_in_with_specific_date() {
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
        "date": "2024-07-15"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_clock_in_twice_returns_error() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload = ClockInRequest { date: None };
    let request = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload).to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let request2 = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload).to_string()))
        .unwrap();
    let response2 = app.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_clock_out_without_clock_in_returns_error() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload = ClockOutRequest { date: None };
    let request = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-out")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_clock_out_after_clock_in_succeeds() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload_in = ClockInRequest { date: None };
    let request_in = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload_in).to_string()))
        .unwrap();
    let response_in = app.clone().oneshot(request_in).await.unwrap();
    assert_eq!(response_in.status(), StatusCode::OK);

    let payload_out = ClockOutRequest { date: None };
    let request_out = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-out")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload_out).to_string()))
        .unwrap();
    let response_out = app.oneshot(request_out).await.unwrap();
    assert_eq!(response_out.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_start_break_without_clock_in_returns_error() {
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
        "attendance_id": AttendanceId::new().to_string()
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/attendance/break-start")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_break_flow_works_correctly() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload_in = ClockInRequest { date: None };
    let request_in = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload_in).to_string()))
        .unwrap();
    let response_in = app.clone().oneshot(request_in).await.unwrap();
    assert_eq!(response_in.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response_in.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let attendance_id = json["id"].as_str().unwrap();

    let payload_start = json!({
        "attendance_id": attendance_id
    });
    let request_start = Request::builder()
        .method("POST")
        .uri("/api/attendance/break-start")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload_start.to_string()))
        .unwrap();
    let response_start = app.clone().oneshot(request_start).await.unwrap();
    assert_eq!(response_start.status(), StatusCode::OK);

    let body_start = axum::body::to_bytes(response_start.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_start: serde_json::Value = serde_json::from_slice(&body_start).unwrap();
    let break_id = json_start["id"].as_str().unwrap();

    let payload_end = json!({
        "break_record_id": break_id
    });
    let request_end = Request::builder()
        .method("POST")
        .uri("/api/attendance/break-end")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload_end.to_string()))
        .unwrap();
    let response_end = app.oneshot(request_end).await.unwrap();
    assert_eq!(response_end.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_attendance_status_returns_correct_status() {
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
        .uri("/api/attendance/status")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "not_started");

    let payload_in = ClockInRequest { date: None };
    let request_in = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload_in).to_string()))
        .unwrap();
    app.clone().oneshot(request_in).await.unwrap();

    let request2 = Request::builder()
        .uri("/api/attendance/status")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response2 = app.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);

    let body2 = axum::body::to_bytes(response2.into_body(), usize::MAX)
        .await
        .unwrap();
    let json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    assert_eq!(json2["status"], "clocked_in");
}

#[tokio::test]
async fn test_get_my_attendance_returns_list() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload_in = ClockInRequest { date: None };
    let request_in = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload_in).to_string()))
        .unwrap();
    app.clone().oneshot(request_in).await.unwrap();

    let request = Request::builder()
        .uri("/api/attendance/me")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_clock_out_during_break_returns_error() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool.clone(), employee.clone());

    let payload_in = ClockInRequest { date: None };
    let request_in = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload_in).to_string()))
        .unwrap();
    let response_in = app.clone().oneshot(request_in).await.unwrap();
    assert_eq!(response_in.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response_in.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let attendance_id = json["id"].as_str().unwrap();

    let payload_start = json!({
        "attendance_id": attendance_id
    });
    let request_start = Request::builder()
        .method("POST")
        .uri("/api/attendance/break-start")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(payload_start.to_string()))
        .unwrap();
    app.clone().oneshot(request_start).await.unwrap();

    let payload_out = ClockOutRequest { date: None };
    let request_out = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-out")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!(payload_out).to_string()))
        .unwrap();
    let response_out = app.oneshot(request_out).await.unwrap();
    assert_eq!(response_out.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_my_attendance_rejects_invalid_ranges_and_month() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool, employee);

    let invalid_range = Request::builder()
        .uri("/api/attendance/me?from=2026-02-10&to=2026-02-01")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build invalid range request");
    let invalid_range_response = app
        .clone()
        .oneshot(invalid_range)
        .await
        .expect("call invalid range");
    assert_eq!(invalid_range_response.status(), StatusCode::BAD_REQUEST);

    let invalid_month = Request::builder()
        .uri("/api/attendance/me?year=2026&month=13")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build invalid month request");
    let invalid_month_response = app
        .oneshot(invalid_month)
        .await
        .expect("call invalid month");
    assert_eq!(invalid_month_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_clock_in_rejects_public_holiday_date() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_public_holiday(
        &pool,
        chrono::NaiveDate::from_ymd_opt(2026, 2, 11).expect("valid date"),
        "National Foundation Day",
    )
    .await;

    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool, employee);

    let request = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"date":"2026-02-11"}).to_string()))
        .expect("build holiday clock-in request");

    let response = app.oneshot(request).await.expect("call holiday clock-in");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_break_start_twice_and_break_end_twice_return_bad_request() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let token = create_test_token(employee.id, employee.role.clone());
    let app = test_router_with_state(pool, employee);

    let clock_in = Request::builder()
        .method("POST")
        .uri("/api/attendance/clock-in")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"date":"2026-03-01"}).to_string()))
        .expect("build clock in");
    let clock_in_response = app.clone().oneshot(clock_in).await.expect("call clock in");
    assert_eq!(clock_in_response.status(), StatusCode::OK);
    let clock_in_body = axum::body::to_bytes(clock_in_response.into_body(), usize::MAX)
        .await
        .expect("read clock in body");
    let clock_in_json: serde_json::Value =
        serde_json::from_slice(&clock_in_body).expect("parse clock in json");
    let attendance_id = clock_in_json["id"].as_str().expect("attendance id");

    let break_start = Request::builder()
        .method("POST")
        .uri("/api/attendance/break-start")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"attendance_id": attendance_id}).to_string(),
        ))
        .expect("build break start");
    let break_start_response = app
        .clone()
        .oneshot(break_start)
        .await
        .expect("call break start");
    assert_eq!(break_start_response.status(), StatusCode::OK);
    let break_body = axum::body::to_bytes(break_start_response.into_body(), usize::MAX)
        .await
        .expect("read break body");
    let break_json: serde_json::Value =
        serde_json::from_slice(&break_body).expect("parse break json");
    let break_id = break_json["id"].as_str().expect("break id");

    let duplicate_break_start = Request::builder()
        .method("POST")
        .uri("/api/attendance/break-start")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"attendance_id": attendance_id}).to_string(),
        ))
        .expect("build duplicate break start");
    let duplicate_break_start_response = app
        .clone()
        .oneshot(duplicate_break_start)
        .await
        .expect("call duplicate break start");
    assert_eq!(
        duplicate_break_start_response.status(),
        StatusCode::BAD_REQUEST
    );

    let break_end = Request::builder()
        .method("POST")
        .uri("/api/attendance/break-end")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"break_record_id": break_id}).to_string()))
        .expect("build break end");
    let break_end_response = app
        .clone()
        .oneshot(break_end)
        .await
        .expect("call break end");
    assert_eq!(break_end_response.status(), StatusCode::OK);

    let duplicate_break_end = Request::builder()
        .method("POST")
        .uri("/api/attendance/break-end")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"break_record_id": break_id}).to_string()))
        .expect("build duplicate break end");
    let duplicate_break_end_response = app
        .oneshot(duplicate_break_end)
        .await
        .expect("call duplicate break end");
    assert_eq!(
        duplicate_break_end_response.status(),
        StatusCode::BAD_REQUEST
    );
}
