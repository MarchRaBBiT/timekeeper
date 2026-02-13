use crate::types::{AttendanceId, UserId};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AttendanceCorrectionStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
    Conflict,
}

impl AttendanceCorrectionStatus {
    pub fn db_value(&self) -> &'static str {
        match self {
            AttendanceCorrectionStatus::Pending => "pending",
            AttendanceCorrectionStatus::Approved => "approved",
            AttendanceCorrectionStatus::Rejected => "rejected",
            AttendanceCorrectionStatus::Cancelled => "cancelled",
            AttendanceCorrectionStatus::Conflict => "conflict",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorrectionBreakItem {
    pub break_start_time: NaiveDateTime,
    pub break_end_time: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttendanceCorrectionSnapshot {
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
    pub breaks: Vec<CorrectionBreakItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttendanceCorrectionRequest {
    pub date: NaiveDate,
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
    pub breaks: Option<Vec<CorrectionBreakItem>>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAttendanceCorrectionRequest {
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
    pub breaks: Option<Vec<CorrectionBreakItem>>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPayload {
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AttendanceCorrectionRequest {
    pub id: String,
    pub user_id: UserId,
    pub attendance_id: AttendanceId,
    pub date: NaiveDate,
    pub status: AttendanceCorrectionStatus,
    pub reason: String,
    pub original_snapshot_json: Value,
    pub proposed_values_json: Value,
    pub decision_comment: Option<String>,
    pub approved_by: Option<UserId>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_by: Option<UserId>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AttendanceCorrectionEffectiveValue {
    pub attendance_id: AttendanceId,
    pub source_request_id: String,
    pub clock_in_time_corrected: Option<NaiveDateTime>,
    pub clock_out_time_corrected: Option<NaiveDateTime>,
    pub break_records_corrected_json: Value,
    pub applied_by: Option<UserId>,
    pub applied_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceCorrectionResponse {
    pub id: String,
    pub user_id: UserId,
    pub attendance_id: AttendanceId,
    pub date: NaiveDate,
    pub status: AttendanceCorrectionStatus,
    pub reason: String,
    pub original_snapshot: AttendanceCorrectionSnapshot,
    pub proposed_values: AttendanceCorrectionSnapshot,
    pub decision_comment: Option<String>,
    pub approved_by: Option<UserId>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_by: Option<UserId>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AttendanceCorrectionRequest {
    pub fn parse_original_snapshot(
        &self,
    ) -> Result<AttendanceCorrectionSnapshot, serde_json::Error> {
        serde_json::from_value(self.original_snapshot_json.clone())
    }

    pub fn parse_proposed_values(&self) -> Result<AttendanceCorrectionSnapshot, serde_json::Error> {
        serde_json::from_value(self.proposed_values_json.clone())
    }

    pub fn to_response(&self) -> anyhow::Result<AttendanceCorrectionResponse> {
        Ok(AttendanceCorrectionResponse {
            id: self.id.clone(),
            user_id: self.user_id,
            attendance_id: self.attendance_id,
            date: self.date,
            status: self.status.clone(),
            reason: self.reason.clone(),
            original_snapshot: self.parse_original_snapshot()?,
            proposed_values: self.parse_proposed_values()?,
            decision_comment: self.decision_comment.clone(),
            approved_by: self.approved_by,
            approved_at: self.approved_at,
            rejected_by: self.rejected_by,
            rejected_at: self.rejected_at,
            cancelled_at: self.cancelled_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
