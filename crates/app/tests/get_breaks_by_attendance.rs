use std::sync::Mutex;

use chrono::{NaiveDate, NaiveDateTime};
use timekeeper_app::attendance::{
    AttendanceDay, BreakPeriod, GetBreaksByAttendance, GetBreaksByAttendanceError,
    GetBreaksByAttendanceQuery, GetBreaksByAttendanceRepository,
};
use timekeeper_domain::WorkDate;

#[derive(Default)]
struct RecordingBreaksRepository {
    attendance: Mutex<Option<AttendanceDay>>,
    breaks: Mutex<Vec<BreakPeriod>>,
}

#[async_trait::async_trait]
impl GetBreaksByAttendanceRepository for RecordingBreaksRepository {
    async fn find_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<AttendanceDay, GetBreaksByAttendanceError> {
        self.attendance
            .lock()
            .expect("attendance lock")
            .clone()
            .filter(|attendance| attendance.attendance_id == attendance_id)
            .ok_or(GetBreaksByAttendanceError::AttendanceNotFound)
    }

    async fn breaks_for_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<Vec<BreakPeriod>, GetBreaksByAttendanceError> {
        Ok(self
            .breaks
            .lock()
            .expect("breaks lock")
            .iter()
            .filter(|period| period.attendance_id == attendance_id)
            .cloned()
            .collect())
    }
}

fn work_date() -> WorkDate {
    WorkDate::from_ymd(2026, 6, 12).expect("work date")
}

fn clock_in_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 12)
        .expect("date")
        .and_hms_opt(9, 0, 0)
        .expect("clock in")
}

fn break_start_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 12)
        .expect("date")
        .and_hms_opt(12, 0, 0)
        .expect("break start")
}

fn attendance(user_id: &str) -> AttendanceDay {
    AttendanceDay {
        attendance_id: "attendance-1".to_string(),
        user_id: user_id.to_string(),
        work_date: work_date(),
        clock_in_time: Some(clock_in_time()),
        clock_out_time: None,
    }
}

fn break_period(id: &str, attendance_id: &str) -> BreakPeriod {
    BreakPeriod {
        break_id: id.to_string(),
        attendance_id: attendance_id.to_string(),
        break_start_time: break_start_time(),
        break_end_time: None,
        duration_minutes: None,
    }
}

fn query() -> GetBreaksByAttendanceQuery {
    GetBreaksByAttendanceQuery {
        user_id: "user-1".to_string(),
        attendance_id: "attendance-1".to_string(),
    }
}

#[tokio::test]
async fn lists_breaks_for_attendance_owner() {
    let repository = RecordingBreaksRepository::default();
    *repository.attendance.lock().expect("attendance lock") = Some(attendance("user-1"));
    *repository.breaks.lock().expect("breaks lock") = vec![
        break_period("break-1", "attendance-1"),
        break_period("break-other", "attendance-other"),
    ];
    let use_case = GetBreaksByAttendance::new(repository);

    let breaks = use_case.execute(query()).await.expect("list succeeds");

    assert_eq!(breaks.len(), 1);
    assert_eq!(breaks[0].break_id, "break-1");
}

#[tokio::test]
async fn rejects_non_owner() {
    let repository = RecordingBreaksRepository::default();
    *repository.attendance.lock().expect("attendance lock") = Some(attendance("other-user"));
    let use_case = GetBreaksByAttendance::new(repository);

    let error = use_case
        .execute(query())
        .await
        .expect_err("non-owner should fail");

    assert!(matches!(error, GetBreaksByAttendanceError::Forbidden));
}

#[tokio::test]
async fn returns_not_found_for_missing_attendance() {
    let use_case = GetBreaksByAttendance::new(RecordingBreaksRepository::default());

    let error = use_case
        .execute(query())
        .await
        .expect_err("missing attendance should fail");

    assert!(matches!(
        error,
        GetBreaksByAttendanceError::AttendanceNotFound
    ));
}
