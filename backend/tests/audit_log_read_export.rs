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
use std::sync::OnceLock;
use timekeeper_backend::{
    handlers::admin::{
        export_audit_logs, get_audit_log_detail, list_audit_logs, AuditLogListResponse,
    },
    models::{audit_log::AuditLog, user::UserRole},
    repositories::{audit_log, permissions},
    state::AppState,
    types::{AuditLogId, UserId},
};
use tokio::sync::Mutex;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn reset_audit_logs(pool: &PgPool) {
    sqlx::query("TRUNCATE audit_logs")
        .execute(pool)
        .await
        .expect("truncate audit_logs");
}

fn build_log(
    actor_id: Option<UserId>,
    event_type: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
    result: &str,
    occurred_at: DateTime<Utc>,
) -> AuditLog {
    AuditLog {
        id: AuditLogId::new(),
        occurred_at,
        actor_id,
        actor_type: actor_id
            .map(|_| "user".to_string())
            .unwrap_or_else(|| "anonymous".to_string()),
        event_type: event_type.to_string(),
        target_type: target_type.map(|v| v.to_string()),
        target_id: target_id.map(|v| v.to_string()),
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
async fn audit_log_list_requires_permission() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = AppState::new(pool.clone(), None, None, None, config);

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
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = AppState::new(pool.clone(), None, None, None, config);

    let permissioned_user = support::seed_user(&pool, UserRole::Admin, false).await;
    let permissioned_user_id = permissioned_user.id.to_string();
    support::grant_permission(&pool, &permissioned_user_id, permissions::AUDIT_LOG_READ).await;
    assert!(permissions::user_has_permission(
        &pool,
        &permissioned_user_id,
        permissions::AUDIT_LOG_READ
    )
    .await
    .expect("check permission"));
    let actor_one = support::seed_user(&pool, UserRole::Employee, false).await;
    let actor_two = support::seed_user(&pool, UserRole::Employee, false).await;

    let now = Utc::now();
    let log_one = build_log(
        Some(actor_one.id),
        "attendance_clock_in",
        Some("attendance"),
        Some("target-1"),
        "success",
        now - Duration::minutes(10),
    );
    let log_two = build_log(
        Some(actor_one.id),
        "request_update",
        Some("request"),
        Some("target-2"),
        "success",
        now - Duration::minutes(5),
    );
    let log_three = build_log(
        Some(actor_two.id),
        "attendance_clock_out",
        Some("attendance"),
        Some("target-3"),
        "failure",
        now - Duration::minutes(1),
    );

    for log in [&log_one, &log_two, &log_three] {
        audit_log::insert_audit_log(&pool, log)
            .await
            .expect("insert audit log");
    }

    let app = Router::new()
        .route("/api/admin/audit-logs", get(list_audit_logs))
        .layer(Extension(permissioned_user.clone()))
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
    assert_eq!(payload.items[0].id, log_one.id.to_string());

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
    assert_eq!(payload.items[0].id, log_two.id.to_string());
}

#[tokio::test]
async fn audit_log_detail_returns_log() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = AppState::new(pool.clone(), None, None, None, config);

    let permissioned_user = support::seed_user(&pool, UserRole::Admin, false).await;
    let permissioned_user_id = permissioned_user.id.to_string();
    support::grant_permission(&pool, &permissioned_user_id, permissions::AUDIT_LOG_READ).await;
    assert!(permissions::user_has_permission(
        &pool,
        &permissioned_user_id,
        permissions::AUDIT_LOG_READ
    )
    .await
    .expect("check permission"));
    let log = build_log(
        Some(permissioned_user.id),
        "attendance_clock_in",
        Some("attendance"),
        Some("target-1"),
        "success",
        Utc::now(),
    );
    audit_log::insert_audit_log(&pool, &log)
        .await
        .expect("insert audit log");

    let app = Router::new()
        .route("/api/admin/audit-logs/{id}", get(get_audit_log_detail))
        .layer(Extension(permissioned_user))
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
        Some(log.id.to_string()).as_deref()
    );
}

#[tokio::test]
async fn audit_log_list_masks_pii_for_non_system_admin() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let state = AppState::new(pool.clone(), None, None, None, support::test_config());

    let admin = support::seed_user(&pool, UserRole::Admin, false).await;
    support::grant_permission(&pool, &admin.id.to_string(), permissions::AUDIT_LOG_READ).await;

    let log = AuditLog {
        id: AuditLogId::new(),
        occurred_at: Utc::now(),
        actor_id: Some(admin.id),
        actor_type: "user".to_string(),
        event_type: "admin_user_create".to_string(),
        target_type: Some("user".to_string()),
        target_id: Some("u-1".to_string()),
        result: "success".to_string(),
        error_code: None,
        metadata: Some(Json(json!({
            "email": "alice@example.com",
            "full_name": "Alice Example"
        }))),
        ip: Some("192.168.10.25".to_string()),
        user_agent: Some("Mozilla/5.0 Test Agent".to_string()),
        request_id: Some(Uuid::new_v4().to_string()),
    };
    audit_log::insert_audit_log(&pool, &log)
        .await
        .expect("insert log");

    let app = Router::new()
        .route("/api/admin/audit-logs", get(list_audit_logs))
        .layer(Extension(admin))
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
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("x-pii-masked")
            .and_then(|v| v.to_str().ok()),
        Some("true")
    );
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: AuditLogListResponse = serde_json::from_slice(&body).expect("parse body");
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].ip.as_deref(), Some("192.168.10.0/24"));
    assert_eq!(
        payload.items[0]
            .metadata
            .as_ref()
            .and_then(|v| v.get("email")),
        Some(&json!("a***@e***.com"))
    );
}

