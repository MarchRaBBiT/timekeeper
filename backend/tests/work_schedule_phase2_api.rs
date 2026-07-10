use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post, put},
    Extension, Router,
};
use chrono::{NaiveDate, NaiveDateTime};
use serde_json::{json, Value};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::{work_schedules as handlers, workday_overrides},
    models::{
        leave_request::LeaveType,
        user::{User, UserRole},
    },
    repositories::work_schedule,
    state::AppState,
};
use timekeeper_contract::work_schedules::WorkScheduleAnomalyKind;
use tower::ServiceExt;
use uuid::Uuid;

mod support;
use support::integration_guard;

use support::{
    seed_attendance, seed_break_record, seed_flex_work_schedule_for_user, seed_leave_request,
    seed_overtime_request, seed_user, seed_work_schedule_for_user, test_config, test_pool,
};

fn router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/work-schedule-projections/generate",
            post(handlers::generate_work_schedule_projections),
        )
        .route(
            "/api/admin/users/{user_id}/work-schedule-calendar",
            get(handlers::get_work_schedule_calendar),
        )
        .route(
            "/api/admin/work-schedule-anomalies",
            get(handlers::list_work_schedule_anomalies),
        )
        .route(
            "/api/admin/overtime-monitor",
            get(handlers::list_overtime_monitor),
        )
        .route(
            "/api/admin/overtime-monitor/settings",
            get(handlers::get_overtime_monitor_settings)
                .put(handlers::upsert_overtime_monitor_settings),
        )
        .route(
            "/api/monthly-closings/me/self-confirm",
            post(handlers::self_confirm_monthly_closing),
        )
        .route(
            "/api/admin/users/{user_id}/monthly-closings/approve",
            post(handlers::approve_monthly_closing),
        )
        .route(
            "/api/admin/users/{user_id}/monthly-closings/close",
            post(handlers::close_monthly_closing),
        )
        .route(
            "/api/admin/users/{user_id}/monthly-closings/reopen",
            post(handlers::reopen_monthly_closing),
        )
        .route(
            "/api/admin/work-schedule-assignments/bulk",
            post(handlers::bulk_create_work_schedule_assignments),
        )
        .route(
            "/api/admin/work-schedule-closures/monthly",
            post(handlers::close_work_schedule_month),
        )
        .route(
            "/api/admin/users/{user_id}/workday-overrides/{date}",
            put(workday_overrides::set_workday_override),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn request_json(
    app: Router,
    method: &str,
    uri: &str,
    payload: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = if let Some(value) = payload {
        builder = builder.header("Content-Type", "application/json");
        Body::from(value.to_string())
    } else {
        Body::empty()
    };
    let response = app
        .oneshot(builder.body(body).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, json)
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}

async fn assign_manager_to_employee_department(pool: &PgPool, manager: &User, employee: &User) {
    let department_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&department_id)
        .bind(format!("Department {department_id}"))
        .execute(pool)
        .await
        .expect("insert department");
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(&department_id)
        .bind(employee.id.to_string())
        .execute(pool)
        .await
        .expect("assign employee department");
    sqlx::query("INSERT INTO department_managers (department_id, user_id) VALUES ($1, $2)")
        .bind(&department_id)
        .bind(manager.id.to_string())
        .execute(pool)
        .await
        .expect("assign manager");
}

#[tokio::test]
async fn projection_calendar_and_monthly_close_lock_workdays() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    let (status, body) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [employee.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-03"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["projected"], 3);
    assert_eq!(body["not_configured"], 0);

    let (calendar_status, calendar) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        &format!(
            "/api/admin/users/{}/work-schedule-calendar?from=2026-07-01&to=2026-07-03",
            employee.id
        ),
        None,
    )
    .await;
    assert_eq!(calendar_status, StatusCode::OK);
    let days = calendar["days"].as_array().expect("calendar days");
    assert_eq!(days.len(), 3);
    assert_eq!(days[0]["resolved_workday"]["work_date"], "2026-07-01");

    let (close_status, close_body) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-closures/monthly",
        Some(json!({
            "year": 2026,
            "month": 7,
            "user_ids": [employee.id.to_string()],
            "reason": "July payroll close"
        })),
    )
    .await;
    assert_eq!(close_status, StatusCode::OK);
    assert_eq!(close_body["locked_count"], 3);

    let locked_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resolved_workdays \
         WHERE user_id = $1 AND work_date BETWEEN $2 AND $3 AND locked_at IS NOT NULL",
    )
    .bind(employee.id.to_string())
    .bind(date(2026, 7, 1))
    .bind(date(2026, 7, 3))
    .fetch_one(&pool)
    .await
    .expect("locked count");
    assert_eq!(locked_count, 3);

    let (override_status, override_body) = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        &format!(
            "/api/admin/users/{}/workday-overrides/2026-07-01",
            employee.id
        ),
        Some(json!({ "kind": "non_working_day", "reason": "after close" })),
    )
    .await;
    assert_eq!(override_status, StatusCode::CONFLICT);
    assert_eq!(override_body["code"], "RESOLVED_WORKDAY_LOCKED");
}

