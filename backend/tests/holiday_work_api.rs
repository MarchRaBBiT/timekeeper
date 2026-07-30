use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    routing::{delete, get, post},
    Extension, Router,
};
use chrono::{NaiveDate, NaiveTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::holiday_work,
    models::user::{User, UserRole},
    state::AppState,
};
use tower::ServiceExt;
use uuid::Uuid;

mod support;
use support::{integration_guard, seed_user, seed_work_schedule_for_user, test_config, test_pool};

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, day).expect("date")
}

fn router(pool: PgPool, actor: User) -> Router {
    Router::new()
        .route("/api/holiday-work-requests", post(holiday_work::submit))
        .route(
            "/api/holiday-work-requests/me",
            get(holiday_work::list_mine),
        )
        .route(
            "/api/holiday-work-requests/{id}",
            delete(holiday_work::cancel),
        )
        .route(
            "/api/admin/holiday-work-requests",
            get(holiday_work::list_pending),
        )
        .route(
            "/api/admin/holiday-work-requests/{id}/approve",
            post(holiday_work::approve),
        )
        .route(
            "/api/admin/holiday-work-requests/{id}/reject",
            post(holiday_work::reject),
        )
        .layer(Extension(actor))
        .with_state(AppState::new(pool, None, None, None, test_config()))
}

async fn call(
    app: Router,
    method: Method,
    uri: &str,
    payload: Option<Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
    let (builder, body) = match payload {
        Some(value) => (
            builder.header("content-type", "application/json"),
            Body::from(value.to_string()),
        ),
        None => (builder, Body::empty()),
    };
    let response = app
        .oneshot(builder.body(body).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json")
    };
    (status, value)
}

async fn assign_manager(pool: &PgPool, manager: &User, employee: &User) {
    let department = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO departments (id,name) VALUES ($1,$2)")
        .bind(&department)
        .bind(format!("D-{department}"))
        .execute(pool)
        .await
        .expect("department");
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(&department)
        .bind(employee.id.to_string())
        .execute(pool)
        .await
        .expect("employee department");
    sqlx::query("INSERT INTO department_managers (department_id,user_id) VALUES ($1,$2)")
        .bind(&department)
        .bind(manager.id.to_string())
        .execute(pool)
        .await
        .expect("manager scope");
}

async fn move_employee_to_managed_child_department(pool: &PgPool, manager: &User, employee: &User) {
    let parent: String = sqlx::query_scalar(
        "SELECT department_id FROM department_managers WHERE user_id = $1 LIMIT 1",
    )
    .bind(manager.id.to_string())
    .fetch_one(pool)
    .await
    .expect("manager root");
    let child = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO departments (id,name,parent_id) VALUES ($1,$2,$3)")
        .bind(&child)
        .bind(format!("Child-{child}"))
        .bind(parent)
        .execute(pool)
        .await
        .expect("child department");
    sqlx::query("UPDATE users SET department_id = $1 WHERE id = $2")
        .bind(child)
        .bind(employee.id.to_string())
        .execute(pool)
        .await
        .expect("employee child department");
}

async fn resolved(
    pool: &PgPool,
    user: &User,
    schedule: Uuid,
    version: Uuid,
    work_date: NaiveDate,
    kind: &str,
    locked: bool,
) {
    sqlx::query(
        "INSERT INTO resolved_workdays
         (id,user_id,work_date,work_schedule_id,work_schedule_version_id,source,source_id,
          day_kind,timezone,workday_boundary,expected_work_minutes,resolved_at,locked_at)
         VALUES ($1,$2,$3,$4,$5,'user',$6,$7,'Asia/Tokyo',$8,480,NOW(),$9)",
    )
    .bind(Uuid::new_v4())
    .bind(user.id.to_string())
    .bind(work_date)
    .bind(schedule)
    .bind(version)
    .bind(Uuid::new_v4())
    .bind(kind)
    .bind(NaiveTime::from_hms_opt(5, 0, 0).expect("time"))
    .bind(locked.then(Utc::now))
    .execute(pool)
    .await
    .expect("resolved");
}

