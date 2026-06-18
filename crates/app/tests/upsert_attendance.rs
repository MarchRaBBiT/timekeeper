use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use timekeeper_app::attendance::{
    AttendancePageItem, AttendanceRecord, AttendanceReplacement, BreakPeriod, UpsertAttendance,
    UpsertAttendanceCommand, UpsertAttendanceError, UpsertAttendanceRepository, UpsertBreakInput,
};

#[derive(Default)]
struct RecordingUpsertRepository {
    replacement: Mutex<Option<AttendanceReplacement>>,
}

#[async_trait::async_trait]
impl UpsertAttendanceRepository for RecordingUpsertRepository {
    async fn replace_attendance(
        &self,
        replacement: AttendanceReplacement,
    ) -> Result<AttendancePageItem, UpsertAttendanceError> {
        *self.replacement.lock().expect("replacement lock") = Some(replacement.clone());
        Ok(AttendancePageItem {
            attendance: AttendanceRecord {
                attendance_id: "attendance-1".to_string(),
                user_id: replacement.user_id,
                date: replacement.date,
                clock_in_time: Some(replacement.clock_in_time),
                clock_out_time: replacement.clock_out_time,
                status: "present".to_string(),
                total_work_hours: replacement.total_work_hours,
            },
            break_periods: replacement
                .breaks
                .into_iter()
                .enumerate()
                .map(|(index, break_period)| BreakPeriod {
                    break_id: format!("break-{index}"),
                    attendance_id: "attendance-1".to_string(),
                    break_start_time: break_period.break_start_time,
                    break_end_time: break_period.break_end_time,
                    duration_minutes: break_period.duration_minutes,
                })
                .collect(),
        })
    }
}

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, 12).expect("date")
}

fn time(hour: u32) -> NaiveDateTime {
    date().and_hms_opt(hour, 0, 0).expect("time")
}

fn recorded_at() -> DateTime<Utc> {
    DateTime::from_naive_utc_and_offset(time(9), Utc)
}

#[tokio::test]
async fn computes_total_hours_after_completed_breaks() {
    let repository = RecordingUpsertRepository::default();
    let use_case = UpsertAttendance::new(repository);

    let item = use_case
        .execute(UpsertAttendanceCommand {
            user_id: "user-1".to_string(),
            date: date(),
            clock_in_time: time(9),
            clock_out_time: Some(time(18)),
            breaks: vec![UpsertBreakInput {
                break_start_time: time(12),
                break_end_time: Some(time(13)),
            }],
            recorded_at: recorded_at(),
        })
        .await
        .expect("upsert succeeds");

    assert_eq!(item.attendance.total_work_hours, Some(8.0));
    assert_eq!(item.break_periods[0].duration_minutes, Some(60));
}

#[tokio::test]
async fn clamps_negative_break_duration_to_zero() {
    let repository = RecordingUpsertRepository::default();
    let use_case = UpsertAttendance::new(repository);

    let item = use_case
        .execute(UpsertAttendanceCommand {
            user_id: "user-1".to_string(),
            date: date(),
            clock_in_time: time(9),
            clock_out_time: Some(time(18)),
            breaks: vec![UpsertBreakInput {
                break_start_time: time(13),
                break_end_time: Some(time(12)),
            }],
            recorded_at: recorded_at(),
        })
        .await
        .expect("upsert succeeds");

    assert_eq!(item.attendance.total_work_hours, Some(9.0));
    assert_eq!(item.break_periods[0].duration_minutes, Some(0));
}
