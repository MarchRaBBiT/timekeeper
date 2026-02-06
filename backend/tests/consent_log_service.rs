use chrono::{Duration, Utc};
use sqlx::PgPool;
use timekeeper_backend::models::user::UserRole;
use timekeeper_backend::services::consent_log::ConsentLogService;
use uuid::Uuid;

mod support;

async fn migrate_db(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("run migrations");
}

#[tokio::test]
async fn consent_log_service_deletes_logs_before_cutoff() {
    let pool = support::test_pool().await;
    migrate_db(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let old = Utc::now() - Duration::days(10);

    sqlx::query(
        "INSERT INTO consent_logs \
         (id, user_id, purpose, policy_version, consented_at, ip, user_agent, request_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user.id.to_string())
    .bind("test-purpose")
    .bind("v1")
    .bind(old)
    .bind(Some("127.0.0.1".to_string()))
    .bind(Some("test-agent".to_string()))
    .bind(None::<String>)
    .execute(&pool)
    .await
    .expect("insert consent log");

    let service = ConsentLogService::new(pool.clone());
    let deleted = service
        .delete_logs_before(Utc::now() - Duration::days(1))
        .await
        .expect("delete logs");
    assert_eq!(deleted, 1);

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM consent_logs WHERE user_id = $1")
        .bind(user.id.to_string())
        .fetch_one(&pool)
        .await
        .expect("count logs");
    assert_eq!(remaining, 0);
}
