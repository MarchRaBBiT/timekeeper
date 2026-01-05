//! Models describing employee leave requests and their lifecycle.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

pub use crate::models::request::RequestStatus;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
/// Database representation of a leave request submitted by an employee.
pub struct LeaveRequest {
    /// Unique identifier for the leave request.
    pub id: String,
    /// Identifier of the employee who submitted the request.
    pub user_id: String,
    /// Type of leave being requested.
    pub leave_type: LeaveType,
    /// First day of the requested leave period.
    pub start_date: NaiveDate,
    /// Last day of the requested leave period.
    pub end_date: NaiveDate,
    /// Optional user-provided explanation for the leave.
    pub reason: Option<String>,
    /// Current status of the leave request.
    pub status: RequestStatus,
    /// Administrator who approved the request, if any.
    pub approved_by: Option<String>,
    /// Timestamp when the request was approved.
    pub approved_at: Option<DateTime<Utc>>,
    /// Administrator who rejected the request, if any.
    pub rejected_by: Option<String>,
    /// Timestamp when the request was rejected.
    pub rejected_at: Option<DateTime<Utc>>,
    /// Timestamp when the requester cancelled the request.
    pub cancelled_at: Option<DateTime<Utc>>,
    /// Supplemental notes recorded during approval or rejection.
    pub decision_comment: Option<String>,
    /// Creation timestamp for auditing.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp for auditing.
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
/// Supported leave categories.
pub enum LeaveType {
    /// Planned vacation or personal time off.
    Annual,
    /// Sick leave.
    Sick,
    /// Personal leave not covered by other categories.
    Personal,
    /// Custom leave type stored as free-form text.
    Other,
}

impl LeaveType {
    pub fn db_value(&self) -> &'static str {
        match self {
            LeaveType::Annual => "annual",
            LeaveType::Sick => "sick",
            LeaveType::Personal => "personal",
            LeaveType::Other => "other",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Payload used to create a new leave request.
#[validate(schema(function = "validate_leave_date_range"))]
pub struct CreateLeaveRequest {
    pub leave_type: LeaveType,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    #[validate(length(max = 500))]
    pub reason: Option<String>,
}

fn validate_leave_date_range(req: &CreateLeaveRequest) -> Result<(), validator::ValidationError> {
    if req.start_date > req.end_date {
        return Err(validator::ValidationError::new("start_date_after_end_date"));
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// API representation shared with clients.
pub struct LeaveRequestResponse {
    pub id: String,
    pub user_id: String,
    pub leave_type: LeaveType,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
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

impl From<LeaveRequest> for LeaveRequestResponse {
    /// Converts the database entity into its transport-friendly variant.
    fn from(request: LeaveRequest) -> Self {
        LeaveRequestResponse {
            id: request.id,
            user_id: request.user_id,
            leave_type: request.leave_type,
            start_date: request.start_date,
            end_date: request.end_date,
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

impl LeaveRequest {
    /// Creates a new leave request pending approval.
    pub fn new(
        user_id: String,
        leave_type: LeaveType,
        start_date: NaiveDate,
        end_date: NaiveDate,
        reason: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            leave_type,
            start_date,
            end_date,
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

    /// Marks the request as approved and records reviewer details.
    #[allow(dead_code)]
    pub fn approve(&mut self, approved_by: String) {
        self.status = RequestStatus::Approved;
        self.approved_by = Some(approved_by);
        self.approved_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Marks the request as rejected and records reviewer details.
    #[allow(dead_code)]
    pub fn reject(&mut self, approved_by: String) {
        self.status = RequestStatus::Rejected;
        self.rejected_by = Some(approved_by);
        self.rejected_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Returns `true` while the request is awaiting a reviewer decision.
    pub fn is_pending(&self) -> bool {
        matches!(self.status, RequestStatus::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leave_type_and_status_serde_snake_case() {
        // LeaveType
        let lt: LeaveType = serde_json::from_str("\"annual\"").unwrap();
        assert!(matches!(lt, LeaveType::Annual));
        let vlt = serde_json::to_value(LeaveType::Personal).unwrap();
        assert_eq!(vlt, serde_json::json!("personal"));

        // RequestStatus
        let rs: RequestStatus = serde_json::from_str("\"rejected\"").unwrap();
        assert!(matches!(rs, RequestStatus::Rejected));
        let vrs = serde_json::to_value(RequestStatus::Cancelled).unwrap();
        assert_eq!(vrs, serde_json::json!("cancelled"));
    }

    #[test]
    fn leave_request_state_transitions() {
        use chrono::NaiveDate;

        let start = NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 4, 2).unwrap();

        let mut request = LeaveRequest::new("user".into(), LeaveType::Annual, start, end, None);
        assert!(request.is_pending());

        request.approve("admin".into());
        assert!(matches!(request.status, RequestStatus::Approved));
        assert_eq!(request.approved_by.as_deref(), Some("admin"));
        assert!(request.approved_at.is_some());

        let mut rejected = LeaveRequest::new("user".into(), LeaveType::Sick, start, end, None);
        rejected.reject("admin2".into());
        assert!(matches!(rejected.status, RequestStatus::Rejected));
        assert_eq!(rejected.rejected_by.as_deref(), Some("admin2"));
        assert!(rejected.rejected_at.is_some());
    }
}
