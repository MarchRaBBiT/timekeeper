use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use timekeeper_contract::subject_requests as contract_subject_requests;
pub use timekeeper_contract::subject_requests::{
    CreateDataSubjectRequest, DataSubjectRequestResponse,
};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::request::RequestStatus;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DataSubjectRequestType {
    Access,
    Rectify,
    Delete,
    Stop,
}

impl DataSubjectRequestType {
    pub fn db_value(&self) -> &'static str {
        match self {
            DataSubjectRequestType::Access => "access",
            DataSubjectRequestType::Rectify => "rectify",
            DataSubjectRequestType::Delete => "delete",
            DataSubjectRequestType::Stop => "stop",
        }
    }
}

impl From<contract_subject_requests::DataSubjectRequestType> for DataSubjectRequestType {
    fn from(value: contract_subject_requests::DataSubjectRequestType) -> Self {
        match value {
            contract_subject_requests::DataSubjectRequestType::Access => Self::Access,
            contract_subject_requests::DataSubjectRequestType::Rectify => Self::Rectify,
            contract_subject_requests::DataSubjectRequestType::Delete => Self::Delete,
            contract_subject_requests::DataSubjectRequestType::Stop => Self::Stop,
        }
    }
}

impl From<DataSubjectRequestType> for contract_subject_requests::DataSubjectRequestType {
    fn from(value: DataSubjectRequestType) -> Self {
        match value {
            DataSubjectRequestType::Access => Self::Access,
            DataSubjectRequestType::Rectify => Self::Rectify,
            DataSubjectRequestType::Delete => Self::Delete,
            DataSubjectRequestType::Stop => Self::Stop,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DataSubjectRequest {
    pub id: String,
    pub user_id: String,
    pub request_type: DataSubjectRequestType,
    pub status: RequestStatus,
    pub details: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_by: Option<String>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub decision_comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DataSubjectRequest {
    pub fn new(
        user_id: String,
        request_type: DataSubjectRequestType,
        details: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            request_type,
            status: RequestStatus::Pending,
            details,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            decision_comment: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl From<DataSubjectRequest> for DataSubjectRequestResponse {
    fn from(request: DataSubjectRequest) -> Self {
        Self {
            id: request.id,
            user_id: request.user_id,
            request_type: request.request_type.into(),
            status: request.status.db_value().to_string(),
            details: request.details,
            approved_by: request.approved_by,
            approved_at: request.approved_at,
            rejected_by: request.rejected_by,
            rejected_at: request.rejected_at,
            cancelled_at: request.cancelled_at,
            decision_comment: request.decision_comment,
            created_at: request.created_at,
            updated_at: request.updated_at,
        }
    }
}
