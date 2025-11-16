//! Shared request-related enums used by multiple models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
/// Common workflow status for leave and overtime requests.
pub enum RequestStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

impl Default for RequestStatus {
    fn default() -> Self {
        RequestStatus::Pending
    }
}
