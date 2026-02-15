use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use validator::Validate;

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct RecordConsentPayload {
    #[validate(length(max = 200))]
    pub purpose: String,
    #[validate(length(max = 100))]
    pub policy_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_consent_payload_rejects_too_long_purpose() {
        let payload = RecordConsentPayload {
            purpose: "a".repeat(201),
            policy_version: "v1".to_string(),
        };

        assert!(payload.validate().is_err());
    }

    #[test]
    fn record_consent_payload_accepts_max_purpose_length() {
        let payload = RecordConsentPayload {
            purpose: "a".repeat(200),
            policy_version: "v1".to_string(),
        };

        assert!(payload.validate().is_ok());
    }

    #[test]
    fn record_consent_payload_rejects_too_long_policy_version() {
        let payload = RecordConsentPayload {
            purpose: "analytics".to_string(),
            policy_version: "v".repeat(101),
        };

        assert!(payload.validate().is_err());
    }

    #[test]
    fn record_consent_payload_accepts_max_policy_version_length() {
        let payload = RecordConsentPayload {
            purpose: "analytics".to_string(),
            policy_version: "v".repeat(100),
        };

        assert!(payload.validate().is_ok());
    }
}
