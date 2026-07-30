//! Models describing employee leave requests and their lifecycle.

use crate::types::{LeaveRequestId, UserId};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::{postgres::PgTypeInfo, Decode, FromRow, Postgres, Type};
pub use timekeeper_contract::requests::{CreateLeaveRequest, LeaveRequestResponse};
use utoipa::ToSchema;

pub use crate::models::request::RequestStatus;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LeaveAcquisitionUnit {
    #[default]
    Day,
    HalfAm,
    HalfPm,
    Hour,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
/// Database representation of a leave request submitted by an employee.
pub struct LeaveRequest {
    /// Unique identifier for the leave request.
    pub id: LeaveRequestId,
    /// Identifier of the employee who submitted the request.
    pub user_id: UserId,
    /// Type of leave being requested.
    pub leave_type: LeaveType,
    /// First day of the requested leave period.
    pub start_date: NaiveDate,
    /// Last day of the requested leave period.
    pub end_date: NaiveDate,
    #[sqlx(default)]
    pub acquisition_unit: LeaveAcquisitionUnit,
    #[sqlx(default)]
    pub start_time: Option<NaiveTime>,
    #[sqlx(default)]
    pub end_time: Option<NaiveTime>,
    #[sqlx(default)]
    pub requested_minutes: Option<i32>,
    /// Optional user-provided explanation for the leave.
    pub reason: Option<String>,
    /// Current status of the leave request.
    pub status: RequestStatus,
    /// Administrator who approved the request, if any.
    pub approved_by: Option<UserId>,
    /// Timestamp when the request was approved.
    pub approved_at: Option<DateTime<Utc>>,
    /// Administrator who rejected the request, if any.
    pub rejected_by: Option<UserId>,
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

#[derive(Debug, Clone, PartialEq, Eq, ToSchema)]
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
    /// Company-specific leave type backed by `leave_types`.
    Custom(String),
}

impl LeaveType {
    #[allow(dead_code)]
    pub fn db_value(&self) -> &str {
        match self {
            LeaveType::Annual => "annual",
            LeaveType::Sick => "sick",
            LeaveType::Personal => "personal",
            LeaveType::Other => "other",
            LeaveType::Custom(value) => value,
        }
    }

    pub fn from_db_value(value: String) -> Self {
        match value.as_str() {
            "annual" => Self::Annual,
            "sick" => Self::Sick,
            "personal" => Self::Personal,
            "other" => Self::Other,
            _ => Self::Custom(value),
        }
    }
}

impl Serialize for LeaveType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.db_value())
    }
}

impl<'de> Deserialize<'de> for LeaveType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::from_db_value)
    }
}

impl Type<Postgres> for LeaveType {
    fn type_info() -> PgTypeInfo {
        <String as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as Type<Postgres>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Postgres> for LeaveType {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        <String as Decode<Postgres>>::decode(value).map(Self::from_db_value)
    }
}

impl From<LeaveRequest> for LeaveRequestResponse {
    /// Converts the database entity into its transport-friendly variant.
    fn from(request: LeaveRequest) -> Self {
        LeaveRequestResponse {
            id: request.id.to_string(),
            user_id: request.user_id.to_string(),
            leave_type: request.leave_type.db_value().to_string(),
            start_date: request.start_date,
            end_date: request.end_date,
            acquisition_unit: match request.acquisition_unit {
                LeaveAcquisitionUnit::Day => {
                    timekeeper_contract::requests::LeaveAcquisitionUnit::Day
                }
                LeaveAcquisitionUnit::HalfAm => {
                    timekeeper_contract::requests::LeaveAcquisitionUnit::HalfAm
                }
                LeaveAcquisitionUnit::HalfPm => {
                    timekeeper_contract::requests::LeaveAcquisitionUnit::HalfPm
                }
                LeaveAcquisitionUnit::Hour => {
                    timekeeper_contract::requests::LeaveAcquisitionUnit::Hour
                }
            },
            start_time: request.start_time,
            end_time: request.end_time,
            requested_minutes: request.requested_minutes,
            reason: request.reason,
            status: request.status.db_value().to_string(),
            approved_by: request.approved_by.map(|id| id.to_string()),
            approved_at: request.approved_at.map(|timestamp| timestamp.to_rfc3339()),
            rejected_by: request.rejected_by.map(|id| id.to_string()),
            rejected_at: request.rejected_at.map(|timestamp| timestamp.to_rfc3339()),
            cancelled_at: request.cancelled_at.map(|timestamp| timestamp.to_rfc3339()),
            decision_comment: request.decision_comment,
            created_at: request.created_at.to_rfc3339(),
        }
    }
}

impl LeaveRequest {
    /// Creates a new leave request pending approval.
    pub fn new(
        user_id: UserId,
        leave_type: LeaveType,
        start_date: NaiveDate,
        end_date: NaiveDate,
        reason: Option<String>,
    ) -> Self {
        Self::new_with_duration(
            user_id,
            leave_type,
            start_date,
            end_date,
            LeaveAcquisitionUnit::Day,
            None,
            None,
            None,
            reason,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_duration(
        user_id: UserId,
        leave_type: LeaveType,
        start_date: NaiveDate,
        end_date: NaiveDate,
        acquisition_unit: LeaveAcquisitionUnit,
        start_time: Option<NaiveTime>,
        end_time: Option<NaiveTime>,
        requested_minutes: Option<i32>,
        reason: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: LeaveRequestId::new(),
            user_id,
            leave_type,
            start_date,
            end_date,
            acquisition_unit,
            start_time,
            end_time,
            requested_minutes,
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
    pub fn approve(&mut self, approved_by: UserId) {
        self.status = RequestStatus::Approved;
        self.approved_by = Some(approved_by);
        self.approved_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Marks the request as rejected and records reviewer details.
    #[allow(dead_code)]
    pub fn reject(&mut self, approved_by: UserId) {
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
        assert_eq!(LeaveType::Annual.db_value(), "annual");

        // RequestStatus
        let rs: RequestStatus = serde_json::from_str("\"rejected\"").unwrap();
        assert!(matches!(rs, RequestStatus::Rejected));
        let vrs = serde_json::to_value(RequestStatus::Cancelled).unwrap();
        assert_eq!(vrs, serde_json::json!("cancelled"));
        assert_eq!(RequestStatus::Pending.db_value(), "pending");
    }

    #[test]
    fn leave_request_state_transitions() {
        use chrono::NaiveDate;

        let start = NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 4, 2).unwrap();

        let user_id = UserId::new();
        let admin_id = UserId::new();
        let admin2_id = UserId::new();

        let mut request = LeaveRequest::new(user_id, LeaveType::Annual, start, end, None);
        assert!(request.is_pending());

        request.approve(admin_id);
        assert!(matches!(request.status, RequestStatus::Approved));
        assert_eq!(request.approved_by, Some(admin_id));
        assert!(request.approved_at.is_some());

        let mut rejected = LeaveRequest::new(user_id, LeaveType::Sick, start, end, None);
        rejected.reject(admin2_id);
        assert!(matches!(rejected.status, RequestStatus::Rejected));
        assert_eq!(rejected.rejected_by, Some(admin2_id));
        assert!(rejected.rejected_at.is_some());
    }
}
