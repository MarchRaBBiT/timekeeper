use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{types::Json, PgPool};
use uuid::Uuid;

use crate::{handlers::audit_log_repo, models::audit_log::AuditLog};

#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Option<String>,
    pub actor_type: String,
    pub event_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: String,
    pub error_code: Option<String>,
    pub metadata: Option<Value>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditLogService {
    pool: PgPool,
}

impl AuditLogService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn record_event(&self, entry: AuditLogEntry) -> Result<(), sqlx::Error> {
        let log = AuditLog {
            id: Uuid::new_v4().to_string(),
            occurred_at: entry.occurred_at,
            actor_id: entry.actor_id,
            actor_type: entry.actor_type,
            event_type: entry.event_type,
            target_type: entry.target_type,
            target_id: entry.target_id,
            result: entry.result,
            error_code: entry.error_code,
            metadata: entry.metadata.map(Json),
            ip: entry.ip,
            user_agent: entry.user_agent,
            request_id: entry.request_id,
        };

        audit_log_repo::insert_audit_log(&self.pool, &log).await
    }

    pub async fn delete_logs_before(&self, cutoff: DateTime<Utc>) -> Result<u64, sqlx::Error> {
        audit_log_repo::delete_audit_logs_before(&self.pool, cutoff).await
    }
}
