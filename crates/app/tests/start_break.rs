use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use timekeeper_app::attendance::{
    AttendanceDay, BreakPeriod, NewBreakPeriod, StartBreak, StartBreakCommand, StartBreakError,
    StartBreakRepository,
};
use timekeeper_domain::WorkDate;

#[derive(Default)]
struct RecordingStartBreakRepository {
    attendance: Mutex<Option<AttendanceDay>>,
    active_break: Mutex<bool>,
    created: Arc<Mutex<Vec<NewBreakPeriod>>>,
}

#[async_trait::async_trait]
impl StartBreakRepository for RecordingStartBreakRepository {
    async fn find_attendance(&self, attendance_id: &str) -> Result<AttendanceDay, StartBreakError> {
        self.attendance
            .lock()
            .expect("attendance lock")
            .clone()
            .filter(|day| day.attendance_id == attendance_id)
            .ok_or(StartBreakError::AttendanceNotFound)
    }

    async fn has_active_break(&self, _attendance_id: &str) -> Result<bool, StartBreakError> {
        Ok(*self.active_break.lock().expect("active break lock"))
    }

    async fn create_break(&self, record: NewBreakPeriod) -> Result<BreakPeriod, StartBreakError> {
        self.created
            .lock()
            .expect("created lock")
            .push(record.clone());
        Ok(BreakPeriod {
            break_id: "break-created".to_string(),
            attendance_id: record.attendance_id,
            break_start_time: record.break_start_time,
            break_end_time: None,
            duration_minutes: None,
        })
    }
}

fn break_start_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 12)
        .expect("date")
        .and_hms_opt(12, 0, 0)
        .expect("break start")
}

fn recorded_at() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_781_262_000, 0).expect("recorded at")
}

fn clocked_in_day(user_id: &str) -> AttendanceDay {
    AttendanceDay {
        attendance_id: "attendance-1".to_string(),
        user_id: user_id.to_string(),
        work_date: WorkDate::from_ymd(2026, 6, 12).expect("work date"),
        clock_in_time: Some(
            NaiveDate::from_ymd_opt(2026, 6, 12)
                .expect("date")
                .and_hms_opt(9, 0, 0)
                .expect("clock in"),
        ),
        clock_out_time: None,
    }
}

fn command() -> StartBreakCommand {
    StartBreakCommand {
        user_id: "user-1".to_string(),
        attendance_id: "attendance-1".to_string(),
        break_start_time: break_start_time(),
        recorded_at: recorded_at(),
    }
}

#[tokio::test]
async fn start_break_creates_break_for_owner_who_is_clocked_in() {
    let repository = RecordingStartBreakRepository::default();
    *repository.attendance.lock().expect("attendance lock") = Some(clocked_in_day("user-1"));
    let created = Arc::clone(&repository.created);
    let use_case = StartBreak::new(repository);

    let result = use_case
        .execute(command())
        .await
        .expect("start break succeeds");

    assert_eq!(result.break_id, "break-created");
    assert_eq!(result.attendance_id, "attendance-1");
    assert_eq!(result.break_start_time, break_start_time());
    let created = created.lock().expect("created lock");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].recorded_at, recorded_at());
}

#[tokio::test]
async fn start_break_rejects_non_owner() {
    let repository = RecordingStartBreakRepository::default();
    *repository.attendance.lock().expect("attendance lock") = Some(clocked_in_day("other-user"));
    let use_case = StartBreak::new(repository);

    let error = use_case
        .execute(command())
        .await
        .expect_err("non-owner should fail");

    assert!(matches!(error, StartBreakError::Forbidden));
}

#[tokio::test]
async fn start_break_requires_clocked_in_attendance() {
    let repository = RecordingStartBreakRepository::default();
    *repository.attendance.lock().expect("attendance lock") = Some(AttendanceDay {
        clock_in_time: None,
        ..clocked_in_day("user-1")
    });
    let use_case = StartBreak::new(repository);

    let error = use_case
        .execute(command())
        .await
        .expect_err("missing clock-in should fail");

    assert!(matches!(error, StartBreakError::ClockInRequired));
}

#[tokio::test]
async fn start_break_rejects_already_clocked_out_attendance() {
    let repository = RecordingStartBreakRepository::default();
    *repository.attendance.lock().expect("attendance lock") = Some(AttendanceDay {
        clock_out_time: Some(
            NaiveDate::from_ymd_opt(2026, 6, 12)
                .expect("date")
                .and_hms_opt(18, 0, 0)
                .expect("clock out"),
        ),
        ..clocked_in_day("user-1")
    });
    let use_case = StartBreak::new(repository);

    let error = use_case
        .execute(command())
        .await
        .expect_err("clocked-out attendance should fail");

    assert!(matches!(error, StartBreakError::ClockInRequired));
}

#[tokio::test]
async fn start_break_rejects_existing_active_break_without_creating() {
    let repository = RecordingStartBreakRepository::default();
    *repository.attendance.lock().expect("attendance lock") = Some(clocked_in_day("user-1"));
    *repository.active_break.lock().expect("active break lock") = true;
    let created = Arc::clone(&repository.created);
    let use_case = StartBreak::new(repository);

    let error = use_case
        .execute(command())
        .await
        .expect_err("active break should fail");

    assert!(matches!(error, StartBreakError::ActiveBreakInProgress));
    assert!(created.lock().expect("created lock").is_empty());
}
