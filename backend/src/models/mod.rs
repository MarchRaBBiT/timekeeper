//! Data models shared across database access and API handlers.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Query parameters for paginated endpoints.
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct PaginationQuery {
    /// Maximum number of records to return (default: 50, max: 500).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Number of records to skip (default: 0).
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

impl PaginationQuery {
    /// Returns a clamped limit value (1..=500).
    pub fn limit(&self) -> i64 {
        self.limit.clamp(1, 500)
    }

    /// Returns offset, floored at 0.
    pub fn offset(&self) -> i64 {
        self.offset.max(0)
    }
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            offset: 0,
        }
    }
}

/// Wrapper for paginated API responses.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    /// The data items for the current page.
    pub data: Vec<T>,
    /// Total number of records matching the query.
    pub total: i64,
    /// Number of records returned in this response.
    pub limit: i64,
    /// Number of records skipped.
    pub offset: i64,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        Self {
            data,
            total,
            limit,
            offset,
        }
    }
}

pub mod attendance;
pub mod audit_log;
pub mod break_record;
pub mod consent_log;
pub mod holiday;
pub mod holiday_exception;
pub mod leave_request;
pub mod overtime_request;
pub mod password_reset;
pub mod request;
pub mod subject_request;
pub mod user;
