use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::{admin::workday_overrides as admin_handlers, attendance as attendance_handlers},
    models::user::{User, UserRole},
    state::AppState,
    types::UserId,
};
use tower::ServiceExt;
use uuid::Uuid;

mod support;
use support::integration_guard;

use support::{
    seed_flex_work_schedule_for_user, seed_user, seed_work_schedule_for_user, test_config,
    test_pool,
};

fn user_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/attendance/me/classification",
            get(attendance_handlers::get_my_classification),
        )
        .layer(Extension(user))
        .with_state(state)
}

fn admin_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/users/{user_id}/classification",
            get(admin_handlers::get_user_classification),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, body)
}

async fn migrate(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("run migrations");
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    date(year, month, day)
        .and_hms_opt(hour, minute, 0)
        .expect("valid datetime")
}

#[allow(clippy::too_many_arguments)]
async fn insert_resolved_workday(
    pool: &PgPool,
    user_id: UserId,
    work_date: NaiveDate,
    schedule_id: Uuid,
    version_id: Uuid,
    day_kind: &str,
    schedule_type: &str,
    expected_work_minutes: i32,
) {
    sqlx::query(
        "INSERT INTO resolved_workdays \
         (id, user_id, work_date, work_schedule_id, work_schedule_version_id, source, source_id, \
          day_kind, timezone, workday_boundary, expected_work_minutes, schedule_type, resolved_at, locked_at) \
         VALUES ($1, $2, $3, $4, $5, 'user', $6, $7, 'Asia/Tokyo', $8, $9, $10, $11, $11)
         ON CONFLICT (user_id, work_date) DO UPDATE SET
          day_kind = EXCLUDED.day_kind,
          expected_work_minutes = EXCLUDED.expected_work_minutes,
          schedule_type = EXCLUDED.schedule_type",
    )
    .bind(Uuid::new_v4())
    .bind(user_id.to_string())
    .bind(work_date)
    .bind(schedule_id)
    .bind(version_id)
    .bind(Uuid::new_v4())
    .bind(day_kind)
    .bind(NaiveTime::from_hms_opt(5, 0, 0).expect("boundary"))
    .bind(expected_work_minutes)
    .bind(schedule_type)
    .bind(Utc::now())
    .execute(pool)
    .await
    .expect("insert resolved workday");
}

async fn insert_attendance(
    pool: &PgPool,
    user_id: UserId,
    work_date: NaiveDate,
    clock_in: Option<NaiveDateTime>,
    clock_out: Option<NaiveDateTime>,
) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO attendance
         (id, user_id, date, clock_in_time, clock_out_time, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'present', NOW(), NOW())",
    )
    .bind(&id)
    .bind(user_id.to_string())
    .bind(work_date)
    .bind(clock_in)
    .bind(clock_out)
    .execute(pool)
    .await
    .expect("insert attendance");
    id
}

async fn insert_effective_correction(
    pool: &PgPool,
    user: &User,
    attendance_id: &str,
    work_date: NaiveDate,
    clock_in: NaiveDateTime,
    clock_out: NaiveDateTime,
    breaks: Value,
) {
    let request_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO attendance_correction_requests
         (id, user_id, attendance_id, date, status, reason,
          original_snapshot_json, proposed_values_json, approved_by, approved_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'approved', 'test correction',
          '{}'::jsonb, '{}'::jsonb, $2, NOW(), NOW(), NOW())",
    )
    .bind(&request_id)
    .bind(user.id.to_string())
    .bind(attendance_id)
    .bind(work_date)
    .execute(pool)
    .await
    .expect("insert correction request");
    sqlx::query(
        "INSERT INTO attendance_correction_effective_values
         (attendance_id, source_request_id, clock_in_time_corrected, clock_out_time_corrected,
          break_records_corrected_json, applied_by, applied_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())",
    )
    .bind(attendance_id)
    .bind(&request_id)
    .bind(clock_in)
    .bind(clock_out)
    .bind(breaks)
    .bind(user.id.to_string())
    .execute(pool)
    .await
    .expect("insert effective correction");
}

async fn assign_manager_to_employee_department(pool: &PgPool, manager: UserId, employee: UserId) {
    let department_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&department_id)
        .bind("Classification QA")
        .execute(pool)
        .await
        .expect("insert department");
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(&department_id)
        .bind(employee.to_string())
        .execute(pool)
        .await
        .expect("assign employee department");
    sqlx::query("INSERT INTO department_managers (department_id, user_id) VALUES ($1, $2)")
        .bind(&department_id)
        .bind(manager.to_string())
        .execute(pool)
        .await
        .expect("assign manager");
}

