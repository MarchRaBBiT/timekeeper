use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BreakRecord {
    pub id: String,
    pub attendance_id: String,
    pub break_start_time: NaiveDateTime,
    pub break_end_time: Option<NaiveDateTime>,
    pub duration_minutes: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BreakRecordResponse {
    pub id: String,
    pub attendance_id: String,
    pub break_start_time: NaiveDateTime,
    pub break_end_time: Option<NaiveDateTime>,
    pub duration_minutes: Option<i32>,
}

impl From<BreakRecord> for BreakRecordResponse {
    fn from(record: BreakRecord) -> Self {
        BreakRecordResponse {
            id: record.id,
            attendance_id: record.attendance_id,
            break_start_time: record.break_start_time,
            break_end_time: record.break_end_time,
            duration_minutes: record.duration_minutes,
        }
    }
}

impl BreakRecord {
    pub fn new(attendance_id: String, break_start_time: NaiveDateTime, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            attendance_id,
            break_start_time,
            break_end_time: None,
            duration_minutes: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn end_break(&mut self, break_end_time: NaiveDateTime, now: DateTime<Utc>) {
        self.break_end_time = Some(break_end_time);
        let duration = break_end_time - self.break_start_time;
        self.duration_minutes = Some(duration.num_minutes() as i32);
        self.updated_at = now;
    }

    pub fn is_active(&self) -> bool {
        self.break_end_time.is_none()
    }
}
