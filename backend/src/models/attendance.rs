use crate::models::break_record::BreakRecordResponse;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Attendance {
    pub id: String,
    pub user_id: String,
    pub date: NaiveDate,
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
    pub status: AttendanceStatus,
    pub total_work_hours: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AttendanceStatus {
    Present,
    Absent,
    Late,
    HalfDay,
}

impl Default for AttendanceStatus {
    fn default() -> Self {
        AttendanceStatus::Present
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClockInRequest {
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClockOutRequest {
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BreakStartRequest {
    pub attendance_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BreakEndRequest {
    pub break_record_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttendanceResponse {
    pub id: String,
    pub user_id: String,
    pub date: NaiveDate,
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
    pub status: AttendanceStatus,
    pub total_work_hours: Option<f64>,
    pub break_records: Vec<BreakRecordResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttendanceSummary {
    pub month: u32,
    pub year: i32,
    pub total_work_hours: f64,
    pub total_work_days: i32,
    pub average_daily_hours: f64,
}

impl Attendance {
    pub fn new(user_id: String, date: NaiveDate, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
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

    pub fn calculate_work_hours(&mut self) {
        if let (Some(clock_in), Some(clock_out)) = (self.clock_in_time, self.clock_out_time) {
            let duration = clock_out - clock_in;
            self.total_work_hours = Some(duration.num_minutes() as f64 / 60.0);
        }
    }

    pub fn is_clocked_in(&self) -> bool {
        self.clock_in_time.is_some() && self.clock_out_time.is_none()
    }

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
}
