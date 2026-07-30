use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde_json::{json, Value};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::{
        admin::{
            attendance_correction_requests as admin_corrections,
            workday_overrides as admin_handlers,
        },
        attendance_correction_requests as user_corrections, work_schedules,
    },
    models::user::{User, UserRole},
    state::AppState,
};
use tower::ServiceExt;
use uuid::Uuid;

mod support;
use support::{
    create_test_token, integration_guard, seed_flex_work_schedule_for_user, seed_user,
    seed_work_schedule_for_user, test_config, test_pool,
};

fn router(pool: PgPool, user: User, admin: bool) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    let route = if admin {
        Router::new().route(
            "/api/admin/users/{user_id}/settlement-balance",
            get(admin_handlers::get_user_settlement_balance),
        )
    } else {
        Router::new().route(
            "/api/work-schedules/me/settlement-balance",
            get(work_schedules::get_my_settlement_balance),
        )
    };
    route.layer(Extension(user)).with_state(state)
}

fn correction_user_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/attendance-corrections",
            axum::routing::post(user_corrections::create_attendance_correction_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

fn correction_admin_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/attendance-corrections/{id}/approve",
            axum::routing::put(admin_corrections::approve_attendance_correction_request),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
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
    (status, serde_json::from_slice(&bytes).expect("json body"))
}

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 7, day).expect("date")
}

fn at(day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    date(day).and_hms_opt(hour, minute, 0).expect("datetime")
}

async fn insert_attendance(pool: &PgPool, user: &User) -> String {
    let attendance_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO attendance
         (id, user_id, date, clock_in_time, clock_out_time, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'present', NOW(), NOW())",
    )
    .bind(&attendance_id)
    .bind(user.id.to_string())
    .bind(date(1))
    .bind(at(1, 9, 0))
    .bind(at(1, 18, 0))
    .execute(pool)
    .await
    .expect("attendance");
    sqlx::query(
        "INSERT INTO break_records
         (id, attendance_id, break_start_time, break_end_time, created_at, updated_at)
         VALUES ($1, $2, $3, $4, NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&attendance_id)
    .bind(at(1, 12, 0))
    .bind(at(1, 13, 0))
    .execute(pool)
    .await
    .expect("break");
    attendance_id
}

async fn insert_effective_correction(pool: &PgPool, user: &User, attendance_id: &str) {
    let request_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO attendance_correction_requests
         (id, user_id, attendance_id, date, status, reason,
          original_snapshot_json, proposed_values_json, approved_by, approved_at,
          created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'approved', 'settlement test',
          '{}'::jsonb, '{}'::jsonb, $2, NOW(), NOW(), NOW())",
    )
    .bind(&request_id)
    .bind(user.id.to_string())
    .bind(attendance_id)
    .bind(date(1))
    .execute(pool)
    .await
    .expect("correction request");
    sqlx::query(
        "INSERT INTO attendance_correction_effective_values
         (attendance_id, source_request_id, clock_in_time_corrected, clock_out_time_corrected,
          break_records_corrected_json, applied_by, applied_at, updated_at)
         VALUES ($1, $2, $3, $4, '[]'::jsonb, $5, NOW(), NOW())",
    )
    .bind(attendance_id)
    .bind(request_id)
    .bind(at(1, 8, 30))
    .bind(at(1, 17, 30))
    .bind(user.id.to_string())
    .execute(pool)
    .await
    .expect("effective correction");
}