#[tokio::test]
async fn anomaly_list_detects_not_configured_and_missing_clock_out() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let unconfigured = seed_user(&pool, UserRole::Employee, false).await;
    let scheduled = seed_user(&pool, UserRole::Employee, false).await;
    let missing_clock_in_user = seed_user(&pool, UserRole::Employee, false).await;
    let unscheduled_user = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, scheduled.id, "non_working").await;
    seed_work_schedule_for_user(&pool, missing_clock_in_user.id, "non_working").await;
    seed_work_schedule_for_user(&pool, unscheduled_user.id, "non_working").await;

    let generated = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [
                scheduled.id.to_string(),
                missing_clock_in_user.id.to_string(),
                unscheduled_user.id.to_string()
            ],
            "from": "2026-07-01",
            "to": "2026-07-01"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);
    seed_attendance(
        &pool,
        scheduled.id,
        date(2026, 7, 1),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        None,
    )
    .await;

    let (not_configured_status, not_configured) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        &format!(
            "/api/admin/work-schedule-anomalies?user_id={}&from=2026-07-01&to=2026-07-01",
            unconfigured.id
        ),
        None,
    )
    .await;
    assert_eq!(not_configured_status, StatusCode::OK);
    assert_eq!(
        not_configured["items"][0]["kind"],
        "schedule_not_configured"
    );

    let (missing_out_status, missing_out) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        &format!(
            "/api/admin/work-schedule-anomalies?user_id={}&from=2026-07-01&to=2026-07-01",
            scheduled.id
        ),
        None,
    )
    .await;
    assert_eq!(missing_out_status, StatusCode::OK);
    assert_eq!(missing_out["items"][0]["kind"], "missing_clock_out");

    seed_attendance(
        &pool,
        missing_clock_in_user.id,
        date(2026, 7, 1),
        None,
        None,
    )
    .await;
    let (missing_in_status, missing_in) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        &format!(
            "/api/admin/work-schedule-anomalies?user_id={}&from=2026-07-01&to=2026-07-01",
            missing_clock_in_user.id
        ),
        None,
    )
    .await;
    assert_eq!(missing_in_status, StatusCode::OK);
    assert_eq!(missing_in["items"][0]["kind"], "missing_clock_in");

    let unscheduled_attendance = seed_attendance(
        &pool,
        unscheduled_user.id,
        date(2026, 7, 1),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T18:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock out"),
        ),
    )
    .await;
    let resolved_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "UPDATE resolved_workdays \
         SET day_kind = 'scheduled_non_working_day', locked_at = NOW() \
         WHERE user_id = $1 AND work_date = $2 RETURNING id",
    )
    .bind(unscheduled_user.id.to_string())
    .bind(date(2026, 7, 1))
    .fetch_one(&pool)
    .await
    .expect("lock non-working resolved workday");
    sqlx::query("UPDATE attendance SET resolved_workday_id = $1 WHERE id = $2")
        .bind(resolved_id)
        .bind(unscheduled_attendance.id.to_string())
        .execute(&pool)
        .await
        .expect("link unscheduled attendance");

    let (unscheduled_status, unscheduled) = request_json(
        router(pool.clone(), admin),
        "GET",
        &format!(
            "/api/admin/work-schedule-anomalies?user_id={}&from=2026-07-01&to=2026-07-01",
            unscheduled_user.id
        ),
        None,
    )
    .await;
    assert_eq!(unscheduled_status, StatusCode::OK);
    assert_eq!(unscheduled["items"][0]["kind"], "unscheduled_work");
}

