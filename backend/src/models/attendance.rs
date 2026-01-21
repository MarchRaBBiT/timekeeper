//! Models that represent employee attendance records and related requests.

use crate::models::break_record::BreakRecordResponse;
use crate::types::{AttendanceId, BreakRecordId, UserId};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
/// Persistent record of a single day's attendance for an employee.
pub struct Attendance {
    /// Unique identifier for the attendance record.
    pub id: AttendanceId,
    /// Identifier of the employee that owns the record.
    pub user_id: UserId,
    /// Calendar day the record tracks.
    pub date: NaiveDate,
    /// Timestamp when the employee clocked in, if any.
    pub clock_in_time: Option<NaiveDateTime>,
    /// Timestamp when the employee clocked out, if any.
    pub clock_out_time: Option<NaiveDateTime>,
    /// High-level status describing the attendance outcome for the day.
    pub status: AttendanceStatus,
    /// Total hours worked for the day once both clock-in and clock-out are present.
    pub total_work_hours: Option<f64>,
    /// Creation timestamp for auditing.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp for auditing.
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema, Default)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
/// Normalized status values stored in the database.
pub enum AttendanceStatus {
    /// Employee was present and on time.
    #[default]
    Present,
    /// Employee was absent for the entire day.
    Absent,
    /// Employee arrived late relative to the configured threshold.
    Late,
    /// Employee was present for only part of the day.
    HalfDay,
}

impl AttendanceStatus {
    pub fn db_value(&self) -> &'static str {
        match self {
            AttendanceStatus::Present => "present",
            AttendanceStatus::Absent => "absent",
            AttendanceStatus::Late => "late",
            AttendanceStatus::HalfDay => "half_day",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Request payload used when an employee clocks in.
pub struct ClockInRequest {
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Request payload used when an employee clocks out.
pub struct ClockOutRequest {
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Request payload for starting a break against an attendance record.
pub struct BreakStartRequest {
    pub attendance_id: AttendanceId,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Request payload for ending a break session.
pub struct BreakEndRequest {
    pub break_record_id: BreakRecordId,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// API representation of attendance with associated break records.
pub struct AttendanceResponse {
    pub id: AttendanceId,
    pub user_id: UserId,
    pub date: NaiveDate,
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
    pub status: AttendanceStatus,
    pub total_work_hours: Option<f64>,
    pub break_records: Vec<BreakRecordResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// High-level summary for reporting an employee's monthly attendance.
pub struct AttendanceSummary {
    pub month: u32,
    pub year: i32,
    pub total_work_hours: f64,
    pub total_work_days: i32,
    pub average_daily_hours: f64,
}

impl From<Attendance> for AttendanceResponse {
    fn from(a: Attendance) -> Self {
        Self {
            id: a.id,
            user_id: a.user_id,
            date: a.date,
            clock_in_time: a.clock_in_time,
            clock_out_time: a.clock_out_time,
            status: a.status,
            total_work_hours: a.total_work_hours,
            break_records: Vec::new(),
        }
    }
}

impl Attendance {
    /// Builds a new attendance record with default status and timestamps.
    pub fn new(user_id: UserId, date: NaiveDate, now: DateTime<Utc>) -> Self {
        Self {
            id: AttendanceId::new(),
            user_id,
            date,
            clock_in_time: None,
            clock_out_time: None,
            status: AttendanceStatus::Present,
            total_work_hours: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Recomputes `total_work_hours` when both clock-in and clock-out exist.
    /// `break_minutes` should be the total minutes spent on breaks for the day.
    pub fn calculate_work_hours(&mut self, break_minutes: i64) {
        if let (Some(clock_in), Some(clock_out)) = (self.clock_in_time, self.clock_out_time) {
            let duration = clock_out - clock_in;
            let gross_minutes = duration.num_minutes();
            let net_minutes = gross_minutes - break_minutes.max(0);
            let effective_minutes = net_minutes.max(0);
            self.total_work_hours = Some(effective_minutes as f64 / 60.0);
        }
    }

    /// Returns `true` when the record has a clock-in but no clock-out yet.
    pub fn is_clocked_in(&self) -> bool {
        self.clock_in_time.is_some() && self.clock_out_time.is_none()
    }

    /// Returns `true` once a clock-out timestamp has been recorded.
    pub fn is_clocked_out(&self) -> bool {
        self.clock_out_time.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attendance_status_serde_snake_case() {
        let s: AttendanceStatus = serde_json::from_str("\"half_day\"").unwrap();
        assert!(matches!(s, AttendanceStatus::HalfDay));
        let v = serde_json::to_value(AttendanceStatus::HalfDay).unwrap();
        assert_eq!(v, serde_json::json!("half_day"));
    }

    #[test]
    fn attendance_clock_state_helpers() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let now = Utc::now();
        let mut attendance = Attendance::new(UserId::new(), date, now);

        assert!(!attendance.is_clocked_in());
        assert!(!attendance.is_clocked_out());

        let clock_in = date.and_hms_opt(9, 0, 0).unwrap();
        attendance.clock_in_time = Some(clock_in);
        assert!(attendance.is_clocked_in());
        assert!(!attendance.is_clocked_out());

        let clock_out = date.and_hms_opt(17, 0, 0).unwrap();
        attendance.clock_out_time = Some(clock_out);
        assert!(attendance.is_clocked_out());
    }

    #[test]
    fn attendance_status_db_value_matches_schema() {
        assert_eq!(AttendanceStatus::Present.db_value(), "present");
        assert_eq!(AttendanceStatus::Absent.db_value(), "absent");
        assert_eq!(AttendanceStatus::Late.db_value(), "late");
        assert_eq!(AttendanceStatus::HalfDay.db_value(), "half_day");
    }
}
