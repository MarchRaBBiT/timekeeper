use axum::{
    body::{to_bytes, Body},
    http::{header::HeaderName, Request, StatusCode},
    routing::get,
    Extension, Router,
};
use chrono::{Datelike, Duration, Months, NaiveDate, Utc};
use serde_json::Value;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::export_data,
    models::{attendance::Attendance, leave_request::LeaveType, user::UserRole},
    repositories::{attendance::AttendanceRepository, AttendanceRepositoryTrait},
    state::AppState,
};
use tower::ServiceExt;

/// "Today" as the admin export handler resolves it (Asia/Tokyo, matching
/// `support::test_config().time_zone`), so default-period tests stay valid
/// regardless of when the suite runs.
fn today_in_test_timezone() -> NaiveDate {
    Utc::now()
        .with_timezone(&chrono_tz::Asia::Tokyo)
        .date_naive()
}

fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("valid month start")
}

fn month_end(date: NaiveDate) -> NaiveDate {
    month_start(date)
        .checked_add_months(Months::new(1))
        .and_then(|next| next.checked_sub_signed(Duration::days(1)))
        .expect("valid month end")
}

#[path = "support/mod.rs"]
mod support;
use support::integration_guard;

async fn reset_attendance_tables(pool: &PgPool) {
    sqlx::query(
        "TRUNCATE attendance_correction_effective_values, \
         attendance_correction_requests, \
         break_records, \
         leave_requests, \
         attendance RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await
    .expect("truncate attendance tables");
}

async fn setup_manager_scope(pool: &PgPool, manager_id: &str, employee_id: &str) {
    let dept_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&dept_id)
        .bind("Export Test Dept")
        .execute(pool)
        .await
        .expect("insert department");
    sqlx::query(
        "INSERT INTO department_managers (department_id, user_id) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(&dept_id)
    .bind(manager_id)
    .execute(pool)
    .await
    .expect("assign manager");
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(&dept_id)
        .bind(employee_id)
        .execute(pool)
        .await
        .expect("set employee department");
}

#[tokio::test]
async fn admin_export_includes_date_strings() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_attendance_tables(&pool).await;

    let admin = support::seed_user(&pool, UserRole::Manager, false).await;
    let employee = support::seed_user(&pool, UserRole::Employee, false).await;
    setup_manager_scope(&pool, &admin.id.to_string(), &employee.id.to_string()).await;

    let date = NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date");
    let now = Utc::now();
    let attendance = Attendance::new(employee.id, date, now);
    let repo = AttendanceRepository::new();
    repo.create(&pool, &attendance)
        .await
        .expect("create attendance");

    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let app = Router::new()
        .route("/api/admin/export", get(export_data))
        .layer(Extension(admin))
        .with_state(state);

    // M-3: default period is the current month when `from`/`to` are
    // omitted, so this test now pins an explicit range to keep asserting on
    // a fixed, historical date regardless of when the suite runs.
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/export?from=2026-01-15&to=2026-01-15")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert!(response.status().is_success());
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: Value = serde_json::from_slice(&body).expect("parse response");
    let csv_data = payload
        .get("csv_data")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    assert!(csv_data.contains("\"2026-01-15\""));
}

#[tokio::test]
async fn admin_export_includes_approved_leave_rows_and_leave_type_column() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_attendance_tables(&pool).await;

    let admin = support::seed_user(&pool, UserRole::Manager, false).await;
    let employee = support::seed_user(&pool, UserRole::Employee, false).await;
    setup_manager_scope(&pool, &admin.id.to_string(), &employee.id.to_string()).await;

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).expect("valid date");
    let leave =
        support::seed_leave_request(&pool, employee.id, LeaveType::Annual, date, date).await;
    sqlx::query(
        "UPDATE leave_requests
         SET status = 'approved', approved_by = $1, approved_at = NOW(), updated_at = NOW()
         WHERE id = $2",
    )
    .bind(admin.id.to_string())
    .bind(leave.id.to_string())
    .execute(&pool)
    .await
    .expect("approve leave");

    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let app = Router::new()
        .route("/api/admin/export", get(export_data))
        .layer(Extension(admin))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/export?from=2026-01-20&to=2026-01-20")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert!(response.status().is_success());
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: Value = serde_json::from_slice(&body).expect("parse response");
    let csv_data = payload
        .get("csv_data")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    assert!(csv_data.contains("\"Leave Type\""));
    assert!(csv_data.contains("\"2026-01-20\""));
    assert!(csv_data.contains("\"on_leave\",\"annual\""));
}