#[tokio::test]
async fn anomaly_list_detects_missing_clock_out_for_flex_schedule_regardless_of_expected_minutes() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let flex_user = seed_user(&pool, UserRole::Employee, false).await;
    seed_flex_work_schedule_for_user(&pool, flex_user.id).await;

    let generated = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [flex_user.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-01"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);
    seed_attendance(
        &pool,
        flex_user.id,
        date(2026, 7, 1),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        None,
    )
    .await;

    // The flex projection's expected_work_minutes represents the flex band
    // (max possible span), not a contracted duration. Anomaly detection must
    // still flag the missing clock-out based on day_kind alone, not misread
    // the flex band as an unmet contracted-time shortfall.
    let (status, body) = request_json(
        router(pool.clone(), admin),
        "GET",
        &format!(
            "/api/admin/work-schedule-anomalies?user_id={}&from=2026-07-01&to=2026-07-01",
            flex_user.id
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"][0]["kind"], "missing_clock_out");
}

#[tokio::test]
async fn approved_leave_surfaces_in_calendar_and_clock_in_conflict_anomaly() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    let generated = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [employee.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-01"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);

    let leave_date = date(2026, 7, 1);
    let leave = seed_leave_request(
        &pool,
        employee.id,
        LeaveType::Annual,
        leave_date,
        leave_date,
    )
    .await;
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

    let (calendar_status, calendar) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        &format!(
            "/api/admin/users/{}/work-schedule-calendar?from=2026-07-01&to=2026-07-01",
            employee.id
        ),
        None,
    )
    .await;
    assert_eq!(calendar_status, StatusCode::OK);
    assert_eq!(calendar["days"][0]["leave"]["leave_type"], "annual");
    assert_eq!(
        calendar["days"][0]["leave"]["leave_request_id"],
        leave.id.to_string()
    );

    let (leave_only_status, leave_only_anomalies) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        &format!(
            "/api/admin/work-schedule-anomalies?user_id={}&from=2026-07-01&to=2026-07-01",
            employee.id
        ),
        None,
    )
    .await;
    assert_eq!(leave_only_status, StatusCode::OK);
    assert!(leave_only_anomalies["items"]
        .as_array()
        .expect("anomaly items")
        .is_empty());

    seed_attendance(
        &pool,
        employee.id,
        leave_date,
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T18:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock out"),
        ),
    )
    .await;

    let (conflict_status, conflict) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        &format!(
            "/api/admin/work-schedule-anomalies?user_id={}&from=2026-07-01&to=2026-07-01",
            employee.id
        ),
        None,
    )
    .await;
    assert_eq!(conflict_status, StatusCode::OK);
    assert_eq!(conflict["items"][0]["kind"], "leave_conflict");

    sqlx::query(
        "UPDATE leave_requests
         SET status = 'cancelled', cancelled_at = NOW(), updated_at = NOW()
         WHERE id = $1",
    )
    .bind(leave.id.to_string())
    .execute(&pool)
    .await
    .expect("cancel leave");

    let (after_cancel_status, after_cancel) = request_json(
        router(pool.clone(), admin),
        "GET",
        &format!(
            "/api/admin/users/{}/work-schedule-calendar?from=2026-07-01&to=2026-07-01",
            employee.id
        ),
        None,
    )
    .await;
    assert_eq!(after_cancel_status, StatusCode::OK);
    assert_eq!(after_cancel["days"][0]["leave"], Value::Null);
}

#[tokio::test]
async fn anomaly_list_detects_overtime_punctuality_absence_and_break_warnings() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let approved_overtime_user = seed_user(&pool, UserRole::Employee, false).await;
    let unapproved_overtime_user = seed_user(&pool, UserRole::Employee, false).await;
    let absent_user = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, approved_overtime_user.id, "non_working").await;
    seed_work_schedule_for_user(&pool, unapproved_overtime_user.id, "non_working").await;
    seed_work_schedule_for_user(&pool, absent_user.id, "non_working").await;

    let generated = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [
                approved_overtime_user.id.to_string(),
                unapproved_overtime_user.id.to_string(),
                absent_user.id.to_string()
            ],
            "from": "2026-07-01",
            "to": "2026-07-01"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);

    let overtime =
        seed_overtime_request(&pool, approved_overtime_user.id, date(2026, 7, 1), 1.0).await;
    sqlx::query(
        "UPDATE overtime_requests
         SET status = 'approved', approved_by = $1, approved_at = NOW(), updated_at = NOW()
         WHERE id = $2",
    )
    .bind(admin.id.to_string())
    .bind(overtime.id.to_string())
    .execute(&pool)
    .await
    .expect("approve overtime request");

    let attendance = seed_attendance(
        &pool,
        approved_overtime_user.id,
        date(2026, 7, 1),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T09:20:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T20:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock out"),
        ),
    )
    .await;
    seed_break_record(
        &pool,
        attendance.id,
        NaiveDateTime::parse_from_str("2026-07-01T12:00:00", "%Y-%m-%dT%H:%M:%S")
            .expect("break start"),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T12:30:00", "%Y-%m-%dT%H:%M:%S")
                .expect("break end"),
        ),
    )
    .await;

    seed_attendance(
        &pool,
        unapproved_overtime_user.id,
        date(2026, 7, 1),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T19:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock out"),
        ),
    )
    .await;

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "GET",
        "/api/admin/work-schedule-anomalies?from=2026-07-01&to=2026-07-01",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let kinds: Vec<_> = body["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["kind"].as_str().expect("kind"))
        .collect();
    assert!(kinds.contains(&"overtime_exceeds_request"));
    assert!(kinds.contains(&"unapproved_overtime"));
    assert!(kinds.contains(&"late"));
    assert!(kinds.contains(&"insufficient_break"));
    assert!(kinds.contains(&"absent"));
}