async fn insert_additional_attendance(
    pool: &PgPool,
    user: &User,
    work_date: NaiveDate,
    clock_in: NaiveDateTime,
    clock_out: Option<NaiveDateTime>,
) {
    sqlx::query(
        "INSERT INTO attendance
         (id, user_id, date, clock_in_time, clock_out_time, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'present', NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user.id.to_string())
    .bind(work_date)
    .bind(clock_in)
    .bind(clock_out)
    .execute(pool)
    .await
    .expect("additional attendance");
}

async fn insert_published_version(
    pool: &PgPool,
    user: &User,
    settlement_minutes: Option<i32>,
) -> (Uuid, Uuid) {
    let schedule_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let schedule_code = format!("st-{}", &schedule_id.to_string()[..8]);
    sqlx::query("INSERT INTO work_schedules (id, code, name, created_by) VALUES ($1, $2, $3, $4)")
        .bind(schedule_id)
        .bind(schedule_code)
        .bind("Settlement test schedule")
        .bind(user.id.to_string())
        .execute(pool)
        .await
        .expect("schedule");
    sqlx::query(
        "INSERT INTO work_schedule_versions
         (id, work_schedule_id, version_number, status, effective_from, timezone,
          workday_boundary, public_holiday_policy, schedule_type)
         VALUES ($1, $2, 1, 'draft', '2000-01-01', 'Asia/Tokyo', '05:00', 'non_working', 'flex')",
    )
    .bind(version_id)
    .bind(schedule_id)
    .execute(pool)
    .await
    .expect("version");
    if let Some(minutes) = settlement_minutes {
        sqlx::query(
            "INSERT INTO work_schedule_settlement_periods
             (version_id, unit, contracted_minutes_per_period) VALUES ($1, 'monthly', $2)",
        )
        .bind(version_id)
        .bind(minutes)
        .execute(pool)
        .await
        .expect("settlement");
    }
    sqlx::query(
        "UPDATE work_schedule_versions
         SET status = 'published', published_by = $2, published_at = NOW() WHERE id = $1",
    )
    .bind(version_id)
    .bind(user.id.to_string())
    .execute(pool)
    .await
    .expect("publish");
    (schedule_id, version_id)
}

async fn self_balance(pool: PgPool, employee: User) -> (StatusCode, Value) {
    get_json(
        router(pool, employee, false),
        "/api/work-schedules/me/settlement-balance?year=2026&month=7",
    )
    .await
}

async fn assign_manager(pool: &PgPool, manager: &User, employee: &User) {
    let department_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, 'Settlement QA')")
        .bind(&department_id)
        .execute(pool)
        .await
        .expect("department");
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(&department_id)
        .bind(employee.id.to_string())
        .execute(pool)
        .await
        .expect("employee department");
    sqlx::query("INSERT INTO department_managers (department_id, user_id) VALUES ($1, $2)")
        .bind(department_id)
        .bind(manager.id.to_string())
        .execute(pool)
        .await
        .expect("manager department");
}

#[tokio::test]
async fn self_endpoint_materializes_flex_month_and_calculates_raw_minutes() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_flex_work_schedule_for_user(&pool, employee.id).await;
    let _ = self_balance(pool.clone(), employee.clone()).await;
    sqlx::query(
        "UPDATE resolved_workdays SET locked_at = NOW()
         WHERE user_id = $1 AND work_date = '2026-07-01'",
    )
    .bind(employee.id.to_string())
    .execute(&pool)
    .await
    .expect("lock first projection");
    let attendance_id = insert_attendance(&pool, &employee).await;
    insert_effective_correction(&pool, &employee, &attendance_id).await;
    insert_additional_attendance(&pool, &employee, date(2), at(2, 9, 0), None).await;
    insert_additional_attendance(
        &pool,
        &employee,
        date(31),
        at(31, 21, 0),
        Some(
            NaiveDate::from_ymd_opt(2026, 8, 1)
                .expect("next month")
                .and_hms_opt(5, 0, 0)
                .expect("overnight clock out"),
        ),
    )
    .await;

    let (status, body) = get_json(
        router(pool, employee, false),
        "/api/work-schedules/me/settlement-balance?year=2026&month=7",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "calculated");
    assert_eq!(body["contracted_minutes"], 9600);
    assert_eq!(body["actual_minutes"], 1020);
    assert_eq!(body["balance_minutes"], -8580);
    let days = body["days"].as_array().expect("days");
    assert_eq!(days.len(), 31);
    assert_eq!(days[0]["actual_minutes"], 540);
    assert_eq!(days[0]["locked"], true);
    assert_eq!(days[1]["actual_minutes"], 0);
    assert_eq!(days[1]["in_progress"], true);
    assert_eq!(days[30]["actual_minutes"], 480);
}

