use axum::{
    body::{to_bytes, Body},
    http::Request,
    routing::get,
    Extension, Router,
};
use chrono::{NaiveDate, Utc};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::OnceLock;
use timekeeper_backend::{
    handlers::admin::export_data,
    models::{attendance::Attendance, user::UserRole},
    repositories::{attendance::AttendanceRepository, AttendanceRepositoryTrait},
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
async fn admin_export_includes_date_strings() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_attendance_tables(&pool).await;

    let admin = support::seed_user(&pool, UserRole::Admin, false).await;
    let employee = support::seed_user(&pool, UserRole::Employee, false).await;

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

    assert!(csv_data.contains("\"2026-01-15\""));
}
