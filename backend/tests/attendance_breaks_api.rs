use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use chrono::Utc;
use serde_json;
use sqlx::PgPool;
use std::sync::OnceLock;
use timekeeper_backend::{
    handlers::attendance,
    models::{attendance::Attendance, break_record::BreakRecord, user::UserRole},
    repositories::{
        attendance::{AttendanceRepository, AttendanceRepositoryTrait},
        break_record::BreakRecordRepository,
        repository::Repository,
    },
    state::AppState,
};
use tokio::sync::Mutex;
use tower::ServiceExt;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn reset_attendance_tables(pool: &PgPool) {
    sqlx::query("TRUNCATE break_records, attendance")
        .execute(pool)
        .await
        .expect("truncate attendance tables");
}

#[tokio::test]
async fn get_breaks_by_attendance_blocks_other_user() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_attendance_tables(&pool).await;

    let owner = support::seed_user(&pool, UserRole::Employee, false).await;
    let other = support::seed_user(&pool, UserRole::Employee, false).await;

    let now = Utc::now();
    let attendance = Attendance::new(owner.id, now.date_naive(), now);
    let repo = AttendanceRepository::new();
    let saved = repo
        .create(&pool, &attendance)
        .await
        .expect("create attendance");

    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let app = Router::new()
        .route(
            "/api/attendance/{id}/breaks",
            get(attendance::get_breaks_by_attendance),
        )
        .layer(Extension(other))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/attendance/{}/breaks", saved.id))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn get_breaks_by_attendance_allows_owner() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_attendance_tables(&pool).await;

    let owner = support::seed_user(&pool, UserRole::Employee, false).await;

    let now = Utc::now();
    let attendance = Attendance::new(owner.id, now.date_naive(), now);
    let attendance_repo = AttendanceRepository::new();
    let saved = attendance_repo
        .create(&pool, &attendance)
        .await
        .expect("create attendance");

    let break_repo = BreakRecordRepository::new();
    let break_record = BreakRecord::new(saved.id, now.naive_utc(), now);
    break_repo
        .create(&pool, &break_record)
        .await
        .expect("create break");

    let state = AppState::new(pool.clone(), None, None, None, support::test_config());
    let app = Router::new()
        .route(
            "/api/attendance/{id}/breaks",
            get(attendance::get_breaks_by_attendance),
        )
        .layer(Extension(owner))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/attendance/{}/breaks", saved.id))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let records: Vec<timekeeper_backend::models::break_record::BreakRecordResponse> =
        serde_json::from_slice(&body).expect("parse response");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].attendance_id, saved.id);
}
