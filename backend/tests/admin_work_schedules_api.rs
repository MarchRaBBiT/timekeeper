use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{delete, get, post},
    Extension, Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::work_schedules as handlers,
    models::user::{User, UserRole},
    state::AppState,
};
use tower::ServiceExt;
use uuid::Uuid;

mod support;

use support::{seed_user, test_config, test_pool};

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
            "/api/admin/work-schedules",
            get(handlers::list_work_schedules).post(handlers::create_work_schedule),
        )
        .route(
            "/api/admin/work-schedules/{id}",
            get(handlers::get_work_schedule).patch(handlers::update_work_schedule),
        )
        .route(
            "/api/admin/work-schedules/{id}/retire",
            post(handlers::retire_work_schedule),
        )
        .route(
            "/api/admin/work-schedules/{id}/versions",
            post(handlers::create_work_schedule_version),
        )
        .route(
            "/api/admin/work-schedules/{id}/versions/{version_id}",
            get(handlers::get_work_schedule_version)
                .put(handlers::replace_work_schedule_version)
                .delete(handlers::delete_work_schedule_version),
        )
        .route(
            "/api/admin/work-schedules/{id}/versions/{version_id}/publish",
            post(handlers::publish_work_schedule_version),
        )
        .route(
            "/api/admin/work-schedule-assignments",
            get(handlers::list_work_schedule_assignments)
                .post(handlers::create_work_schedule_assignment),
        )
        .route(
            "/api/admin/work-schedule-assignments/{id}",
            delete(handlers::delete_work_schedule_assignment),
        )
        .layer(Extension(user))
        .with_state(state)
}

fn master_payload(code: &str) -> Value {
    json!({
        "code": code,
        "name": "Standard schedule",
        "description": "Weekday fixed schedule"
    })
}

fn version_payload() -> Value {
    let days: Vec<Value> = (1..=7)
        .map(|weekday| {
            if weekday <= 5 {
                json!({
                    "weekday": weekday,
                    "day_kind": "working_day",
                    "work_intervals": [{
                        "start_time": "09:00:00",
                        "start_day_offset": 0,
                        "end_time": "18:00:00",
                        "end_day_offset": 0
                    }],
                    "planned_breaks": [{
                        "start_time": "12:00:00",
                        "start_day_offset": 0,
                        "end_time": "13:00:00",
                        "end_day_offset": 0
                    }]
                })
            } else {
                json!({
                    "weekday": weekday,
                    "day_kind": "non_working_day",
                    "work_intervals": [],
                    "planned_breaks": []
                })
            }
        })
        .collect();

    json!({
        "effective_from": "2026-07-01",
        "effective_until": null,
        "timezone": "Asia/Tokyo",
        "workday_boundary": "05:00:00",
        "public_holiday_policy": "non_working",
        "late_grace_minutes": 0,
        "early_leave_grace_minutes": 0,
        "days": days
    })
}

