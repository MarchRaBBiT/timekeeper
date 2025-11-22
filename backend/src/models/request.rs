//! Shared request-related enums used by multiple models.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema, Default)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
/// Common workflow status for leave and overtime requests.
pub enum RequestStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

impl RequestStatus {
    pub fn db_value(&self) -> &'static str {
        match self {
            RequestStatus::Pending => "pending",
            RequestStatus::Approved => "approved",
            RequestStatus::Rejected => "rejected",
            RequestStatus::Cancelled => "cancelled",
        }
    }
}
