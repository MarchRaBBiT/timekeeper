use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::{admin::workday_overrides as admin_handlers, work_schedules as user_handlers},
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

fn me_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/work-schedules/me",
            get(user_handlers::get_my_workdays),
        )
        .layer(Extension(user))
        .with_state(state)
}

fn admin_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/users/{user_id}/resolved-workdays",
            get(admin_handlers::get_user_resolved_workdays),
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
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, json)
}

#[allow(clippy::too_many_arguments)]
async fn insert_resolved_workday(
    pool: &PgPool,
    user_id: UserId,
    work_date: NaiveDate,
    schedule_id: Uuid,
    version_id: Uuid,
    locked_at: Option<DateTime<Utc>>,
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
    .bind(locked_at)
    .execute(pool)
    .await
    .expect("insert resolved workday");
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}

#[tokio::test]
async fn employee_reads_own_resolved_workdays_in_range() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let (schedule_id, version_id) =
        seed_work_schedule_for_user(&pool, employee.id, "non_working").await;
    insert_resolved_workday(
        &pool,
        employee.id,
        date(2026, 7, 1),
        schedule_id,
        version_id,
        None,
    )
    .await;
    insert_resolved_workday(
        &pool,
        employee.id,
        date(2026, 7, 5),
        schedule_id,
        version_id,
        None,
    )
    .await;
    // 範囲外の勤務日は返らない
    insert_resolved_workday(
        &pool,
        employee.id,
        date(2026, 8, 1),
        schedule_id,
        version_id,
        None,
    )
    .await;

    let (status, body) = get_json(
        me_router(pool.clone(), employee.clone()),
        "/api/work-schedules/me?from=2026-07-01&to=2026-07-31",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 31);
    assert_eq!(items[0]["work_date"], "2026-07-01");
    assert_eq!(items[4]["work_date"], "2026-07-05");
    assert_eq!(items[0]["day_kind"], "scheduled_workday");
    assert_eq!(items[0]["source"], "user");
}

#[tokio::test]
async fn employee_read_materializes_missing_resolved_workdays() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let (_schedule_id, _version_id) =
        seed_work_schedule_for_user(&pool, employee.id, "non_working").await;

    let (status, body) = get_json(
        me_router(pool.clone(), employee.clone()),
        "/api/work-schedules/me?from=2026-07-01&to=2026-07-31",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 31);
    assert_eq!(items[0]["work_date"], "2026-07-01");
    assert_eq!(items[0]["day_kind"], "scheduled_workday");
}

#[tokio::test]
async fn me_rejects_reversed_range() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let (status, body) = get_json(
        me_router(pool.clone(), employee),
        "/api/work-schedules/me?from=2026-07-31&to=2026-07-01",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_WORK_SCHEDULE");
}

#[tokio::test]
async fn system_admin_reads_any_user_resolved_workdays() {
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
    insert_resolved_workday(
        &pool,
        employee.id,
        date(2026, 7, 2),
        schedule_id,
        version_id,
        None,
    )
    .await;

    let uri = format!(
        "/api/admin/users/{}/resolved-workdays?from=2026-07-01&to=2026-07-31",
        employee.id
    );
    let (status, body) = get_json(admin_router(pool.clone(), admin), &uri).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().expect("items").len(), 31);
    assert_eq!(
        body["items"].as_array().expect("items")[1]["work_date"],
        "2026-07-02"
    );
}

#[tokio::test]
async fn manager_without_scope_is_forbidden() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let employee = seed_user(&pool, UserRole::Employee, false).await;

    let uri = format!(
        "/api/admin/users/{}/resolved-workdays?from=2026-07-01&to=2026-07-31",
        employee.id
    );
    let (status, _body) = get_json(admin_router(pool.clone(), manager), &uri).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}