#[tokio::test]
async fn admin_export_masks_full_name_for_non_system_admin() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_attendance_tables(&pool).await;

    let admin = support::seed_user(&pool, UserRole::Manager, false).await;
    let employee = support::seed_user(&pool, UserRole::Employee, false).await;
    setup_manager_scope(&pool, &admin.id.to_string(), &employee.id.to_string()).await;

    let date = NaiveDate::from_ymd_opt(2026, 1, 16).expect("valid date");
    let now = Utc::now();
    let attendance = Attendance::new(employee.id, date, now);
    let repo = AttendanceRepository::new();
    repo.create(&pool, &attendance)
        .await
        .expect("create attendance");

    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let app = Router::new()
        .route("/api/admin/export", get(export_data))
        .layer(Extension(admin))
        .with_state(state);

    // M-3: default period is the current month when `from`/`to` are
    // omitted, so pin an explicit range to keep asserting on a fixed,
    // historical date regardless of when the suite runs.
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/export?from=2026-01-16&to=2026-01-16")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert!(response.status().is_success());
    let masked_header = response
        .headers()
        .get(HeaderName::from_static("x-pii-masked"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: Value = serde_json::from_slice(&body).expect("parse response");
    let csv_data = payload
        .get("csv_data")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    assert_eq!(masked_header, "true");
    assert!(csv_data.contains('*'));
    assert!(!csv_data.contains("Test User"));
}

#[tokio::test]
async fn admin_export_defaults_to_current_month_when_range_omitted() {
    // M-3: omitting `from`/`to` must fall back to the current month, not an
    // unbounded (all employees, all time) query.
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_attendance_tables(&pool).await;

    let admin = support::seed_user(&pool, UserRole::Manager, false).await;
    let employee = support::seed_user(&pool, UserRole::Employee, false).await;
    setup_manager_scope(&pool, &admin.id.to_string(), &employee.id.to_string()).await;

    let today = today_in_test_timezone();
    let month_start_date = month_start(today);
    let month_end_date = month_end(today);
    let out_of_month_date = month_start_date - Duration::days(1); // last day of previous month

    let now = Utc::now();
    let repo = AttendanceRepository::new();
    repo.create(&pool, &Attendance::new(employee.id, month_start_date, now))
        .await
        .expect("create first-of-month attendance");
    repo.create(&pool, &Attendance::new(employee.id, month_end_date, now))
        .await
        .expect("create last-of-month attendance");
    repo.create(&pool, &Attendance::new(employee.id, out_of_month_date, now))
        .await
        .expect("create out-of-month attendance");

    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let app = Router::new()
        .route("/api/admin/export", get(export_data))
        .layer(Extension(admin))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/export")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert!(response.status().is_success());
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: Value = serde_json::from_slice(&body).expect("parse response");
    let csv_data = payload
        .get("csv_data")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    assert!(csv_data.contains(&format!("\"{}\"", month_start_date.format("%Y-%m-%d"))));
    assert!(csv_data.contains(&format!("\"{}\"", month_end_date.format("%Y-%m-%d"))));
    assert!(!csv_data.contains(&format!("\"{}\"", out_of_month_date.format("%Y-%m-%d"))));
}

#[tokio::test]
async fn admin_export_rejects_range_exceeding_max_span() {
    // M-3: an explicit range wider than the maximum allowed span (366 days)
    // must be rejected with 400, protecting the unbounded
    // generate_series-based leave-days query from huge fan-out.
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_attendance_tables(&pool).await;

    let admin = support::seed_user(&pool, UserRole::Manager, false).await;

    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let app = Router::new()
        .route("/api/admin/export", get(export_data))
        .layer(Extension(admin))
        .with_state(state);

    let from = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date");
    let to = from + Duration::days(400);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/admin/export?from={}&to={}",
                    from.format("%Y-%m-%d"),
                    to.format("%Y-%m-%d")
                ))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
