//! Models describing overtime requests and review workflow.

use crate::types::{OvertimeRequestId, UserId};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use validator::Validate;

pub use crate::models::request::RequestStatus;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
/// Database representation of an overtime work request.
pub struct OvertimeRequest {
    /// Unique identifier for the overtime request.
    pub id: OvertimeRequestId,
    /// Identifier of the employee submitting the request.
    pub user_id: UserId,
    /// Date when the overtime is planned.
    pub date: NaiveDate,
    /// Number of overtime hours planned.
    pub planned_hours: f64,
    /// Optional justification provided by the requester.
    pub reason: Option<String>,
    /// Current status of the request.
    pub status: RequestStatus,
    /// Administrator who approved the request, if any.
    pub approved_by: Option<UserId>,
    /// Timestamp when the request received approval.
    pub approved_at: Option<DateTime<Utc>>,
    /// Administrator who rejected the request, if any.
    pub rejected_by: Option<UserId>,
    /// Timestamp when the request was rejected.
    pub rejected_at: Option<DateTime<Utc>>,
    /// Timestamp when the requester cancelled the request.
    pub cancelled_at: Option<DateTime<Utc>>,
    /// Supplemental comments recorded during review.
    pub decision_comment: Option<String>,
    /// Creation timestamp for auditing.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp for auditing.
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Payload used to create a new overtime request.
pub struct CreateOvertimeRequest {
    pub date: NaiveDate,
    #[validate(range(min = 0.5, max = 24.0))]
    pub planned_hours: f64,
    #[validate(length(max = 500))]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// API response returned for overtime requests.
pub struct OvertimeRequestResponse {
    pub id: OvertimeRequestId,
    pub user_id: UserId,
    pub date: NaiveDate,
    pub planned_hours: f64,
    pub reason: Option<String>,
    pub status: RequestStatus,
    pub approved_by: Option<UserId>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_by: Option<UserId>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub decision_comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<OvertimeRequest> for OvertimeRequestResponse {
    /// Converts a persisted overtime request into its response form.
    fn from(request: OvertimeRequest) -> Self {
        OvertimeRequestResponse {
            id: request.id,
            user_id: request.user_id,
            date: request.date,
            planned_hours: request.planned_hours,
            reason: request.reason,
            status: request.status,
            approved_by: request.approved_by,
            approved_at: request.approved_at,
            rejected_by: request.rejected_by,
            rejected_at: request.rejected_at,
            cancelled_at: request.cancelled_at,
            decision_comment: request.decision_comment,
            created_at: request.created_at,
        }
    }
}

impl OvertimeRequest {
    /// Creates a new overtime request pending review.
    pub fn new(
        user_id: UserId,
        date: NaiveDate,
        planned_hours: f64,
        reason: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: OvertimeRequestId::new(),
            user_id,
            date,
            planned_hours,
            reason,
            status: RequestStatus::Pending,
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

    /// Marks the request as approved.
    #[allow(dead_code)]
    pub fn approve(&mut self, approved_by: UserId) {
        self.status = RequestStatus::Approved;
        self.approved_by = Some(approved_by);
        self.approved_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Marks the request as rejected.
    #[allow(dead_code)]
    pub fn reject(&mut self, approved_by: UserId) {
        self.status = RequestStatus::Rejected;
        self.rejected_by = Some(approved_by);
        self.rejected_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Returns `true` while the request is awaiting review.
    pub fn is_pending(&self) -> bool {
        matches!(self.status, RequestStatus::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overtime_request_status_serde_snake_case() {
        let s: RequestStatus = serde_json::from_str("\"approved\"").unwrap();
        assert!(matches!(s, RequestStatus::Approved));
        let v = serde_json::to_value(RequestStatus::Pending).unwrap();
        assert_eq!(v, serde_json::json!("pending"));
        assert_eq!(RequestStatus::Approved.db_value(), "approved");
    }

    #[test]
    fn overtime_request_state_transitions() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();

        let user_id = UserId::new();
        let admin_id = UserId::new();
        let admin2_id = UserId::new();

        let mut request = OvertimeRequest::new(user_id, date, 2.5, None);
        assert!(request.is_pending());
        request.approve(admin_id);
        assert!(matches!(request.status, RequestStatus::Approved));
        assert_eq!(request.approved_by, Some(admin_id));
        assert!(request.approved_at.is_some());

        let mut rejected = OvertimeRequest::new(user_id, date, 1.0, None);
        rejected.reject(admin2_id);
        assert!(matches!(rejected.status, RequestStatus::Rejected));
        assert_eq!(rejected.rejected_by, Some(admin2_id));
        assert!(rejected.rejected_at.is_some());
    }
}
