use chrono::{Duration as ChronoDuration, Utc};
use serde_json::json;
use sqlx::types::Json;
use std::sync::OnceLock;
use timekeeper_backend::{
    models::{audit_log::AuditLog, user::UserRole},
    repositories::audit_log,
    types::AuditLogId,
};
use tokio::sync::Mutex;
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

#[tokio::test]
async fn audit_log_repo_inserts_and_fetches() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE audit_logs")
        .execute(&pool)
        .await
        .expect("truncate audit logs");

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let metadata_value = json!({ "source": "test" });
    let log = AuditLog {
        id: AuditLogId::new(),
        occurred_at: Utc::now(),
        actor_id: Some(user.id),
        actor_type: "user".into(),
        event_type: "attendance_clock_in".into(),
        target_type: Some("attendance".into()),
        target_id: Some(Uuid::new_v4().to_string()),
        result: "success".into(),
        error_code: None,
        metadata: Some(Json(metadata_value.clone())),
        ip: Some("127.0.0.1".into()),
        user_agent: Some("test-agent".into()),
        request_id: Some("req-123".into()),
    };

    audit_log::insert_audit_log(&pool, &log)
        .await
        .expect("insert audit log");

    let fetched = audit_log::fetch_audit_log(&pool, log.id)
        .await
        .expect("fetch audit log")
        .expect("audit log exists");

    assert_eq!(fetched.id, log.id);
    assert_eq!(fetched.actor_id, log.actor_id);
    assert_eq!(fetched.actor_type, log.actor_type);
    assert_eq!(fetched.event_type, log.event_type);
    assert_eq!(fetched.target_type, log.target_type);
    assert_eq!(fetched.target_id, log.target_id);
    assert_eq!(fetched.result, log.result);
    assert_eq!(fetched.error_code, log.error_code);
    assert_eq!(
        fetched.metadata.as_ref().map(|value| value.0.clone()),
        Some(metadata_value)
    );
    assert_eq!(fetched.ip, log.ip);
    assert_eq!(fetched.user_agent, log.user_agent);
    assert_eq!(fetched.request_id, log.request_id);
    assert_eq!(
        fetched.occurred_at.timestamp_micros(),
        log.occurred_at.timestamp_micros()
    );

    sqlx::query("TRUNCATE audit_logs")
        .execute(&pool)
        .await
        .expect("truncate audit logs");
}

#[tokio::test]
async fn audit_log_repo_deletes_logs_before_cutoff() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE audit_logs")
        .execute(&pool)
        .await
        .expect("truncate audit logs");

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let now = Utc::now();
    let old_log = AuditLog {
        id: AuditLogId::new(),
        occurred_at: now - ChronoDuration::days(40),
        actor_id: Some(user.id),
        actor_type: "user".into(),
        event_type: "admin_user_create".into(),
        target_type: Some("user".into()),
        target_id: Some(Uuid::new_v4().to_string()),
        result: "success".into(),
        error_code: None,
        metadata: None,
        ip: None,
        user_agent: None,
        request_id: Some("req-old".into()),
    };
    let recent_log = AuditLog {
        id: AuditLogId::new(),
        occurred_at: now - ChronoDuration::days(10),
        actor_id: Some(user.id),
        actor_type: "user".into(),
        event_type: "admin_user_create".into(),
        target_type: Some("user".into()),
        target_id: Some(Uuid::new_v4().to_string()),
        result: "success".into(),
        error_code: None,
        metadata: None,
        ip: None,
        user_agent: None,
        request_id: Some("req-recent".into()),
    };

    audit_log::insert_audit_log(&pool, &old_log)
        .await
        .expect("insert old log");
    audit_log::insert_audit_log(&pool, &recent_log)
        .await
        .expect("insert recent log");

    let cutoff = now - ChronoDuration::days(30);
    let deleted = audit_log::delete_audit_logs_before(&pool, cutoff)
        .await
        .expect("delete audit logs");

    assert_eq!(deleted, 1);

    let remaining: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs")
        .fetch_one(&pool)
        .await
        .expect("count remaining logs");
    assert_eq!(remaining.0, 1);

    let remaining_old: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE id = $1")
        .bind(old_log.id.to_string())
        .fetch_one(&pool)
        .await
        .expect("count old logs");
    assert_eq!(remaining_old.0, 0);

    sqlx::query("TRUNCATE audit_logs")
        .execute(&pool)
        .await
        .expect("truncate audit logs");
}

#[tokio::test]
async fn audit_log_export_reports_truncation_when_over_max_rows() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE audit_logs")
        .execute(&pool)
        .await
        .expect("truncate audit logs");

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    for idx in 0..3 {
        let log = AuditLog {
            id: AuditLogId::new(),
            occurred_at: Utc::now() + ChronoDuration::seconds(idx),
            actor_id: Some(user.id),
            actor_type: "user".into(),
            event_type: "export_test".into(),
            target_type: Some("audit".into()),
            target_id: Some(Uuid::new_v4().to_string()),
            result: "success".into(),
            error_code: None,
            metadata: None,
            ip: None,
            user_agent: None,
            request_id: Some(format!("req-{idx}")),
        };
        audit_log::insert_audit_log(&pool, &log)
            .await
            .expect("insert log");
    }

    let filters = audit_log::AuditLogFilters {
        event_type: Some("export_test".to_string()),
        ..Default::default()
    };
    let (rows, truncated) = audit_log::export_audit_logs(&pool, &filters, 2)
        .await
        .expect("export logs");

    assert_eq!(rows.len(), 2);
    assert!(truncated);

    sqlx::query("TRUNCATE audit_logs")
        .execute(&pool)
        .await
        .expect("truncate audit logs");
}
