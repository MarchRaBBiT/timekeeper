use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::models::audit_log::AuditLog;
use crate::types::{AuditLogId, UserId};

#[derive(Debug, Clone, Default)]
pub struct AuditLogFilters {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub actor_id: Option<UserId>,
    pub actor_type: Option<String>,
    pub event_type: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: Option<String>,
}

pub async fn insert_audit_log(pool: &PgPool, log: &AuditLog) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_logs \
         (id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result, \
         error_code, metadata, ip, user_agent, request_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(log.id.to_string())
    .bind(log.occurred_at)
    .bind(log.actor_id.as_ref().map(|id| id.to_string()))
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

pub async fn fetch_audit_log(
    pool: &PgPool,
    id: AuditLogId,
) -> Result<Option<AuditLog>, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        "SELECT id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result, \
         error_code, metadata, ip, user_agent, request_id \
         FROM audit_logs WHERE id = $1",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await
}

pub async fn list_audit_logs(
    pool: &PgPool,
    filters: &AuditLogFilters,
    per_page: i64,
    offset: i64,
) -> Result<(Vec<AuditLog>, i64), sqlx::Error> {
    let items = query_audit_logs(pool, filters, Some((per_page, offset))).await?;

    let mut count_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM audit_logs");
    let mut count_has_clause = false;
    apply_audit_log_filters(&mut count_builder, &mut count_has_clause, filters);
    let total = count_builder
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;

    Ok((items, total))
}

pub async fn export_audit_logs(
    pool: &PgPool,
    filters: &AuditLogFilters,
) -> Result<Vec<AuditLog>, sqlx::Error> {
    query_audit_logs(pool, filters, None).await
}

pub async fn delete_audit_logs_before(
    pool: &PgPool,
    cutoff: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM audit_logs WHERE occurred_at < $1")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

async fn query_audit_logs(
    pool: &PgPool,
    filters: &AuditLogFilters,
    pagination: Option<(i64, i64)>,
) -> Result<Vec<AuditLog>, sqlx::Error> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result, \
         error_code, metadata, ip, user_agent, request_id FROM audit_logs",
    );
    let mut has_clause = false;
    apply_audit_log_filters(&mut builder, &mut has_clause, filters);
    builder.push(" ORDER BY occurred_at DESC, id DESC");

    if let Some((per_page, offset)) = pagination {
        builder
            .push(" LIMIT ")
            .push_bind(per_page)
            .push(" OFFSET ")
            .push_bind(offset);
    }

    builder.build_query_as::<AuditLog>().fetch_all(pool).await
}

fn apply_audit_log_filters(
    builder: &mut QueryBuilder<'_, Postgres>,
    has_clause: &mut bool,
    filters: &AuditLogFilters,
) {
    if let Some(from) = filters.from.as_ref() {
        push_clause(builder, has_clause);
        builder.push("occurred_at >= ").push_bind(from.to_owned());
    }
    if let Some(to) = filters.to.as_ref() {
        push_clause(builder, has_clause);
        builder.push("occurred_at <= ").push_bind(to.to_owned());
    }
    if let Some(actor_id) = filters.actor_id.as_ref() {
        push_clause(builder, has_clause);
        builder.push("actor_id = ").push_bind(actor_id.to_string());
    }
    if let Some(actor_type) = filters.actor_type.as_ref() {
        push_clause(builder, has_clause);
        builder
            .push("actor_type = ")
            .push_bind(actor_type.to_string());
    }
    if let Some(event_type) = filters.event_type.as_ref() {
        push_clause(builder, has_clause);
        builder
            .push("event_type = ")
            .push_bind(event_type.to_string());
    }
    if let Some(target_type) = filters.target_type.as_ref() {
        push_clause(builder, has_clause);
        builder
            .push("target_type = ")
            .push_bind(target_type.to_string());
    }
    if let Some(target_id) = filters.target_id.as_ref() {
        push_clause(builder, has_clause);
        builder
            .push("target_id = ")
            .push_bind(target_id.to_string());
    }
    if let Some(result) = filters.result.as_ref() {
        push_clause(builder, has_clause);
        builder.push("result = ").push_bind(result.to_string());
    }
}

fn push_clause(builder: &mut QueryBuilder<'_, Postgres>, has_clause: &mut bool) {
    if *has_clause {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        *has_clause = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_log_filters_default_all_none() {
        let filters = AuditLogFilters::default();
        assert!(filters.from.is_none());
        assert!(filters.to.is_none());
        assert!(filters.actor_id.is_none());
        assert!(filters.actor_type.is_none());
        assert!(filters.event_type.is_none());
        assert!(filters.target_type.is_none());
        assert!(filters.target_id.is_none());
        assert!(filters.result.is_none());
    }

    #[test]
    fn audit_log_filters_all_fields() {
        let user_id = UserId::new();
        let filters = AuditLogFilters {
            from: Some(Utc::now()),
            to: Some(Utc::now()),
            actor_id: Some(user_id),
            actor_type: Some("user".to_string()),
            event_type: Some("login".to_string()),
            target_type: Some("attendance".to_string()),
            target_id: Some("att123".to_string()),
            result: Some("success".to_string()),
        };
        assert!(filters.from.is_some());
        assert!(filters.to.is_some());
        assert_eq!(filters.actor_id, Some(user_id));
        assert_eq!(filters.actor_type, Some("user".to_string()));
        assert_eq!(filters.event_type, Some("login".to_string()));
        assert_eq!(filters.target_type, Some("attendance".to_string()));
        assert_eq!(filters.target_id, Some("att123".to_string()));
        assert_eq!(filters.result, Some("success".to_string()));
    }
}
