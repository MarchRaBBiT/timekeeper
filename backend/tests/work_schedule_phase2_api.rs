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
    models::user::{User, UserRole},
    state::AppState,
};
use tower::ServiceExt;

mod support;

use support::{seed_attendance, seed_user, seed_work_schedule_for_user, test_config, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

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
