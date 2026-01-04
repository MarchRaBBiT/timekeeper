use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware as axum_middleware,
    response::IntoResponse,
    routing::post,
    Extension, Router,
};
use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use timekeeper_backend::{
    models::{audit_log::AuditLog, user::UserRole},
    services::audit_log::{AuditLogService, AuditLogServiceTrait},
};
use tokio::time::{sleep, Duration};
use tower::ServiceExt;
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

async fn ok_handler() -> impl IntoResponse {
    StatusCode::OK
}

async fn fetch_audit_log_by_request_id(pool: &PgPool, request_id: &str) -> Option<AuditLog> {
    sqlx::query_as::<_, AuditLog>(
        "SELECT id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result, \
         error_code, metadata, ip, user_agent, request_id \
         FROM audit_logs WHERE request_id = $1 ORDER BY occurred_at DESC LIMIT 1",
    )
    .bind(request_id)
    .fetch_optional(pool)
    .await
    .expect("fetch audit log")
}

async fn reset_audit_logs(pool: &PgPool) {
    sqlx::query("TRUNCATE audit_logs")
        .execute(pool)
        .await
        .expect("truncate audit_logs");
}

fn metadata_value(log: &AuditLog) -> &Value {
    log.metadata
        .as_ref()
        .map(|value| &value.0)
        .expect("metadata")
}

async fn wait_for_audit_log_by_request_id(pool: &PgPool, request_id: &str) -> Option<AuditLog> {
    for _ in 0..100 {
        if let Some(log) = fetch_audit_log_by_request_id(pool, request_id).await {
            return Some(log);
        }
        sleep(Duration::from_millis(50)).await;
    }
    None
}

#[tokio::test]
async fn audit_log_middleware_records_event() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let mut config = support::test_config();
    config.audit_log_retention_days = 365;
    config.audit_log_retention_forever = false;
    let state = (pool.clone(), config.clone());

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let audit_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool.clone()));

    let app = Router::new()
        .route("/api/attendance/clock-in", post(ok_handler))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timekeeper_backend::middleware::audit_log,
        ))
        .layer(Extension(user))
        .layer(Extension(audit_service))
        .with_state(state);

    let request_id = Uuid::new_v4().to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attendance/clock-in")
                .header("x-client-source", "web")
                .header("x-request-id", &request_id)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");
    assert_eq!(response.status(), StatusCode::OK);

    let logged = wait_for_audit_log_by_request_id(&pool, &request_id)
        .await
        .expect("audit log");
    assert_eq!(logged.event_type, "attendance_clock_in");
    assert_eq!(logged.result, "success");
    assert_eq!(logged.actor_type, "user");
    assert!(logged.actor_id.is_some());

    let metadata = metadata_value(&logged);
    assert_eq!(
        metadata.get("clock_type").and_then(Value::as_str),
        Some("clock_in")
    );
    assert_eq!(metadata.get("source").and_then(Value::as_str), Some("web"));
    assert_eq!(
        metadata.get("timezone").and_then(Value::as_str),
        Some("Asia/Tokyo")
    );
}

#[tokio::test]
async fn audit_log_middleware_skips_when_retention_is_zero() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let mut config = support::test_config();
    config.audit_log_retention_days = 0;
    config.audit_log_retention_forever = false;
    let state = (pool.clone(), config.clone());

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let audit_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool.clone()));

    let app = Router::new()
        .route("/api/attendance/clock-in", post(ok_handler))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timekeeper_backend::middleware::audit_log,
        ))
        .layer(Extension(user))
        .layer(Extension(audit_service))
        .with_state(state);

    let request_id = Uuid::new_v4().to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attendance/clock-in")
                .header("x-request-id", &request_id)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");
    assert_eq!(response.status(), StatusCode::OK);

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE request_id = $1")
        .bind(&request_id)
        .fetch_one(&pool)
        .await
        .expect("count audit logs");
    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn audit_log_middleware_records_failure_with_error_code() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = (pool.clone(), config.clone());

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let audit_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool.clone()));

    let app = Router::new()
        .route(
            "/api/attendance/clock-in",
            post(|| async { (StatusCode::BAD_REQUEST, "bad") }),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timekeeper_backend::middleware::audit_log,
        ))
        .layer(Extension(user))
        .layer(Extension(audit_service))
        .with_state(state);

    let request_id = Uuid::new_v4().to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attendance/clock-in")
                .header("x-request-id", &request_id)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let logged = wait_for_audit_log_by_request_id(&pool, &request_id)
        .await
        .expect("audit log");
    assert_eq!(logged.event_type, "attendance_clock_in");
    assert_eq!(logged.result, "failure");
    assert_eq!(logged.error_code.as_deref(), Some("http_400"));
    assert!(logged.occurred_at <= Utc::now());

    let metadata = metadata_value(&logged);
    assert_eq!(
        metadata.get("clock_type").and_then(Value::as_str),
        Some("clock_in")
    );
}

#[tokio::test]
async fn audit_log_middleware_records_auth_failure() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let mut config = support::test_config();
    config.audit_log_retention_days = 365;
    config.audit_log_retention_forever = false;
    let state = (pool.clone(), config.clone());

    let audit_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool.clone()));

    let app = Router::new()
        .route("/api/attendance/clock-in", post(ok_handler))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timekeeper_backend::middleware::auth,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timekeeper_backend::middleware::audit_log,
        ))
        .layer(Extension(audit_service))
        .with_state(state);

    let request_id = Uuid::new_v4().to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attendance/clock-in")
                .header("x-request-id", &request_id)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call app");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let logged = wait_for_audit_log_by_request_id(&pool, &request_id)
        .await
        .expect("audit log");
    assert_eq!(logged.event_type, "attendance_clock_in");
    assert_eq!(logged.result, "failure");
    assert_eq!(logged.error_code.as_deref(), Some("http_401"));
    assert_eq!(logged.actor_type, "anonymous");
    assert!(logged.actor_id.is_none());
}