#[tokio::test]
async fn request_validation_and_admin_scope_are_enforced() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let manager = seed_user(&pool, UserRole::Manager, false).await;

    let (status, body) = get_json(
        router(pool.clone(), employee.clone(), false),
        "/api/work-schedules/me/settlement-balance?year=2026&month=13",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE");

    let uri = format!(
        "/api/admin/users/{}/settlement-balance?year=2026&month=7",
        employee.id
    );
    let (status, _) = get_json(router(pool, manager, true), &uri).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unavailable_statuses_are_returned_as_tagged_200_responses() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let zero_resolved = seed_user(&pool, UserRole::Employee, false).await;
    let (status, body) = self_balance(pool.clone(), zero_resolved).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "unresolved_days");

    let one_missing = seed_user(&pool, UserRole::Employee, false).await;
    seed_flex_work_schedule_for_user(&pool, one_missing.id).await;
    sqlx::query(
        "UPDATE work_schedule_assignments SET valid_from = '2026-07-02' WHERE user_id = $1",
    )
    .bind(one_missing.id.to_string())
    .execute(&pool)
    .await
    .expect("shift assignment");
    let (_, body) = self_balance(pool.clone(), one_missing).await;
    assert_eq!(body["status"], "unresolved_days");

    let fixed = seed_user(&pool, UserRole::Employee, false).await;
    seed_work_schedule_for_user(&pool, fixed.id, "non_working").await;
    let (_, body) = self_balance(pool.clone(), fixed).await;
    assert_eq!(body["status"], "not_applicable");

    let mixed = seed_user(&pool, UserRole::Employee, false).await;
    seed_flex_work_schedule_for_user(&pool, mixed.id).await;
    let _ = self_balance(pool.clone(), mixed.clone()).await;
    sqlx::query(
        "UPDATE resolved_workdays SET schedule_type = 'fixed', locked_at = NOW()
         WHERE user_id = $1 AND work_date = '2026-07-31'",
    )
    .bind(mixed.id.to_string())
    .execute(&pool)
    .await
    .expect("mix schedule type");
    let (_, body) = self_balance(pool.clone(), mixed).await;
    assert_eq!(body["status"], "not_applicable");

    let version_mixed = seed_user(&pool, UserRole::Employee, false).await;
    seed_flex_work_schedule_for_user(&pool, version_mixed.id).await;
    let _ = self_balance(pool.clone(), version_mixed.clone()).await;
    let (schedule_id, version_id) =
        insert_published_version(&pool, &version_mixed, Some(10000)).await;
    sqlx::query(
        "UPDATE resolved_workdays
         SET work_schedule_id = $2, work_schedule_version_id = $3,
             day_kind = 'scheduled_non_working_day', locked_at = NOW()
         WHERE user_id = $1 AND work_date = '2026-07-31'",
    )
    .bind(version_mixed.id.to_string())
    .bind(schedule_id)
    .bind(version_id)
    .execute(&pool)
    .await
    .expect("mix version on non-working day");
    let (_, body) = self_balance(pool.clone(), version_mixed).await;
    assert_eq!(body["status"], "version_mixed");

    let not_configured = seed_user(&pool, UserRole::Employee, false).await;
    seed_flex_work_schedule_for_user(&pool, not_configured.id).await;
    let _ = self_balance(pool.clone(), not_configured.clone()).await;
    let (schedule_id, version_id) = insert_published_version(&pool, &not_configured, None).await;
    sqlx::query(
        "UPDATE resolved_workdays
         SET work_schedule_id = $2, work_schedule_version_id = $3, locked_at = NOW()
         WHERE user_id = $1 AND work_date BETWEEN '2026-07-01' AND '2026-07-31'",
    )
    .bind(not_configured.id.to_string())
    .bind(schedule_id)
    .bind(version_id)
    .execute(&pool)
    .await
    .expect("point to version without settlement");
    let (_, body) = self_balance(pool, not_configured).await;
    assert_eq!(body["status"], "not_configured");
}

