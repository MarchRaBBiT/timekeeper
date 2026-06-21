use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use timekeeper_app::attendance::{
    AttendanceDay, ClockOut, ClockOutCommand, ClockOutError, ClockOutRepository, ExistingClockOut,
};
use timekeeper_domain::WorkDate;

#[derive(Default)]
struct RecordingClockOutRepository {
    existing: Mutex<Option<AttendanceDay>>,
    active_break: Mutex<bool>,
    break_minutes: Mutex<i64>,
    updated: Arc<Mutex<Vec<ExistingClockOut>>>,
}

#[async_trait::async_trait]
impl ClockOutRepository for RecordingClockOutRepository {
    async fn find_for_clock_out(
        &self,
        _user_id: &str,
        _requested_work_date: Option<WorkDate>,
    ) -> Result<Option<AttendanceDay>, ClockOutError> {
        Ok(self.existing.lock().expect("existing lock").clone())
    }

    async fn has_active_break(&self, _attendance_id: &str) -> Result<bool, ClockOutError> {
        Ok(*self.active_break.lock().expect("active break lock"))
    }

    async fn total_break_minutes(&self, _attendance_id: &str) -> Result<i64, ClockOutError> {
        Ok(*self.break_minutes.lock().expect("break minutes lock"))
    }

    async fn update_clock_out(
        &self,
        record: ExistingClockOut,
    ) -> Result<AttendanceDay, ClockOutError> {
        self.updated
            .lock()
            .expect("updated lock")
            .push(record.clone());
        Ok(AttendanceDay {
            attendance_id: record.attendance_id,
            user_id: record.user_id,
            work_date: record.work_date,
            clock_in_time: Some(record.clock_in_time),
            clock_out_time: Some(record.clock_out_time),
        })
    }
}

fn clock_in_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 12)
        .expect("date")
        .and_hms_opt(9, 0, 0)
        .expect("clock in")
}

fn clock_out_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 12)
        .expect("date")
        .and_hms_opt(18, 0, 0)
        .expect("clock out")
}

fn command() -> ClockOutCommand {
    ClockOutCommand {
        user_id: "user-1".to_string(),
        requested_work_date: Some(WorkDate::from_ymd(2026, 6, 12).expect("valid work date")),
        clock_out_time: clock_out_time(),
        recorded_at: DateTime::<Utc>::from_timestamp(1_781_283_600, 0).expect("recorded at"),
    }
}

fn clocked_in_day() -> AttendanceDay {
    AttendanceDay {
        attendance_id: "attendance-1".to_string(),
        user_id: "user-1".to_string(),
        work_date: command().requested_work_date.expect("work date"),
        clock_in_time: Some(clock_in_time()),
        clock_out_time: None,
    }
}

#[tokio::test]
async fn clock_out_updates_existing_clocked_in_day_with_net_work_hours() {
    let repository = RecordingClockOutRepository::default();
    *repository.existing.lock().expect("existing lock") = Some(clocked_in_day());
    *repository.break_minutes.lock().expect("break minutes lock") = 60;
    let updated = Arc::clone(&repository.updated);
    let use_case = ClockOut::new(repository);

    let result = use_case
        .execute(command())
        .await
        .expect("clock out succeeds");

    assert_eq!(result.clock_out_time, Some(clock_out_time()));
    let updated = updated.lock().expect("updated lock");
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].attendance_id, "attendance-1");
    assert_eq!(updated[0].total_work_hours, Some(8.0));
}

#[tokio::test]
async fn clock_out_rejects_missing_attendance_record() {
    let use_case = ClockOut::new(RecordingClockOutRepository::default());

    let error = use_case
        .execute(command())
        .await
        .expect_err("missing attendance should fail");

    assert!(matches!(error, ClockOutError::AttendanceNotFound));
}

#[tokio::test]
async fn clock_out_rejects_missing_clock_in_time() {
    let repository = RecordingClockOutRepository::default();
    *repository.existing.lock().expect("existing lock") = Some(AttendanceDay {
        clock_in_time: None,
        ..clocked_in_day()
    });
    let use_case = ClockOut::new(repository);

    let error = use_case
        .execute(command())
        .await
        .expect_err("missing clock in should fail");

    assert!(matches!(error, ClockOutError::ClockInRequired));
}

#[tokio::test]
async fn clock_out_rejects_duplicate_clock_out() {
    let repository = RecordingClockOutRepository::default();
    *repository.existing.lock().expect("existing lock") = Some(AttendanceDay {
        clock_out_time: Some(clock_out_time()),
        ..clocked_in_day()
    });
    let use_case = ClockOut::new(repository);

    let error = use_case
        .execute(command())
        .await
        .expect_err("duplicate clock out should fail");

    assert!(matches!(error, ClockOutError::AlreadyClockedOut));
}

#[tokio::test]
async fn clock_out_rejects_active_break_without_updating() {
    let repository = RecordingClockOutRepository::default();
    *repository.existing.lock().expect("existing lock") = Some(clocked_in_day());
    *repository.active_break.lock().expect("active break lock") = true;
    let updated = Arc::clone(&repository.updated);
    let use_case = ClockOut::new(repository);

    let error = use_case
        .execute(command())
        .await
        .expect_err("active break should fail");

    assert!(matches!(error, ClockOutError::ActiveBreakInProgress));
    assert!(updated.lock().expect("updated lock").is_empty());
}

#[tokio::test]
async fn clock_out_without_requested_date_uses_the_open_attendance_workday() {
    let repository = RecordingClockOutRepository::default();
    *repository.existing.lock().expect("existing lock") = Some(clocked_in_day());
    let use_case = ClockOut::new(repository);
    let mut command = command();
    command.requested_work_date = None;

    let result = use_case
        .execute(command)
        .await
        .expect("open attendance is used");

    assert_eq!(result.work_date.to_string(), "2026-06-12");
}