async fn request_json(
    app: Router,
    method: &str,
    uri: &str,
    payload: Option<Value>,
) -> (StatusCode, Value, Option<String>) {
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
    let location = response
        .headers()
        .get("Location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, json, location)
}

async fn create_master(pool: &PgPool, user: &User, code: &str) -> String {
    let (status, body, location) = request_json(
        router(pool.clone(), user.clone()),
        "POST",
        "/api/admin/work-schedules",
        Some(master_payload(code)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let schedule_id = body["id"].as_str().expect("schedule id").to_string();
    let expected_location = format!("/api/admin/work-schedules/{schedule_id}");
    assert_eq!(location.as_deref(), Some(expected_location.as_str()));
    schedule_id
}

#[tokio::test]
async fn system_admin_creates_master_and_manager_can_list_it() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let code = format!("standard-{}", Uuid::new_v4());

    let schedule_id = create_master(&pool, &admin, &code).await;

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let (status, body, _) = request_json(
        router(pool.clone(), manager),
        "GET",
        &format!("/api/admin/work-schedules?q={code}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["id"], schedule_id);
}

#[tokio::test]
async fn employee_cannot_read_and_manager_cannot_mutate_master() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let employee = seed_user(&pool, UserRole::Employee, false).await;
    let (status, _, _) = request_json(
        router(pool.clone(), employee),
        "GET",
        "/api/admin/work-schedules",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let (status, _, _) = request_json(
        router(pool, manager),
        "POST",
        "/api/admin/work-schedules",
        Some(master_payload("forbidden")),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn duplicate_master_code_returns_specific_conflict() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let code = format!("duplicate-{}", Uuid::new_v4());
    create_master(&pool, &admin, &code).await;

    let (status, body, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedules",
        Some(master_payload(&code)),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "WORK_SCHEDULE_CODE_CONFLICT");
}

#[tokio::test]
async fn system_admin_updates_retires_and_cannot_version_retired_master() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let schedule_id = create_master(&pool, &admin, &format!("retired-{}", Uuid::new_v4())).await;

    let (status, updated, _) = request_json(
        router(pool.clone(), admin.clone()),
        "PATCH",
        &format!("/api/admin/work-schedules/{schedule_id}"),
        Some(json!({ "name": " Night shift ", "description": "" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["name"], "Night shift");
    assert!(updated["description"].is_null());

    let (status, retired, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/retire"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(retired["status"], "retired");

    let (status, body, _) = request_json(
        router(pool, admin),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/versions"),
        Some(version_payload()),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "WORK_SCHEDULE_RETIRED");
}

#[tokio::test]
async fn invalid_weekly_definition_is_rejected_before_persistence() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let schedule_id = create_master(&pool, &admin, &format!("invalid-{}", Uuid::new_v4())).await;
    let mut invalid = version_payload();
    invalid["days"].as_array_mut().expect("days").pop();

    let (status, body, _) = request_json(
        router(pool, admin),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/versions"),
        Some(invalid),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "INVALID_SCHEDULE_INTERVALS");
}

#[tokio::test]
async fn published_version_is_readable_and_immutable() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let schedule_id = create_master(&pool, &admin, &format!("versioned-{}", Uuid::new_v4())).await;

    let (status, version, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/versions"),
        Some(version_payload()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(version["status"], "draft");
    assert_eq!(version["days"].as_array().expect("days").len(), 7);
    assert_eq!(version["days"][0]["expected_work_minutes"], 480);
    let version_id = version["id"].as_str().expect("version id");

    let (status, published, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/versions/{version_id}/publish"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(published["status"], "published");

    let mut replacement = version_payload();
    replacement["revision"] = json!(1);
    let (status, body, _) = request_json(
        router(pool, admin),
        "PUT",
        &format!("/api/admin/work-schedules/{schedule_id}/versions/{version_id}"),
        Some(replacement),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "PUBLISHED_VERSION_IMMUTABLE");
}

#[tokio::test]
async fn draft_replace_uses_revision_and_draft_can_be_deleted() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let schedule_id = create_master(&pool, &admin, &format!("draft-{}", Uuid::new_v4())).await;
    let (_, version, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/versions"),
        Some(version_payload()),
    )
    .await;
    let version_id = version["id"].as_str().expect("version id");

    let mut stale = version_payload();
    stale["revision"] = json!(99);
    let (status, body, _) = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        &format!("/api/admin/work-schedules/{schedule_id}/versions/{version_id}"),
        Some(stale),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "REVISION_CONFLICT");

    let mut replacement = version_payload();
    replacement["revision"] = json!(1);
    replacement["late_grace_minutes"] = json!(5);
    let (status, replaced, _) = request_json(
        router(pool.clone(), admin.clone()),
        "PUT",
        &format!("/api/admin/work-schedules/{schedule_id}/versions/{version_id}"),
        Some(replacement),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replaced["revision"], 2);
    assert_eq!(replaced["late_grace_minutes"], 5);

    let (status, _, _) = request_json(
        router(pool.clone(), admin.clone()),
        "DELETE",
        &format!("/api/admin/work-schedules/{schedule_id}/versions/{version_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _, _) = request_json(
        router(pool, admin),
        "GET",
        &format!("/api/admin/work-schedules/{schedule_id}/versions/{version_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn publishing_overlapping_versions_returns_specific_conflict() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let schedule_id = create_master(&pool, &admin, &format!("overlap-{}", Uuid::new_v4())).await;
    let (_, first, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/versions"),
        Some(version_payload()),
    )
    .await;
    let (_, second, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/versions"),
        Some(version_payload()),
    )
    .await;
    let first_id = first["id"].as_str().expect("first id");
    let second_id = second["id"].as_str().expect("second id");
    let (status, _, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/versions/{first_id}/publish"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body, _) = request_json(
        router(pool, admin),
        "POST",
        &format!("/api/admin/work-schedules/{schedule_id}/versions/{second_id}/publish"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "EFFECTIVE_PERIOD_OVERLAP");
}

#[tokio::test]
async fn assignment_rejects_overlapping_period_for_same_target() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let admin = seed_user(&pool, UserRole::Manager, true).await;
    let schedule_id = create_master(&pool, &admin, &format!("assigned-{}", Uuid::new_v4())).await;
    let payload = json!({
        "work_schedule_id": schedule_id,
        "target": { "type": "organization" },
        "valid_from": "2026-07-01",
        "valid_until": null
    });

    let (status, assignment, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-assignments",
        Some(payload.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(assignment["target"]["type"], "organization");
    let assignment_id = assignment["id"].as_str().expect("assignment id");

    let (status, listed, _) = request_json(
        router(pool.clone(), admin.clone()),
        "GET",
        "/api/admin/work-schedule-assignments",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(listed["total"].as_i64().expect("total") >= 1);

    let (status, body, _) = request_json(
        router(pool.clone(), admin.clone()),
        "POST",
        "/api/admin/work-schedule-assignments",
        Some(payload),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "EFFECTIVE_PERIOD_OVERLAP");

    let (status, _, _) = request_json(
        router(pool, admin),
        "DELETE",
        &format!("/api/admin/work-schedule-assignments/{assignment_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}
