//! Models that capture break sessions within an attendance record.

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
/// Persistent representation of a single break interval.
pub struct BreakRecord {
    /// Unique identifier for the break record.
    pub id: String,
    /// Associated attendance record identifier.
    pub attendance_id: String,
    /// Timestamp when the break started.
    pub break_start_time: NaiveDateTime,
    /// Timestamp when the break ended, if the break is closed.
    pub break_end_time: Option<NaiveDateTime>,
    /// Duration of the break in minutes, filled when the break ends.
    pub duration_minutes: Option<i32>,
    /// Creation timestamp for auditing.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp for auditing.
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
/// API-friendly representation of a break interval.
pub struct BreakRecordResponse {
    pub id: String,
    pub attendance_id: String,
    pub break_start_time: NaiveDateTime,
    pub break_end_time: Option<NaiveDateTime>,
    pub duration_minutes: Option<i32>,
}

impl From<BreakRecord> for BreakRecordResponse {
    /// Converts a database model into the response payload variant.
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
    /// Creates a new break record that starts immediately.
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

    /// Marks the break as completed and computes its duration.
    pub fn end_break(&mut self, break_end_time: NaiveDateTime, now: DateTime<Utc>) {
        self.break_end_time = Some(break_end_time);
        let duration = break_end_time - self.break_start_time;
        self.duration_minutes = Some(duration.num_minutes() as i32);
        self.updated_at = now;
    }

    /// Returns `true` while the break is still active.
    pub fn is_active(&self) -> bool {
        self.break_end_time.is_none()
    }
}
