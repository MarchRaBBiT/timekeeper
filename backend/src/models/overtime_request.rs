//! Models describing overtime requests and review workflow.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
/// Database representation of an overtime work request.
pub struct OvertimeRequest {
    /// Unique identifier for the overtime request.
    pub id: String,
    /// Identifier of the employee submitting the request.
    pub user_id: String,
    /// Date when the overtime is planned.
    pub date: NaiveDate,
    /// Number of overtime hours planned.
    pub planned_hours: f64,
    /// Optional justification provided by the requester.
    pub reason: Option<String>,
    /// Current status of the request.
    pub status: RequestStatus,
    /// Administrator who approved the request, if any.
    pub approved_by: Option<String>,
    /// Timestamp when the request received approval.
    pub approved_at: Option<DateTime<Utc>>,
    /// Administrator who rejected the request, if any.
    pub rejected_by: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
/// Workflow status for an overtime request.
pub enum RequestStatus {
    /// Awaiting review.
    Pending,
    /// Approved by an administrator.
    Approved,
    /// Rejected by an administrator.
    Rejected,
    /// Cancelled by the requester or system.
    Cancelled,
}

impl Default for RequestStatus {
    fn default() -> Self {
        RequestStatus::Pending
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// Payload used to create a new overtime request.
pub struct CreateOvertimeRequest {
    pub date: NaiveDate,
    pub planned_hours: f64,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
/// API response returned for overtime requests.
pub struct OvertimeRequestResponse {
    pub id: String,
    pub user_id: String,
    pub date: NaiveDate,
    pub planned_hours: f64,
    pub reason: Option<String>,
    pub status: RequestStatus,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_by: Option<String>,
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
        user_id: String,
        date: NaiveDate,
        planned_hours: f64,
        reason: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
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
    pub fn approve(&mut self, approved_by: String) {
        self.status = RequestStatus::Approved;
        self.approved_by = Some(approved_by);
        self.approved_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Marks the request as rejected.
    pub fn reject(&mut self, approved_by: String) {
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
    }

    #[test]
    fn overtime_request_state_transitions() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();

        let mut request = OvertimeRequest::new("user".into(), date, 2.5, None);
        assert!(request.is_pending());
        request.approve("approver".into());
        assert!(matches!(request.status, RequestStatus::Approved));
        assert_eq!(request.approved_by.as_deref(), Some("approver"));
        assert!(request.approved_at.is_some());

        let mut rejected = OvertimeRequest::new("user".into(), date, 1.0, None);
        rejected.reject("approver2".into());
        assert!(matches!(rejected.status, RequestStatus::Rejected));
        assert_eq!(rejected.rejected_by.as_deref(), Some("approver2"));
        assert!(rejected.rejected_at.is_some());
    }
}
