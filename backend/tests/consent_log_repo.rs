use chrono::{Duration as ChronoDuration, Utc};
use std::sync::OnceLock;
use timekeeper_backend::{
    models::{consent_log::ConsentLog, user::UserRole},
    repositories::consent_log,
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
async fn consent_log_repo_deletes_logs_before_cutoff() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE consent_logs")
        .execute(&pool)
        .await
        .expect("truncate consent logs");

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let now = Utc::now();
    let old_log = ConsentLog {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.to_string(),
        purpose: "attendance".into(),
        policy_version: "2026-01".into(),
        consented_at: now - ChronoDuration::days(40),
        ip: None,
        user_agent: None,
        request_id: None,
        created_at: now - ChronoDuration::days(40),
    };
    let recent_log = ConsentLog {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.to_string(),
        purpose: "attendance".into(),
        policy_version: "2026-02".into(),
        consented_at: now - ChronoDuration::days(5),
        ip: None,
        user_agent: None,
        request_id: None,
        created_at: now - ChronoDuration::days(5),
    };

    consent_log::insert_consent_log(&pool, &old_log)
        .await
        .expect("insert old consent");
    consent_log::insert_consent_log(&pool, &recent_log)
        .await
        .expect("insert recent consent");

    let cutoff = now - ChronoDuration::days(30);
    let deleted = consent_log::delete_consent_logs_before(&pool, cutoff)
        .await
        .expect("delete consent logs");

    assert_eq!(deleted, 1);

    let remaining: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM consent_logs")
        .fetch_one(&pool)
        .await
        .expect("count remaining logs");
    assert_eq!(remaining.0, 1);

    sqlx::query("TRUNCATE consent_logs")
        .execute(&pool)
        .await
        .expect("truncate consent logs");
}

#[tokio::test]
async fn consent_log_repo_orders_by_consented_at_desc() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE consent_logs")
        .execute(&pool)
        .await
        .expect("truncate consent logs");

    let user = support::seed_user(&pool, UserRole::Admin, false).await;
    let now = Utc::now();
    let older = ConsentLog {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.to_string(),
        purpose: "attendance".into(),
        policy_version: "2026-01".into(),
        consented_at: now - ChronoDuration::days(1),
        ip: None,
        user_agent: None,
        request_id: None,
        created_at: now - ChronoDuration::days(1),
    };
    let newer = ConsentLog {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.to_string(),
        purpose: "attendance".into(),
        policy_version: "2026-02".into(),
        consented_at: now,
        ip: None,
        user_agent: None,
        request_id: None,
        created_at: now,
    };

    consent_log::insert_consent_log(&pool, &older)
        .await
        .expect("insert older consent");
    consent_log::insert_consent_log(&pool, &newer)
        .await
        .expect("insert newer consent");

    let user_id = user.id.to_string();
    let logs = consent_log::list_consent_logs_for_user(&pool, &user_id)
        .await
        .expect("list consent logs");

    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].policy_version, "2026-02");
    assert_eq!(logs[1].policy_version, "2026-01");

    sqlx::query("TRUNCATE consent_logs")
        .execute(&pool)
        .await
        .expect("truncate consent logs");
}
