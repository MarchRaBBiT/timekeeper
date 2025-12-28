use axum::{
    body::{to_bytes, Body},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        Request, StatusCode,
    },
    routing::get,
    Extension, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::{types::Json, PgPool};
use timekeeper_backend::{
    handlers::{
        admin::{export_audit_logs, get_audit_log_detail, list_audit_logs, AuditLogListResponse},
        audit_log_repo,
    },
    models::{audit_log::AuditLog, user::UserRole},
};
use tower::ServiceExt;
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

async fn reset_audit_logs(pool: &PgPool) {
    sqlx::query("TRUNCATE audit_logs")
        .execute(pool)
        .await
        .expect("truncate audit_logs");
}

fn build_log(
    actor_id: Option<String>,
    event_type: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
    result: &str,
    occurred_at: DateTime<Utc>,
) -> AuditLog {
    AuditLog {
        id: Uuid::new_v4().to_string(),
        occurred_at,
        actor_id: actor_id.clone(),
        actor_type: actor_id
            .map(|_| "user".to_string())
            .unwrap_or_else(|| "anonymous".to_string()),
        event_type: event_type.to_string(),
        target_type: target_type.map(|value| value.to_string()),
        target_id: target_id.map(|value| value.to_string()),
        result: result.to_string(),
        error_code: if result == "failure" {
            Some("http_400".to_string())
        } else {
            None
        },
        metadata: Some(Json(json!({ "note": event_type }))),
        ip: Some("127.0.0.1".to_string()),
        user_agent: Some("test-agent".to_string()),
        request_id: Some(Uuid::new_v4().to_string()),
    }
}

#[tokio::test]
async fn audit_log_list_requires_system_admin() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = (pool.clone(), config);

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let app = Router::new()
        .route("/api/admin/audit-logs", get(list_audit_logs))
        .layer(Extension(user))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/audit-logs")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn audit_log_list_filters_and_paginates() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = (pool.clone(), config);

    let system_admin = support::seed_user(&pool, UserRole::Admin, true).await;
    let actor_one = support::seed_user(&pool, UserRole::Employee, false).await;
    let actor_two = support::seed_user(&pool, UserRole::Employee, false).await;

    let now = Utc::now();
    let log_one = build_log(
        Some(actor_one.id.clone()),
        "attendance_clock_in",
        Some("attendance"),
        Some("target-1"),
        "success",
        now - Duration::minutes(10),
    );
    let log_two = build_log(
        Some(actor_one.id.clone()),
        "request_update",
        Some("request"),
        Some("target-2"),
        "success",
        now - Duration::minutes(5),
    );
    let log_three = build_log(
        Some(actor_two.id.clone()),
        "attendance_clock_out",
        Some("attendance"),
        Some("target-3"),
        "failure",
        now - Duration::minutes(1),
    );

    for log in [&log_one, &log_two, &log_three] {
        audit_log_repo::insert_audit_log(&pool, log)
            .await
            .expect("insert audit log");
    }

    let app = Router::new()
        .route("/api/admin/audit-logs", get(list_audit_logs))
        .layer(Extension(system_admin.clone()))
        .with_state(state.clone());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/admin/audit-logs?actor_id={}&result=success&per_page=1&page=2",
                    actor_one.id
                ))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: AuditLogListResponse = serde_json::from_slice(&body).expect("parse response");
    assert_eq!(payload.total, 2);
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].id, log_one.id);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/audit-logs?target_id=target-2")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: AuditLogListResponse = serde_json::from_slice(&body).expect("parse response");
    assert_eq!(payload.total, 1);
    assert_eq!(payload.items[0].id, log_two.id);
}

#[tokio::test]
async fn audit_log_detail_returns_log() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = (pool.clone(), config);

    let system_admin = support::seed_user(&pool, UserRole::Admin, true).await;
    let log = build_log(
        Some(system_admin.id.clone()),
        "attendance_clock_in",
        Some("attendance"),
        Some("target-1"),
        "success",
        Utc::now(),
    );
    audit_log_repo::insert_audit_log(&pool, &log)
        .await
        .expect("insert audit log");

    let app = Router::new()
        .route("/api/admin/audit-logs/:id", get(get_audit_log_detail))
        .layer(Extension(system_admin))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/admin/audit-logs/{}", log.id))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("parse response");
    assert_eq!(
        payload.get("id").and_then(|v| v.as_str()),
        Some(log.id.as_str())
    );
}

#[tokio::test]
async fn audit_log_export_returns_json_file() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = (pool.clone(), config);

    let system_admin = support::seed_user(&pool, UserRole::Admin, true).await;
    let log = build_log(
        Some(system_admin.id.clone()),
        "request_update",
        Some("request"),
        Some("target-2"),
        "success",
        Utc::now(),
    );
    audit_log_repo::insert_audit_log(&pool, &log)
        .await
        .expect("insert audit log");

    let app = Router::new()
        .route("/api/admin/audit-logs/export", get(export_audit_logs))
        .layer(Extension(system_admin))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/audit-logs/export?event_type=request_update")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    let content_disposition = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(content_disposition.starts_with("attachment; filename=\"audit_logs_"));

    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse response");
    assert_eq!(payload.len(), 1);
    assert_eq!(
        payload[0].get("id").and_then(|value| value.as_str()),
        Some(log.id.as_str())
    );
}