#[tokio::test]
async fn overtime_calculation_deducts_break_minutes() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    let settings = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        "/api/admin/overtime-monitor/settings",
        Some(json!({
            "valid_from": "2026-01-01",
            "fiscal_year_start_month": 4,
            "monthly_limit_minutes": 120,
            "yearly_limit_minutes": 600,
            "rolling_average_limit_minutes": 120,
            "single_month_absolute_limit_minutes": 300,
            "warning_ratio_percent": 50,
            "overtime_request_tolerance_minutes": 0
        })),
    )
    .await;
    assert_eq!(settings.0, StatusCode::OK);

    let generated = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [employee.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-02"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);

    // Day 1: worked exactly the scheduled 9:00-18:00 span (raw span = 540
    // minutes) and took the scheduled 60-minute break. Net worked time equals
    // expected_work_minutes (480), so no overtime should be detected even
    // though the raw clock span exceeds 480.
    let on_time = seed_attendance(
        &pool,
        employee.id,
        date(2026, 7, 1),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T18:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock out"),
        ),
    )
    .await;
    seed_break_record(
        &pool,
        on_time.id,
        NaiveDateTime::parse_from_str("2026-07-01T12:00:00", "%Y-%m-%dT%H:%M:%S")
            .expect("break start"),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T13:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("break end"),
        ),
    )
    .await;

    // Day 2: worked until 20:00 with the same 60-minute break. Net worked time
    // is 600 minutes, 120 minutes above expected — this should be detected,
    // and the excess must be the break-deducted figure (120), not the raw
    // clock span minus expected (660 - 480 = 180).
    let overworked = seed_attendance(
        &pool,
        employee.id,
        date(2026, 7, 2),
        Some(
            NaiveDateTime::parse_from_str("2026-07-02T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        Some(
            NaiveDateTime::parse_from_str("2026-07-02T20:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock out"),
        ),
    )
    .await;
    seed_break_record(
        &pool,
        overworked.id,
        NaiveDateTime::parse_from_str("2026-07-02T12:00:00", "%Y-%m-%dT%H:%M:%S")
            .expect("break start"),
        Some(
            NaiveDateTime::parse_from_str("2026-07-02T13:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("break end"),
        ),
    )
    .await;

    let (status, body) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        &format!(
            "/api/admin/work-schedule-anomalies?user_id={}&from=2026-07-01&to=2026-07-02",
            employee.id
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = body["items"].as_array().expect("items");
    let day1_has_overtime = items.iter().any(|item| {
        item["work_date"] == "2026-07-01"
            && (item["kind"] == "unapproved_overtime" || item["kind"] == "overtime_exceeds_request")
    });
    assert!(
        !day1_has_overtime,
        "break-compliant on-time day must not report an overtime anomaly"
    );
    let day2_has_overtime = items
        .iter()
        .any(|item| item["work_date"] == "2026-07-02" && item["kind"] == "unapproved_overtime");
    assert!(
        day2_has_overtime,
        "day exceeding expected work minutes net of break must report unapproved overtime"
    );

    let (monitor_status, monitor_body) = request_json(
        router(pool.clone(), admin),
        "GET",
        "/api/admin/overtime-monitor?year=2026&month=7",
        None,
    )
    .await;
    assert_eq!(monitor_status, StatusCode::OK);
    let employee_row = monitor_body["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["user_id"] == employee.id.to_string())
        .expect("employee row");
    assert_eq!(employee_row["month_statutory_excess_minutes"], 120);
}

#[tokio::test]
async fn flex_schedule_is_excluded_from_overtime_anomalies_and_monitor() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let flex_user = seed_user(&pool, UserRole::Employee, false).await;
    seed_flex_work_schedule_for_user(&pool, flex_user.id).await;

    let settings = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        "/api/admin/overtime-monitor/settings",
        Some(json!({
            "valid_from": "2026-01-01",
            "fiscal_year_start_month": 4,
            "monthly_limit_minutes": 120,
            "yearly_limit_minutes": 600,
            "rolling_average_limit_minutes": 120,
            "single_month_absolute_limit_minutes": 300,
            "warning_ratio_percent": 50,
            "overtime_request_tolerance_minutes": 0
        })),
    )
    .await;
    assert_eq!(settings.0, StatusCode::OK);

    let generated = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [flex_user.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-01"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);

    // A long clock span, well above the flex day's expected_work_minutes
    // (which represents the flex band's max span, not a contracted duration).
    // Flex users must not be judged against this figure for overtime purposes.
    seed_attendance(
        &pool,
        flex_user.id,
        date(2026, 7, 1),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T07:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T22:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock out"),
        ),
    )
    .await;

    let (status, body) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        &format!(
            "/api/admin/work-schedule-anomalies?user_id={}&from=2026-07-01&to=2026-07-01",
            flex_user.id
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let kinds: Vec<_> = body["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["kind"].as_str().expect("kind"))
        .collect();
    assert!(!kinds.contains(&"unapproved_overtime"));
    assert!(!kinds.contains(&"overtime_exceeds_request"));

    let (monitor_status, monitor_body) = request_json(
        router(pool.clone(), admin),
        "GET",
        "/api/admin/overtime-monitor?year=2026&month=7",
        None,
    )
    .await;
    assert_eq!(monitor_status, StatusCode::OK);
    let employee_row = monitor_body["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["user_id"] == flex_user.id.to_string())
        .expect("flex employee row is still present with zero excess minutes");
    assert_eq!(employee_row["month_statutory_excess_minutes"], 0);
}

#[tokio::test]
async fn overtime_monitor_fiscal_year_total_includes_months_before_rolling_window() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    // Fiscal year starts in April. Querying month=12 means the fiscal year
    // began 8 months earlier, but the 6-month rolling window only reaches
    // back to July. April attendance must still be included in the fiscal
    // year total even though it falls outside the rolling window.
    let settings = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        "/api/admin/overtime-monitor/settings",
        Some(json!({
            "valid_from": "2026-01-01",
            "fiscal_year_start_month": 4,
            "monthly_limit_minutes": 1000,
            "yearly_limit_minutes": 1000,
            "rolling_average_limit_minutes": 1000,
            "single_month_absolute_limit_minutes": 1000,
            "warning_ratio_percent": 50,
            "overtime_request_tolerance_minutes": 0
        })),
    )
    .await;
    assert_eq!(settings.0, StatusCode::OK);

    let generated = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [employee.id.to_string()],
            "from": "2026-04-01",
            "to": "2026-04-01"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);
    seed_attendance(
        &pool,
        employee.id,
        date(2026, 4, 1),
        Some(
            NaiveDateTime::parse_from_str("2026-04-01T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        Some(
            NaiveDateTime::parse_from_str("2026-04-01T20:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock out"),
        ),
    )
    .await;

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "GET",
        "/api/admin/overtime-monitor?year=2026&month=12",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let employee_row = body["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["user_id"] == employee.id.to_string())
        .expect("employee row");
    // April overtime (180 minutes: 660 raw - 480 expected, no break recorded)
    // must be reflected in the fiscal year total for a December query.
    assert_eq!(employee_row["fiscal_year_statutory_excess_minutes"], 180);
    // April is outside the 6-month rolling window ending in December
    // (Jul-Dec), so it must not appear in the rolling average.
    assert_eq!(employee_row["rolling_average_statutory_excess_minutes"], 0);
    assert_eq!(employee_row["month_statutory_excess_minutes"], 0);
}

#[tokio::test]
async fn overtime_monitor_reports_threshold_statuses() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    let settings = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        "/api/admin/overtime-monitor/settings",
        Some(json!({
            "valid_from": "2026-01-01",
            "fiscal_year_start_month": 4,
            "monthly_limit_minutes": 120,
            "yearly_limit_minutes": 600,
            "rolling_average_limit_minutes": 120,
            "single_month_absolute_limit_minutes": 300,
            "warning_ratio_percent": 50,
            "overtime_request_tolerance_minutes": 0
        })),
    )
    .await;
    assert_eq!(settings.0, StatusCode::OK);

    let generated = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [employee.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-01"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);
    seed_attendance(
        &pool,
        employee.id,
        date(2026, 7, 1),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock in"),
        ),
        Some(
            NaiveDateTime::parse_from_str("2026-07-01T20:00:00", "%Y-%m-%dT%H:%M:%S")
                .expect("clock out"),
        ),
    )
    .await;

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "GET",
        "/api/admin/overtime-monitor?year=2026&month=7",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let employee_row = body["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["user_id"] == employee.id.to_string())
        .expect("employee row");
    assert_eq!(employee_row["month_statutory_excess_minutes"], 180);
    assert_eq!(employee_row["monthly_status"], "exceeded");
}

#[tokio::test]
async fn upsert_overtime_monitor_settings_rejects_minutes_beyond_i32_range() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;

    // i32::MAX + 1 minutes overflows the `INTEGER` DB column. Before the fix this
    // reached the repository's `i32::try_from`, which returned `CorruptData` and
    // surfaced as a 500 INTERNAL_SERVER_ERROR instead of a validation error.
    let overflow_minutes = i64::from(i32::MAX) + 1;
    let (status, body) = request_json(
        router(pool.clone(), admin),
        "PUT",
        "/api/admin/overtime-monitor/settings",
        Some(json!({
            "valid_from": "2026-01-01",
            "fiscal_year_start_month": 4,
            "monthly_limit_minutes": overflow_minutes,
            "yearly_limit_minutes": 600,
            "rolling_average_limit_minutes": 120,
            "single_month_absolute_limit_minutes": 300,
            "warning_ratio_percent": 50,
            "overtime_request_tolerance_minutes": 0
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE");
}

#[tokio::test]
async fn anomaly_list_uses_caller_supplied_today_for_absent_boundary() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    // Both dates are scheduled weekdays; neither has any attendance recorded.
    let yesterday = date(2026, 7, 7);
    let today = date(2026, 7, 8);

    let generated = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [employee.id.to_string()],
            "from": "2026-07-07",
            "to": "2026-07-08"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);

    // Call the repository directly with an explicit `today` so the boundary is
    // deterministic regardless of the wall-clock date the suite runs on.
    let anomalies = work_schedule::list_anomalies(
        &pool,
        Some(vec![employee.id.to_string()]),
        yesterday,
        today,
        today,
    )
    .await
    .expect("list anomalies");

    let kind_for = |work_date: NaiveDate| {
        anomalies
            .iter()
            .find(|item| item.work_date == work_date)
            .unwrap_or_else(|| panic!("no anomaly recorded for {work_date}"))
            .kind
    };

    assert_eq!(
        kind_for(yesterday),
        WorkScheduleAnomalyKind::Absent,
        "a scheduled workday strictly before `today` with no attendance must be absent"
    );
    assert_eq!(
        kind_for(today),
        WorkScheduleAnomalyKind::MissingClockIn,
        "`today` itself must not be treated as a past day yet"
    );
}

#[tokio::test]
async fn monthly_closing_workflow_enforces_order_and_locks_on_close() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let system_admin = seed_user(&pool, UserRole::Manager, true).await;
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    assign_manager_to_employee_department(&pool, &manager, &employee).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    let generated = request_json(
        router(pool.clone(), system_admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [employee.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-01"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);

    let premature_close = request_json(
        router(pool.clone(), system_admin.clone()),
        "POST",
        &format!("/api/admin/users/{}/monthly-closings/close", employee.id),
        Some(json!({ "year": 2026, "month": 7, "reason": "too early" })),
    )
    .await;
    assert_eq!(premature_close.0, StatusCode::CONFLICT);
    assert_eq!(
        premature_close.1["code"],
        "INVALID_MONTHLY_CLOSING_TRANSITION"
    );

    let self_confirmed = request_json(
        router(pool.clone(), employee.clone()),
        "POST",
        "/api/monthly-closings/me/self-confirm",
        Some(json!({ "year": 2026, "month": 7, "reason": "confirmed" })),
    )
    .await;
    assert_eq!(self_confirmed.0, StatusCode::OK);
    assert_eq!(self_confirmed.1["status"], "self_confirmed");

    let approved = request_json(
        router(pool.clone(), manager.clone()),
        "POST",
        &format!("/api/admin/users/{}/monthly-closings/approve", employee.id),
        Some(json!({ "year": 2026, "month": 7, "reason": "approved" })),
    )
    .await;
    assert_eq!(approved.0, StatusCode::OK);
    assert_eq!(approved.1["status"], "approved");

    let closed = request_json(
        router(pool.clone(), system_admin.clone()),
        "POST",
        &format!("/api/admin/users/{}/monthly-closings/close", employee.id),
        Some(json!({ "year": 2026, "month": 7, "reason": "close" })),
    )
    .await;
    assert_eq!(closed.0, StatusCode::OK);
    assert_eq!(closed.1["status"], "closed");

    let locked_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resolved_workdays
         WHERE user_id = $1 AND work_date = $2 AND locked_at IS NOT NULL",
    )
    .bind(employee.id.to_string())
    .bind(date(2026, 7, 1))
    .fetch_one(&pool)
    .await
    .expect("locked count");
    assert_eq!(locked_count, 1);

    let reopened = request_json(
        router(pool.clone(), system_admin),
        "POST",
        &format!("/api/admin/users/{}/monthly-closings/reopen", employee.id),
        Some(json!({ "year": 2026, "month": 7, "reason": "audit" })),
    )
    .await;
    assert_eq!(reopened.0, StatusCode::OK);
    assert_eq!(reopened.1["status"], "reopened");
}

#[tokio::test]
async fn manager_cannot_approve_own_monthly_closing() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let system_admin = seed_user(&pool, UserRole::Manager, true).await;
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    // The manager is also a member of the department they manage, so
    // `authorize_scope` alone would let them approve their own month.
    assign_manager_to_employee_department(&pool, &manager, &manager).await;
    seed_work_schedule_for_user(&pool, manager.id, "non_working").await;

    let generated = request_json(
        router(pool.clone(), system_admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [manager.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-01"
        })),
    )
    .await;
    assert_eq!(generated.0, StatusCode::OK);

    let self_confirmed = request_json(
        router(pool.clone(), manager.clone()),
        "POST",
        "/api/monthly-closings/me/self-confirm",
        Some(json!({ "year": 2026, "month": 7, "reason": "confirmed" })),
    )
    .await;
    assert_eq!(self_confirmed.0, StatusCode::OK);
    assert_eq!(self_confirmed.1["status"], "self_confirmed");

    let self_approve = request_json(
        router(pool.clone(), manager.clone()),
        "POST",
        &format!("/api/admin/users/{}/monthly-closings/approve", manager.id),
        Some(json!({ "year": 2026, "month": 7, "reason": "self approve" })),
    )
    .await;
    assert_eq!(self_approve.0, StatusCode::FORBIDDEN);

    let status: String = sqlx::query_scalar(
        "SELECT status FROM monthly_closing_workflows
         WHERE user_id = $1 AND year = $2 AND month = $3",
    )
    .bind(manager.id.to_string())
    .bind(2026_i32)
    .bind(7_i32)
    .fetch_one(&pool)
    .await
    .expect("workflow status");
    assert_eq!(status, "self_confirmed");
}

#[tokio::test]
async fn approve_monthly_closing_rejects_unknown_target_user() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let system_admin = seed_user(&pool, UserRole::Manager, true).await;
    // System admins bypass the department-scope check in `authorize_scope`, so
    // a valid-format but nonexistent target user_id reaches the repository
    // layer, where `monthly_closing_workflows.user_id` has a FK to `users`.
    let unknown_user_id = Uuid::new_v4().to_string();

    let (status, body) = request_json(
        router(pool.clone(), system_admin),
        "POST",
        &format!("/api/admin/users/{unknown_user_id}/monthly-closings/approve"),
        Some(json!({ "year": 2026, "month": 7, "reason": "approve unknown user" })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE_REFERENCE");

    let workflow_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM monthly_closing_workflows WHERE user_id = $1")
            .bind(&unknown_user_id)
            .fetch_one(&pool)
            .await
            .expect("workflow count");
    assert_eq!(workflow_count, 0);
}

#[tokio::test]
async fn concurrent_first_self_confirm_requests_do_not_500() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    // Two concurrent first-requests for the same user/year/month (e.g. a
    // double-click on self-confirm) used to race on the initial workflow
    // INSERT: both could observe no existing row and both attempt the INSERT,
    // and the loser hit the `monthly_closing_workflows_user_month_key` unique
    // violation as a raw 500 instead of the normal 409 transition-conflict
    // response. Assert that never happens, regardless of scheduling.
    let first = request_json(
        router(pool.clone(), employee.clone()),
        "POST",
        "/api/monthly-closings/me/self-confirm",
        Some(json!({ "year": 2026, "month": 8, "reason": "confirm 1" })),
    );
    let second = request_json(
        router(pool.clone(), employee.clone()),
        "POST",
        "/api/monthly-closings/me/self-confirm",
        Some(json!({ "year": 2026, "month": 8, "reason": "confirm 2" })),
    );
    let (first_result, second_result) = tokio::join!(first, second);

    let mut statuses = vec![first_result.0, second_result.0];
    statuses.sort_by_key(|status| status.as_u16());
    assert_eq!(
        statuses,
        vec![StatusCode::OK, StatusCode::CONFLICT],
        "expected exactly one winner (200) and one conflict (409), got {:?} / {:?}",
        first_result,
        second_result
    );

    let workflow_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM monthly_closing_workflows
         WHERE user_id = $1 AND year = $2 AND month = $3",
    )
    .bind(employee.id.to_string())
    .bind(2026_i32)
    .bind(8_i32)
    .fetch_one(&pool)
    .await
    .expect("workflow count");
    assert_eq!(workflow_count, 1);
}

#[tokio::test]
async fn manager_anomaly_list_without_user_id_is_limited_to_subordinates() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let subordinate = seed_user(&pool, UserRole::Employee, false).await;
    let outside_user = seed_user(&pool, UserRole::Employee, false).await;
    assign_manager_to_employee_department(&pool, &manager, &subordinate).await;

    let (status, body) = request_json(
        router(pool.clone(), manager),
        "GET",
        "/api/admin/work-schedule-anomalies?from=2026-07-01&to=2026-07-01",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let returned_user_ids: Vec<_> = body["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["user_id"].as_str().expect("user_id"))
        .collect();
    assert!(returned_user_ids.contains(&subordinate.id.to_string().as_str()));
    assert!(!returned_user_ids.contains(&outside_user.id.to_string().as_str()));
}

#[tokio::test]
async fn bulk_assignment_reports_per_target_results() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let schedule_owner = seed_user(&pool, UserRole::Employee, false).await;
    let target_one = seed_user(&pool, UserRole::Employee, false).await;
    let target_two = seed_user(&pool, UserRole::Employee, false).await;
    let (schedule_id, _) =
        seed_work_schedule_for_user(&pool, schedule_owner.id, "non_working").await;

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/work-schedule-assignments/bulk",
        Some(json!({
            "work_schedule_id": schedule_id.to_string(),
            "targets": [
                { "type": "user", "user_id": target_one.id.to_string() },
                { "type": "user", "user_id": target_two.id.to_string() }
            ],
            "valid_from": "2027-01-01",
            "valid_until": null
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["created"].as_array().expect("created").len(), 2);
    assert!(body["failed"].as_array().expect("failed").is_empty());
}

#[tokio::test]
async fn bulk_assignment_rejects_too_many_targets() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let schedule_owner = seed_user(&pool, UserRole::Employee, false).await;
    let (schedule_id, _) =
        seed_work_schedule_for_user(&pool, schedule_owner.id, "non_working").await;
    let targets: Vec<_> = (0..501)
        .map(|_| json!({ "type": "user", "user_id": Uuid::new_v4().to_string() }))
        .collect();

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/work-schedule-assignments/bulk",
        Some(json!({
            "work_schedule_id": schedule_id.to_string(),
            "targets": targets,
            "valid_from": "2027-01-01",
            "valid_until": null
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE");
}

#[tokio::test]
async fn projection_generation_rejects_too_many_users() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let user_ids: Vec<_> = (0..501).map(|_| Uuid::new_v4().to_string()).collect();

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": user_ids,
            "from": "2026-07-01",
            "to": "2026-07-01"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE");
}

#[tokio::test]
async fn bulk_assignment_hides_database_error_details() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let schedule_owner = seed_user(&pool, UserRole::Employee, false).await;
    let (schedule_id, _) =
        seed_work_schedule_for_user(&pool, schedule_owner.id, "non_working").await;

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/work-schedule-assignments/bulk",
        Some(json!({
            "work_schedule_id": schedule_id.to_string(),
            "targets": [
                { "type": "user", "user_id": Uuid::new_v4().to_string() }
            ],
            "valid_from": "2027-01-01",
            "valid_until": null
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["failed"][0]["code"], "INVALID_WORK_SCHEDULE_REFERENCE");
    assert_eq!(
        body["failed"][0]["message"],
        "Referenced user or department does not exist"
    );
}

#[tokio::test]
async fn monthly_close_is_idempotent_when_month_is_already_locked() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    let projection = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [employee.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-03"
        })),
    )
    .await;
    assert_eq!(projection.0, StatusCode::OK);

    for expected_locked_count in [3, 0] {
        let (status, body) = request_json(
            router(pool.clone(), admin.clone()),
            "POST",
            "/api/admin/work-schedule-closures/monthly",
            Some(json!({
                "year": 2026,
                "month": 7,
                "user_ids": [employee.id.to_string()],
                "reason": "July payroll close"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["locked_count"], expected_locked_count);
    }

    let closure_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM work_schedule_monthly_closures \
         WHERE year = 2026 AND month = 7 AND user_ids = $1",
    )
    .bind(vec![employee.id.to_string()])
    .fetch_one(&pool)
    .await
    .expect("closure count");
    assert_eq!(closure_count, 1);
}

#[tokio::test]
async fn monthly_close_rejects_unknown_user_ids() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/work-schedule-closures/monthly",
        Some(json!({
            "year": 2026,
            "month": 7,
            "user_ids": [Uuid::new_v4().to_string()],
            "reason": "invalid user"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE_REFERENCE");
}

#[tokio::test]
async fn monthly_close_rejects_year_outside_supported_range() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;

    let (status, body) = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/work-schedule-closures/monthly",
        Some(json!({
            "year": 10000,
            "month": 7,
            "user_ids": [],
            "reason": "invalid year"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE");
}

#[tokio::test]
async fn monthly_close_rolls_back_locks_when_closure_insert_fails() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    let projection = request_json(
        router(pool.clone(), admin),
        "POST",
        "/api/admin/work-schedule-projections/generate",
        Some(json!({
            "user_ids": [employee.id.to_string()],
            "from": "2026-07-01",
            "to": "2026-07-03"
        })),
    )
    .await;
    assert_eq!(projection.0, StatusCode::OK);

    let result = work_schedule::close_month(
        &pool,
        2026,
        7,
        &[employee.id.to_string()],
        "missing-closed-by-user",
        Some("should fail"),
    )
    .await;
    assert!(result.is_err());

    let locked_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resolved_workdays \
         WHERE user_id = $1 AND work_date BETWEEN $2 AND $3 AND locked_at IS NOT NULL",
    )
    .bind(employee.id.to_string())
    .bind(date(2026, 7, 1))
    .bind(date(2026, 7, 3))
    .fetch_one(&pool)
    .await
    .expect("locked count");
    assert_eq!(locked_count, 0);
}
