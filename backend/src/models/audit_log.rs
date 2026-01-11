use crate::types::{AuditLogId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{types::Json, FromRow};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: AuditLogId,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Option<UserId>,
    pub actor_type: String,
    pub event_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: String,
    pub error_code: Option<String>,
    pub metadata: Option<Json<Value>>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}
