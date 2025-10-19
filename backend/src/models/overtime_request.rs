use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OvertimeRequest {
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
    pub updated_at: DateTime<Utc>,
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
pub struct CreateOvertimeRequest {
    pub date: NaiveDate,
    pub planned_hours: f64,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    fn overtime_request_status_serde_snake_case() {
        let s: RequestStatus = serde_json::from_str("\"approved\"").unwrap();
        assert!(matches!(s, RequestStatus::Approved));
        let v = serde_json::to_value(RequestStatus::Pending).unwrap();
        assert_eq!(v, serde_json::json!("pending"));
    }
}
