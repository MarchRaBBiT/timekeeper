use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension, Router,
};
use chrono::{NaiveDate, NaiveDateTime};
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
    create_test_token, seed_attendance, seed_break_record, seed_user, test_config, test_pool,
};

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
            "/api/admin/attendance",
            axum::routing::get(attendance::get_all_attendance),
        )
        .layer(Extension(user))
        .with_state(state)
}

fn test_router_with_upsert(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/attendance",
            axum::routing::put(attendance::upsert_attendance),
        )
        .route(
            "/api/admin/breaks/{id}/force-end",
            axum::routing::put(attendance::force_end_break),
        )
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

#[tokio::test]
async fn test_upsert_attendance_with_invalid_user_id_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_upsert(pool.clone(), sysadmin.clone());

    let payload = json!({
        "user_id": "not-a-uuid",
        "date": "2024-07-15",
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

#[tokio::test]
async fn test_upsert_attendance_with_invalid_clock_times_fail() {
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

    let invalid_clock_in = json!({
        "user_id": employee.id.to_string(),
        "date": "2024-07-15",
        "clock_in_time": "invalid"
    });
    let req_clock_in = Request::builder()
        .method("PUT")
        .uri("/api/admin/attendance")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_clock_in.to_string()))
        .unwrap();
    let res_clock_in = app.clone().oneshot(req_clock_in).await.unwrap();
    assert_eq!(res_clock_in.status(), StatusCode::BAD_REQUEST);

    let invalid_clock_out = json!({
        "user_id": employee.id.to_string(),
        "date": "2024-07-15",
        "clock_in_time": "2024-07-15T09:00:00",
        "clock_out_time": "invalid"
    });
    let req_clock_out = Request::builder()
        .method("PUT")
        .uri("/api/admin/attendance")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_clock_out.to_string()))
        .unwrap();
    let res_clock_out = app.clone().oneshot(req_clock_out).await.unwrap();
    assert_eq!(res_clock_out.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_upsert_attendance_with_invalid_break_start_fails() {
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
        "breaks": [
            { "break_start_time": "invalid", "break_end_time": "2024-07-15T13:00:00" }
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
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_system_admin_can_force_end_active_break() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2024, 7, 15).expect("valid date");
    let clock_in = NaiveDateTime::parse_from_str("2024-07-15T09:00:00", "%Y-%m-%dT%H:%M:%S")
        .expect("clock in");
    let clock_out = NaiveDateTime::parse_from_str("2024-07-15T18:00:00", "%Y-%m-%dT%H:%M:%S")
        .expect("clock out");
    let attendance =
        seed_attendance(&pool, employee.id, date, Some(clock_in), Some(clock_out)).await;
    let break_start = NaiveDateTime::parse_from_str("2024-07-15T12:00:00", "%Y-%m-%dT%H:%M:%S")
        .expect("break start");
    let break_record = seed_break_record(&pool, attendance.id, break_start, None).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_upsert(pool.clone(), sysadmin.clone());

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/admin/breaks/{}/force-end", break_record.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let ended: (Option<NaiveDateTime>, Option<i32>) =
        sqlx::query_as("SELECT break_end_time, duration_minutes FROM break_records WHERE id = $1")
            .bind(break_record.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("fetch break");
    assert!(ended.0.is_some());
    assert!(ended.1.unwrap_or_default() >= 0);
}

#[tokio::test]
async fn test_force_end_break_rejects_invalid_or_already_ended_break() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let sysadmin = seed_user(&pool, UserRole::Admin, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let date = NaiveDate::from_ymd_opt(2024, 7, 15).expect("valid date");
    let clock_in = NaiveDateTime::parse_from_str("2024-07-15T09:00:00", "%Y-%m-%dT%H:%M:%S")
        .expect("clock in");
    let clock_out = NaiveDateTime::parse_from_str("2024-07-15T18:00:00", "%Y-%m-%dT%H:%M:%S")
        .expect("clock out");
    let attendance =
        seed_attendance(&pool, employee.id, date, Some(clock_in), Some(clock_out)).await;
    let break_start = NaiveDateTime::parse_from_str("2024-07-15T12:00:00", "%Y-%m-%dT%H:%M:%S")
        .expect("break start");
    let break_end = NaiveDateTime::parse_from_str("2024-07-15T13:00:00", "%Y-%m-%dT%H:%M:%S")
        .expect("break end");
    let ended_break = seed_break_record(&pool, attendance.id, break_start, Some(break_end)).await;

    let token = create_test_token(sysadmin.id, sysadmin.role.clone());
    let app = test_router_with_upsert(pool.clone(), sysadmin.clone());

    let invalid_id_request = Request::builder()
        .method("PUT")
        .uri("/api/admin/breaks/not-a-uuid/force-end")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let invalid_id_response = app.clone().oneshot(invalid_id_request).await.unwrap();
    assert_eq!(invalid_id_response.status(), StatusCode::BAD_REQUEST);

    let ended_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/admin/breaks/{}/force-end", ended_break.id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let ended_response = app.clone().oneshot(ended_request).await.unwrap();
    assert_eq!(ended_response.status(), StatusCode::BAD_REQUEST);

    let regular_admin = seed_user(&pool, UserRole::Admin, false).await;
    let regular_token = create_test_token(regular_admin.id, regular_admin.role.clone());
    let app_regular = test_router_with_upsert(pool.clone(), regular_admin.clone());
    let forbidden_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/admin/breaks/{}/force-end", ended_break.id))
        .header("Authorization", format!("Bearer {}", regular_token))
        .body(Body::empty())
        .unwrap();
    let forbidden_response = app_regular.oneshot(forbidden_request).await.unwrap();
    assert_eq!(forbidden_response.status(), StatusCode::FORBIDDEN);
}