#[tokio::test]
async fn admin_authorization_and_request_boundaries_are_fixed() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let system_admin = seed_user(&pool, UserRole::Manager, true).await;
    seed_flex_work_schedule_for_user(&pool, employee.id).await;
    assign_manager(&pool, &manager, &employee).await;
    let uri = format!(
        "/api/admin/users/{}/settlement-balance?year=2026&month=7",
        employee.id
    );
    for actor in [manager, system_admin] {
        let (status, body) = get_json(router(pool.clone(), actor, true), &uri).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "calculated");
    }
    let (status, body) = get_json(
        router(
            pool.clone(),
            seed_user(&pool, UserRole::Manager, true).await,
            true,
        ),
        "/api/admin/users/not-a-uuid/settlement-balance?year=2026&month=7",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE");

    for query in [
        "year=1900&month=1",
        "year=9999&month=12",
        "year=1899&month=1",
        "year=10000&month=1",
        "year=2026&month=0",
    ] {
        let employee = seed_user(&pool, UserRole::Employee, false).await;
        let (status, body) = get_json(
            router(pool.clone(), employee, false),
            &format!("/api/work-schedules/me/settlement-balance?{query}"),
        )
        .await;
        if query.starts_with("year=1900") || query.starts_with("year=9999") {
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["status"], "unresolved_days");
        } else {
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(body["code"], "INVALID_WORK_SCHEDULE");
        }
    }
}

#[tokio::test]
async fn approved_correction_http_flow_updates_locked_month_balance() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    assign_manager(&pool, &manager, &employee).await;
    seed_flex_work_schedule_for_user(&pool, employee.id).await;
    insert_attendance(&pool, &employee).await;

    let (_, before) = self_balance(pool.clone(), employee.clone()).await;
    assert_eq!(before["actual_minutes"], 480);
    sqlx::query(
        "UPDATE resolved_workdays SET locked_at = NOW()
         WHERE user_id = $1 AND work_date BETWEEN '2026-07-01' AND '2026-07-31'",
    )
    .bind(employee.id.to_string())
    .execute(&pool)
    .await
    .expect("lock month");

    let employee_token = create_test_token(employee.id, employee.role.clone());
    let create_payload = json!({
        "date": date(1).to_string(),
        "clock_out_time": (at(1, 18, 0) + Duration::hours(1))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string(),
        "breaks": [{
            "break_start_time": at(1, 12, 0).format("%Y-%m-%dT%H:%M:%S").to_string(),
            "break_end_time": at(1, 13, 0).format("%Y-%m-%dT%H:%M:%S").to_string()
        }],
        "reason": "清算残高のHTTP統合テスト"
    });
    let create_response = correction_user_router(pool.clone(), employee.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attendance-corrections")
                .header("Authorization", format!("Bearer {employee_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(create_payload.to_string()))
                .expect("create request"),
        )
        .await
        .expect("create response");
    assert_eq!(create_response.status(), StatusCode::OK);
    let created = json_body(create_response).await;
    assert_eq!(created["status"], "pending");
    let request_id = created["id"].as_str().expect("request id");

    let manager_token = create_test_token(manager.id, manager.role.clone());
    let approve_response = correction_admin_router(pool.clone(), manager)
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/admin/attendance-corrections/{request_id}/approve"
                ))
                .header("Authorization", format!("Bearer {manager_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"comment": "承認"}).to_string()))
                .expect("approve request"),
        )
        .await
        .expect("approve response");
    assert_eq!(approve_response.status(), StatusCode::OK);

    let (_, after) = self_balance(pool, employee).await;
    assert_eq!(after["actual_minutes"], 540);
    assert_eq!(after["balance_minutes"], -9060);
    assert_eq!(after["days"][0]["locked"], true);
}

#[tokio::test]
async fn boundary_before_month_start_stays_with_previous_work_date() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    seed_flex_work_schedule_for_user(&pool, employee.id).await;
    let previous_work_date = NaiveDate::from_ymd_opt(2026, 6, 30).expect("previous work date");
    let july_first = date(1);
    insert_additional_attendance(
        &pool,
        &employee,
        previous_work_date,
        july_first.and_hms_opt(2, 0, 0).expect("clock in"),
        Some(july_first.and_hms_opt(4, 0, 0).expect("clock out")),
    )
    .await;

    let (status, june) = get_json(
        router(pool.clone(), employee.clone(), false),
        "/api/work-schedules/me/settlement-balance?year=2026&month=6",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(june["actual_minutes"], 120);
    assert_eq!(june["days"][29]["work_date"], "2026-06-30");

    let (status, july) = self_balance(pool, employee).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(july["actual_minutes"], 0);
}
