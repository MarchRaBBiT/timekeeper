use sqlx::PgPool;

use crate::models::audit_log::AuditLog;

pub async fn insert_audit_log(pool: &PgPool, log: &AuditLog) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_logs \
         (id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result, \
         error_code, metadata, ip, user_agent, request_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(&log.id)
    .bind(log.occurred_at)
    .bind(&log.actor_id)
    .bind(&log.actor_type)
    .bind(&log.event_type)
    .bind(&log.target_type)
    .bind(&log.target_id)
    .bind(&log.result)
    .bind(&log.error_code)
    .bind(&log.metadata)
    .bind(&log.ip)
    .bind(&log.user_agent)
    .bind(&log.request_id)
    .execute(pool)
    .await
    .map(|_| ())
}

#[allow(dead_code)]
pub async fn fetch_audit_log(pool: &PgPool, id: &str) -> Result<Option<AuditLog>, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        "SELECT id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result, \
         error_code, metadata, ip, user_agent, request_id \
         FROM audit_logs WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