#[tokio::test]
async fn self_classification_uses_effective_values_and_marks_legal_holiday() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    migrate(&pool).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let (schedule_id, version_id) =
        seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    let monday_attendance = insert_attendance(
        &pool,
        employee.id,
        date(2026, 7, 6),
        Some(at(2026, 7, 6, 9, 0)),
        Some(at(2026, 7, 6, 17, 0)),
    )
    .await;
    insert_effective_correction(
        &pool,
        &employee,
        &monday_attendance,
        date(2026, 7, 6),
        at(2026, 7, 6, 21, 0),
        at(2026, 7, 7, 1, 0),
        json!([
            {
                "break_start_time": "2026-07-06T23:00:00",
                "break_end_time": "2026-07-06T23:30:00"
            }
        ]),
    )
    .await;

    insert_resolved_workday(
        &pool,
        employee.id,
        date(2026, 7, 5),
        schedule_id,
        version_id,
        "scheduled_non_working_day",
        "fixed",
        0,
    )
    .await;
    insert_attendance(
        &pool,
        employee.id,
        date(2026, 7, 5),
        Some(at(2026, 7, 5, 10, 0)),
        Some(at(2026, 7, 5, 12, 0)),
    )
    .await;

    let (status, body) = get_json(
        user_router(pool.clone(), employee),
        "/api/attendance/me/classification?year=2026&month=7",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "calculated");
    let days = body["days"].as_array().expect("days");
    let monday = days
        .iter()
        .find(|day| day["work_date"] == "2026-07-06")
        .expect("monday");
    assert_eq!(monday["actual_minutes"], 210);
    assert_eq!(monday["night_minutes"], 150);
    assert_eq!(monday["scheduled_minutes"], 210);
    let sunday = days
        .iter()
        .find(|day| day["work_date"] == "2026-07-05")
        .expect("sunday");
    assert_eq!(sunday["legal_holiday_minutes"], 120);
    assert_eq!(body["totals"]["actual_minutes"], 330);
    assert_eq!(body["totals"]["legal_holiday_minutes"], 120);
}

#[tokio::test]
async fn mixed_fixed_and_flex_month_classifies_fixed_day_with_resolved_expected_minutes() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    migrate(&pool).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let (schedule_id, version_id) = seed_flex_work_schedule_for_user(&pool, employee.id).await;
    insert_resolved_workday(
        &pool,
        employee.id,
        date(2026, 7, 6),
        schedule_id,
        version_id,
        "scheduled_workday",
        "fixed",
        420,
    )
    .await;
    insert_attendance(
        &pool,
        employee.id,
        date(2026, 7, 6),
        Some(at(2026, 7, 6, 9, 0)),
        Some(at(2026, 7, 6, 17, 0)),
    )
    .await;
    insert_attendance(
        &pool,
        employee.id,
        date(2026, 7, 7),
        Some(at(2026, 7, 7, 9, 0)),
        Some(at(2026, 7, 7, 17, 0)),
    )
    .await;

    let (status, body) = get_json(
        user_router(pool.clone(), employee),
        "/api/attendance/me/classification?year=2026&month=7",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "calculated");
    assert_eq!(body["flex_period"]["status"], "not_applicable");
    let days = body["days"].as_array().expect("days");
    let fixed_monday = days
        .iter()
        .find(|day| day["work_date"] == "2026-07-06")
        .expect("fixed monday");
    assert_eq!(fixed_monday["schedule_type"], "fixed");
    assert_eq!(fixed_monday["scheduled_minutes"], 420);
    assert_eq!(fixed_monday["statutory_within_minutes"], 60);
    let flex_tuesday = days
        .iter()
        .find(|day| day["work_date"] == "2026-07-07")
        .expect("flex tuesday");
    assert_eq!(flex_tuesday["schedule_type"], "flex");
    assert_eq!(flex_tuesday["actual_minutes"], 480);
    assert_eq!(flex_tuesday["scheduled_minutes"], 0);
}

#[tokio::test]
async fn admin_classification_uses_resolved_workday_department_scope() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    migrate(&pool).await;
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let outsider_manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;
    assign_manager_to_employee_department(&pool, manager.id, employee.id).await;

    let uri = format!(
        "/api/admin/users/{}/classification?year=2026&month=7",
        employee.id
    );
    let (status, body) = get_json(admin_router(pool.clone(), manager), &uri).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "calculated");

    let (status, body) = get_json(admin_router(pool.clone(), outsider_manager), &uri).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "Forbidden");
}
