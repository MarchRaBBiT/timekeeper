use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConsentLog {
    pub id: String,
    pub user_id: String,
    pub purpose: String,
    pub policy_version: String,
    pub consented_at: DateTime<Utc>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsentLogResponse {
    pub id: String,
    pub purpose: String,
    pub policy_version: String,
    pub consented_at: DateTime<Utc>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<ConsentLog> for ConsentLogResponse {
    fn from(log: ConsentLog) -> Self {
        Self {
            id: log.id,
            purpose: log.purpose,
            policy_version: log.policy_version,
            consented_at: log.consented_at,
            ip: log.ip,
            user_agent: log.user_agent,
            request_id: log.request_id,
            created_at: log.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecordConsentPayload {
    pub purpose: String,
    pub policy_version: String,
}