#[tokio::test]
async fn substitution_submit_list_approve_and_approved_cancel_via_http() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    assign_manager(&pool, &manager, &employee).await;
    move_employee_to_managed_child_department(&pool, &manager, &employee).await;
    let (schedule, version) = seed_work_schedule_for_user(&pool, employee.id, "non_working").await;
    resolved(
        &pool,
        &employee,
        schedule,
        version,
        date(6),
        "scheduled_non_working_day",
        false,
    )
    .await;
    resolved(
        &pool,
        &employee,
        schedule,
        version,
        date(7),
        "scheduled_workday",
        false,
    )
    .await;

    let (status, submitted) = call(
        router(pool.clone(), employee.clone()),
        Method::POST,
        "/api/holiday-work-requests",
        Some(json!({
            "work_date": date(6),
            "benefit": "substitution",
            "substitute_date": date(7),
            "reason": "swap"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = submitted["id"].as_str().expect("id");

    let (status, mine) = call(
        router(pool.clone(), employee.clone()),
        Method::GET,
        "/api/holiday-work-requests/me?status=pending",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mine.as_array().expect("array").len(), 1);

    let (status, pending) = call(
        router(pool.clone(), manager.clone()),
        Method::GET,
        "/api/admin/holiday-work-requests",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(pending.as_array().expect("array").len(), 1);

    let (status, approved) = call(
        router(pool.clone(), manager),
        Method::POST,
        &format!("/api/admin/holiday-work-requests/{id}/approve"),
        Some(json!({"comment":"ok"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(approved["status"], "approved");

    let (status, cancelled) = call(
        router(pool.clone(), employee),
        Method::DELETE,
        &format!("/api/holiday-work-requests/{id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled["status"], "cancelled");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workday_overrides")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn compensatory_approve_cancel_and_reject_via_http() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    assign_manager(&pool, &manager, &employee).await;
    sqlx::query(
        "INSERT INTO compensatory_leave_settings
         (id,effective_from,expiry_months,created_by) VALUES ($1,$2,2,$3)",
    )
    .bind(Uuid::new_v4())
    .bind(date(1))
    .bind(manager.id.to_string())
    .execute(&pool)
    .await
    .expect("setting");

    let (status, submitted) = call(
        router(pool.clone(), employee.clone()),
        Method::POST,
        "/api/holiday-work-requests",
        Some(json!({
            "work_date": date(13),
            "benefit": "compensatory",
            "compensatory_minutes": 480,
            "reason": "comp"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = submitted["id"].as_str().expect("id");
    let (status, _) = call(
        router(pool.clone(), manager.clone()),
        Method::POST,
        &format!("/api/admin/holiday-work-requests/{id}/approve"),
        Some(json!({"comment":"ok"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        router(pool.clone(), employee.clone()),
        Method::DELETE,
        &format!("/api/holiday-work-requests/{id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, submitted) = call(
        router(pool.clone(), employee),
        Method::POST,
        "/api/holiday-work-requests",
        Some(json!({
            "work_date": date(20),
            "benefit": "compensatory",
            "compensatory_minutes": 240,
            "reason": "reject"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = submitted["id"].as_str().expect("id");
    let (status, rejected) = call(
        router(pool, manager),
        Method::POST,
        &format!("/api/admin/holiday-work-requests/{id}/reject"),
        Some(json!({"comment":"no"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rejected["status"], "rejected");
}

#[tokio::test]
async fn authorization_and_locked_errors_are_exposed_via_http() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let scoped_manager = seed_user(&pool, UserRole::Manager, false).await;
    let outside_manager = seed_user(&pool, UserRole::Manager, false).await;
    assign_manager(&pool, &scoped_manager, &employee).await;
    let (schedule, version) = seed_work_schedule_for_user(&pool, employee.id, "non_working").await;
    resolved(
        &pool,
        &employee,
        schedule,
        version,
        date(27),
        "scheduled_non_working_day",
        false,
    )
    .await;
    resolved(
        &pool,
        &employee,
        schedule,
        version,
        date(28),
        "scheduled_workday",
        true,
    )
    .await;
    let (status, submitted) = call(
        router(pool.clone(), employee.clone()),
        Method::POST,
        "/api/holiday-work-requests",
        Some(json!({
            "work_date": date(27),
            "benefit": "substitution",
            "substitute_date": date(28),
            "reason": "locked"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = submitted["id"].as_str().expect("id");

    let (status, _) = call(
        router(pool.clone(), outside_manager),
        Method::POST,
        &format!("/api/admin/holiday-work-requests/{id}/approve"),
        Some(json!({"comment":"outside"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = call(
        router(pool.clone(), employee),
        Method::POST,
        &format!("/api/admin/holiday-work-requests/{id}/approve"),
        Some(json!({"comment":"self"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = call(
        router(pool, scoped_manager),
        Method::POST,
        &format!("/api/admin/holiday-work-requests/{id}/approve"),
        Some(json!({"comment":"locked"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "CONFLICT");
}
