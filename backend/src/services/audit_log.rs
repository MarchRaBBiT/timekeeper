use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{types::Json, PgPool};

use crate::types::{AuditLogId, UserId};
use crate::{models::audit_log::AuditLog, repositories::audit_log};

#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Option<UserId>,
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
}

#[async_trait::async_trait]
pub trait AuditLogServiceTrait: Send + Sync {
    async fn record_event(&self, entry: AuditLogEntry) -> Result<(), sqlx::Error>;
    async fn delete_logs_before(&self, cutoff: DateTime<Utc>) -> Result<u64, sqlx::Error>;
}

#[async_trait::async_trait]
impl AuditLogServiceTrait for AuditLogService {
    async fn record_event(&self, entry: AuditLogEntry) -> Result<(), sqlx::Error> {
        let log = AuditLog {
            id: AuditLogId::new(),
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

        audit_log::insert_audit_log(&self.pool, &log).await
    }

    async fn delete_logs_before(&self, cutoff: DateTime<Utc>) -> Result<u64, sqlx::Error> {
        audit_log::delete_audit_logs_before(&self.pool, cutoff).await
    }
}
