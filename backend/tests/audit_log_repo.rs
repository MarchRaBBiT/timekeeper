use chrono::Utc;
use serde_json::json;
use sqlx::types::Json;
use timekeeper_backend::{
    handlers::audit_log_repo,
    models::{audit_log::AuditLog, user::UserRole},
};
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

#[tokio::test]
async fn audit_log_repo_inserts_and_fetches() {
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let metadata_value = json!({ "source": "test" });
    let log = AuditLog {
        id: Uuid::new_v4().to_string(),
        occurred_at: Utc::now(),
        actor_id: Some(user.id.clone()),
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

    audit_log_repo::insert_audit_log(&pool, &log)
        .await
        .expect("insert audit log");

    let fetched = audit_log_repo::fetch_audit_log(&pool, &log.id)
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
}
