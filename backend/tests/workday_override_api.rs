use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{get, put},
    Extension, Router,
};
use chrono::{NaiveDate, NaiveTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::workday_overrides as handlers,
    models::user::{User, UserRole},
    state::AppState,
    types::UserId,
};
use tower::ServiceExt;
use uuid::Uuid;

mod support;

use support::{seed_user, seed_work_schedule_for_user, test_config, test_pool};

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
            "/api/admin/users/{user_id}/workday-overrides/{date}",
            put(handlers::set_workday_override).delete(handlers::delete_workday_override),
        )
        .route(
            "/api/admin/users/{user_id}/resolved-workdays",
            get(handlers::get_user_resolved_workdays),
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

async fn insert_locked_resolved_workday(
    pool: &PgPool,
    user_id: UserId,
    work_date: NaiveDate,
    schedule_id: Uuid,
    version_id: Uuid,
) {
    sqlx::query(
        "INSERT INTO resolved_workdays \
         (id, user_id, work_date, work_schedule_id, work_schedule_version_id, source, source_id, \
          day_kind, timezone, workday_boundary, expected_work_minutes, resolved_at, locked_at) \
         VALUES ($1, $2, $3, $4, $5, 'user', $6, 'scheduled_workday', 'Asia/Tokyo', $7, 480, $8, $9)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id.to_string())
    .bind(work_date)
    .bind(schedule_id)
    .bind(version_id)
    .bind(Uuid::new_v4())
    .bind(NaiveTime::from_hms_opt(5, 0, 0).expect("boundary"))
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(pool)
    .await
    .expect("insert locked resolved workday");
}

async fn assign_manager_to_employee_department(pool: &PgPool, manager: UserId, employee: UserId) {
    let department_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&department_id)
        .bind("Engineering")
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

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 7, 15).expect("valid date")
}

#[tokio::test]
async fn system_admin_upserts_non_working_day_override() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let uri = format!(
        "/api/admin/users/{}/workday-overrides/2026-07-15",
        employee.id
    );
    let (status, body) = request_json(
        router(pool.clone(), admin),
        "PUT",
        &uri,
        Some(json!({ "kind": "non_working_day", "reason": "創立記念日" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["kind"], "non_working_day");
    assert_eq!(body["reason"], "創立記念日");
    assert!(body["work_schedule_id"].is_null());
}

#[tokio::test]
async fn use_schedule_override_requires_schedule_id() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let uri = format!(
        "/api/admin/users/{}/workday-overrides/2026-07-15",
        employee.id
    );
    let (status, body) = request_json(
        router(pool.clone(), admin),
        "PUT",
        &uri,
        Some(json!({ "kind": "use_schedule", "reason": "特別出勤" })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE");
}

#[tokio::test]
async fn override_rejected_when_resolved_workday_locked() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let (schedule_id, version_id) =
        seed_work_schedule_for_user(&pool, employee.id, "non_working").await;
    insert_locked_resolved_workday(&pool, employee.id, date(), schedule_id, version_id).await;

    let uri = format!(
        "/api/admin/users/{}/workday-overrides/2026-07-15",
        employee.id
    );
    let (status, body) = request_json(
        router(pool.clone(), admin),
        "PUT",
        &uri,
        Some(json!({ "kind": "non_working_day", "reason": "締め後変更" })),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "RESOLVED_WORKDAY_LOCKED");
}

#[tokio::test]
async fn delete_existing_then_missing_override() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let uri = format!(
        "/api/admin/users/{}/workday-overrides/2026-07-15",
        employee.id
    );

    let (created, _) = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        &uri,
        Some(json!({ "kind": "non_working_day", "reason": "臨時休業" })),
    )
    .await;
    assert_eq!(created, StatusCode::OK);

    let (deleted, _) =
        request_json(router(pool.clone(), admin.clone()), "DELETE", &uri, None).await;
    assert_eq!(deleted, StatusCode::NO_CONTENT);

    let (missing, _) = request_json(router(pool.clone(), admin), "DELETE", &uri, None).await;
    assert_eq!(missing, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn scoped_manager_can_upsert_override() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    assign_manager_to_employee_department(&pool, manager.id, employee.id).await;

    let uri = format!(
        "/api/admin/users/{}/workday-overrides/2026-07-15",
        employee.id
    );
    let (status, _body) = request_json(
        router(pool.clone(), manager),
        "PUT",
        &uri,
        Some(json!({ "kind": "non_working_day", "reason": "部署内対応" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn unscoped_manager_cannot_upsert_override() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let uri = format!(
        "/api/admin/users/{}/workday-overrides/2026-07-15",
        employee.id
    );
    let (status, _body) = request_json(
        router(pool.clone(), manager),
        "PUT",
        &uri,
        Some(json!({ "kind": "non_working_day", "reason": "権限外" })),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}
