use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LeaveRequest {
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
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LeaveType {
    Annual,
    Sick,
    Personal,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLeaveRequest {
    pub leave_type: LeaveType,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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

    pub fn approve(&mut self, approved_by: String) {
        self.status = RequestStatus::Approved;
        self.approved_by = Some(approved_by);
        self.approved_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn reject(&mut self, approved_by: String) {
        self.status = RequestStatus::Rejected;
        self.rejected_by = Some(approved_by);
        self.rejected_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

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
}