#[tokio::test]
async fn audit_log_middleware_records_request_metadata() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = (pool.clone(), config.clone());

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let audit_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool.clone()));

    let app = Router::new()
        .route("/api/requests/leave", post(ok_handler))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timekeeper_backend::middleware::audit_log,
        ))
        .layer(Extension(user))
        .layer(Extension(audit_service))
        .with_state(state);

    let request_id = Uuid::new_v4().to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/requests/leave")
                .header("content-type", "application/json")
                .header("x-request-id", &request_id)
                .body(Body::from(
                    r#"{"leave_type":"annual","start_date":"2025-01-10","end_date":"2025-01-12","reason":"test"}"#,
                ))
                .expect("build request"),
        )
        .await
        .expect("call app");
    assert_eq!(response.status(), StatusCode::OK);

    let logged = wait_for_audit_log_by_request_id(&pool, &request_id)
        .await
        .expect("audit log");
    let metadata = metadata_value(&logged);
    assert_eq!(
        metadata.get("request_type").and_then(Value::as_str),
        Some("leave")
    );
    let payload = metadata
        .get("payload_summary")
        .and_then(Value::as_object)
        .expect("payload_summary");
    assert_eq!(
        payload.get("leave_type").and_then(Value::as_str),
        Some("annual")
    );
    assert_eq!(
        payload.get("start_date").and_then(Value::as_str),
        Some("2025-01-10")
    );
    assert!(payload.get("reason").is_none());
}

#[tokio::test]
async fn audit_log_middleware_records_request_metadata_on_failure_with_large_body() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = (pool.clone(), config.clone());

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let audit_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool.clone()));

    let app = Router::new()
        .route(
            "/api/requests/leave",
            post(|| async { (StatusCode::BAD_REQUEST, "bad") }),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timekeeper_backend::middleware::audit_log,
        ))
        .layer(Extension(user))
        .layer(Extension(audit_service))
        .with_state(state);

    let request_id = Uuid::new_v4().to_string();
    let body = vec![b'a'; 64 * 1024 + 1];
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/requests/leave")
                .header("content-type", "application/json")
                .header("x-request-id", &request_id)
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("call app");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let logged = wait_for_audit_log_by_request_id(&pool, &request_id)
        .await
        .expect("audit log");
    let metadata = metadata_value(&logged);
    assert_eq!(
        metadata.get("request_type").and_then(Value::as_str),
        Some("leave")
    );
    let payload = metadata
        .get("payload_summary")
        .and_then(Value::as_object)
        .expect("payload_summary");
    assert!(payload.is_empty());
}

#[tokio::test]
async fn audit_log_middleware_records_approval_metadata() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = (pool.clone(), config.clone());

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let audit_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool.clone()));

    let app = Router::new()
        .route(
            "/api/admin/requests/:id/approve",
            axum::routing::put(ok_handler),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timekeeper_backend::middleware::audit_log,
        ))
        .layer(Extension(user))
        .layer(Extension(audit_service))
        .with_state(state);

    let request_id = Uuid::new_v4().to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/admin/requests/req-123/approve")
                .header("content-type", "application/json")
                .header("x-request-id", &request_id)
                .body(Body::from(r#"{"comment":"ok"}"#))
                .expect("build request"),
        )
        .await
        .expect("call app");
    assert_eq!(response.status(), StatusCode::OK);

    let logged = wait_for_audit_log_by_request_id(&pool, &request_id)
        .await
        .expect("audit log");
    let metadata = metadata_value(&logged);
    assert_eq!(
        metadata.get("approval_step").and_then(Value::as_str),
        Some("single")
    );
    assert_eq!(
        metadata.get("decision").and_then(Value::as_str),
        Some("approve")
    );
}

#[tokio::test]
async fn audit_log_middleware_records_password_change_metadata() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_audit_logs(&pool).await;
    let config = support::test_config();
    let state = (pool.clone(), config.clone());

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let audit_service: Arc<dyn AuditLogServiceTrait> = Arc::new(AuditLogService::new(pool.clone()));

    let app = Router::new()
        .route("/api/auth/change-password", axum::routing::put(ok_handler))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timekeeper_backend::middleware::audit_log,
        ))
        .layer(Extension(user))
        .layer(Extension(audit_service))
        .with_state(state);

    let request_id = Uuid::new_v4().to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/auth/change-password")
                .header("content-type", "application/json")
                .header("x-request-id", &request_id)
                .body(Body::from(
                    r#"{"current_password":"old","new_password":"newpass123"}"#,
                ))
                .expect("build request"),
        )
        .await
        .expect("call app");
    assert_eq!(response.status(), StatusCode::OK);

    let logged = wait_for_audit_log_by_request_id(&pool, &request_id)
        .await
        .expect("audit log");
    let metadata = metadata_value(&logged);
    assert_eq!(
        metadata.get("method").and_then(Value::as_str),
        Some("password")
    );
    assert_eq!(
        metadata.get("mfa_enabled").and_then(Value::as_bool),
        Some(false)
    );
}