#[tokio::test]
async fn audit_log_export_returns_json_file() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = AppState::new(pool.clone(), None, None, None, config);

    let permissioned_user = support::seed_user(&pool, UserRole::Admin, false).await;
    let permissioned_user_id = permissioned_user.id.to_string();
    support::grant_permission(&pool, &permissioned_user_id, permissions::AUDIT_LOG_READ).await;
    assert!(permissions::user_has_permission(
        &pool,
        &permissioned_user_id,
        permissions::AUDIT_LOG_READ
    )
    .await
    .expect("check permission"));
    let log = build_log(
        Some(permissioned_user.id),
        "request_update",
        Some("request"),
        Some("target-2"),
        "success",
        Utc::now(),
    );
    audit_log::insert_audit_log(&pool, &log)
        .await
        .expect("insert audit log");

    let app = Router::new()
        .route("/api/admin/audit-logs/export", get(export_audit_logs))
        .layer(Extension(permissioned_user))
        .with_state(state);

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/admin/audit-logs/export?from={}&to={}&event_type=request_update",
                    today, today
                ))
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
    assert_eq!(
        response
            .headers()
            .get("x-truncated")
            .and_then(|value| value.to_str().ok()),
        Some("false")
    );

    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let payload: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse response");
    assert_eq!(payload.len(), 1);
    assert_eq!(
        payload[0].get("id").and_then(|value| value.as_str()),
        Some(log.id.to_string()).as_deref()
    );
}

#[tokio::test]
async fn audit_log_export_requires_from_and_to() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = AppState::new(pool.clone(), None, None, None, config);

    let permissioned_user = support::seed_user(&pool, UserRole::Admin, false).await;
    let permissioned_user_id = permissioned_user.id.to_string();
    support::grant_permission(&pool, &permissioned_user_id, permissions::AUDIT_LOG_READ).await;

    let app = Router::new()
        .route("/api/admin/audit-logs/export", get(export_audit_logs))
        .layer(Extension(permissioned_user))
        .with_state(state);

    // given: no from/to parameters
    // when: export is called
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/audit-logs/export")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    // then: 400 Bad Request is returned
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn audit_log_export_rejects_period_exceeding_31_days() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = AppState::new(pool.clone(), None, None, None, config);

    let permissioned_user = support::seed_user(&pool, UserRole::Admin, false).await;
    let permissioned_user_id = permissioned_user.id.to_string();
    support::grant_permission(&pool, &permissioned_user_id, permissions::AUDIT_LOG_READ).await;

    let app = Router::new()
        .route("/api/admin/audit-logs/export", get(export_audit_logs))
        .layer(Extension(permissioned_user))
        .with_state(state);

    // given: period of 32 days (exceeds 31-day limit)
    let from_date = (Utc::now() - Duration::days(32))
        .format("%Y-%m-%d")
        .to_string();
    let to_date = Utc::now().format("%Y-%m-%d").to_string();

    // when: export is called
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/admin/audit-logs/export?from={}&to={}",
                    from_date, to_date
                ))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    // then: 400 Bad Request is returned with period limit message
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 64)
        .await
        .expect("read body");
    let error_response: serde_json::Value = serde_json::from_slice(&body).expect("parse response");
    let error_message = error_response
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(error_message.contains("31"));
}

#[tokio::test]
async fn audit_log_export_allows_exactly_31_days() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = AppState::new(pool.clone(), None, None, None, config);

    let permissioned_user = support::seed_user(&pool, UserRole::Admin, false).await;
    let permissioned_user_id = permissioned_user.id.to_string();
    support::grant_permission(&pool, &permissioned_user_id, permissions::AUDIT_LOG_READ).await;

    let app = Router::new()
        .route("/api/admin/audit-logs/export", get(export_audit_logs))
        .layer(Extension(permissioned_user))
        .with_state(state);

    // given: period of 31 calendar days (from to to inclusive, within limit)
    let from_date = (Utc::now() - Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let to_date = Utc::now().format("%Y-%m-%d").to_string();

    // when: export is called
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/admin/audit-logs/export?from={}&to={}",
                    from_date, to_date
                ))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");

    // then: 200 OK is returned
    assert_eq!(response.status(), StatusCode::OK);
}
