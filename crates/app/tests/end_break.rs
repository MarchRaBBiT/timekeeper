use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use timekeeper_app::attendance::{
    AttendanceDay, BreakEnd, BreakEndCommand, BreakEndError, BreakEndRepository, BreakPeriod,
    EndedBreakPeriod, ForceEndBreak, ForceEndBreakCommand,
};
use timekeeper_domain::WorkDate;

#[derive(Default)]
struct RecordingBreakEndRepository {
    break_period: Mutex<Option<BreakPeriod>>,
    attendance: Mutex<Option<AttendanceDay>>,
    updated: Arc<Mutex<Vec<EndedBreakPeriod>>>,
    recalculated: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl BreakEndRepository for RecordingBreakEndRepository {
    async fn find_break(&self, break_id: &str) -> Result<BreakPeriod, BreakEndError> {
        self.break_period
            .lock()
            .expect("break lock")
            .clone()
            .filter(|period| period.break_id == break_id)
            .ok_or(BreakEndError::BreakNotFound)
    }

    async fn find_attendance(&self, attendance_id: &str) -> Result<AttendanceDay, BreakEndError> {
        self.attendance
            .lock()
            .expect("attendance lock")
            .clone()
            .filter(|day| day.attendance_id == attendance_id)
            .ok_or(BreakEndError::AttendanceNotFound)
    }

    async fn update_break(&self, record: EndedBreakPeriod) -> Result<BreakPeriod, BreakEndError> {
        self.updated
            .lock()
            .expect("updated lock")
            .push(record.clone());
        Ok(BreakPeriod {
            break_id: record.break_id,
            attendance_id: record.attendance_id,
            break_start_time: record.break_start_time,
            break_end_time: Some(record.break_end_time),
            duration_minutes: Some(record.duration_minutes),
        })
    }

    async fn recalculate_total_hours(
        &self,
        attendance_id: &str,
        _recorded_at: DateTime<Utc>,
    ) -> Result<(), BreakEndError> {
        self.recalculated
            .lock()
            .expect("recalculated lock")
            .push(attendance_id.to_string());
        Ok(())
    }
}

fn break_start_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 12)
        .expect("date")
        .and_hms_opt(12, 0, 0)
        .expect("break start")
}

fn break_end_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 12)
        .expect("date")
        .and_hms_opt(12, 45, 0)
        .expect("break end")
}

fn recorded_at() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_781_264_700, 0).expect("recorded at")
}

fn active_break() -> BreakPeriod {
    BreakPeriod {
        break_id: "break-1".to_string(),
        attendance_id: "attendance-1".to_string(),
        break_start_time: break_start_time(),
        break_end_time: None,
        duration_minutes: None,
    }
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

fn command() -> BreakEndCommand {
    BreakEndCommand {
        user_id: "user-1".to_string(),
        break_id: "break-1".to_string(),
        break_end_time: break_end_time(),
        recorded_at: recorded_at(),
    }
}

fn force_command() -> ForceEndBreakCommand {
    ForceEndBreakCommand {
        break_id: "break-1".to_string(),
        break_end_time: break_end_time(),
        recorded_at: recorded_at(),
    }
}

#[tokio::test]
async fn end_break_closes_active_break_for_owner() {
    let repository = RecordingBreakEndRepository::default();
    *repository.break_period.lock().expect("break lock") = Some(active_break());
    *repository.attendance.lock().expect("attendance lock") = Some(clocked_in_day("user-1"));
    let updated = Arc::clone(&repository.updated);
    let recalculated = Arc::clone(&repository.recalculated);
    let use_case = BreakEnd::new(repository);

    let result = use_case
        .execute(command())
        .await
        .expect("end break succeeds");

    assert_eq!(result.break_id, "break-1");
    assert_eq!(result.break_end_time, Some(break_end_time()));
    assert_eq!(result.duration_minutes, Some(45));
    let updated = updated.lock().expect("updated lock");
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].duration_minutes, 45);
    assert!(recalculated.lock().expect("recalculated lock").is_empty());
}

#[tokio::test]
async fn end_break_rejects_non_owner() {
    let repository = RecordingBreakEndRepository::default();
    *repository.break_period.lock().expect("break lock") = Some(active_break());
    *repository.attendance.lock().expect("attendance lock") = Some(clocked_in_day("other-user"));
    let use_case = BreakEnd::new(repository);

    let error = use_case
        .execute(command())
        .await
        .expect_err("non-owner should fail");

    assert!(matches!(error, BreakEndError::Forbidden));
}

#[tokio::test]
async fn end_break_rejects_already_ended_break() {
    let repository = RecordingBreakEndRepository::default();
    *repository.break_period.lock().expect("break lock") = Some(BreakPeriod {
        break_end_time: Some(break_end_time()),
        duration_minutes: Some(45),
        ..active_break()
    });
    let use_case = BreakEnd::new(repository);

    let error = use_case
        .execute(command())
        .await
        .expect_err("already-ended break should fail");

    assert!(matches!(error, BreakEndError::BreakAlreadyEnded));
}

#[tokio::test]
async fn end_break_recalculates_total_hours_when_attendance_is_clocked_out() {
    let repository = RecordingBreakEndRepository::default();
    *repository.break_period.lock().expect("break lock") = Some(active_break());
    *repository.attendance.lock().expect("attendance lock") = Some(AttendanceDay {
        clock_out_time: Some(
            NaiveDate::from_ymd_opt(2026, 6, 12)
                .expect("date")
                .and_hms_opt(18, 0, 0)
                .expect("clock out"),
        ),
        ..clocked_in_day("user-1")
    });
    let recalculated = Arc::clone(&repository.recalculated);
    let use_case = BreakEnd::new(repository);

    use_case
        .execute(command())
        .await
        .expect("end break succeeds");

    let recalculated = recalculated.lock().expect("recalculated lock");
    assert_eq!(recalculated.as_slice(), ["attendance-1"]);
}

#[tokio::test]
async fn force_end_break_closes_active_break_without_owner_check() {
    let repository = RecordingBreakEndRepository::default();
    *repository.break_period.lock().expect("break lock") = Some(active_break());
    *repository.attendance.lock().expect("attendance lock") = Some(clocked_in_day("other-user"));
    let use_case = ForceEndBreak::new(repository);

    let result = use_case
        .execute(force_command())
        .await
        .expect("force end succeeds");

    assert_eq!(result.break_id, "break-1");
    assert_eq!(result.break_end_time, Some(break_end_time()));
    assert_eq!(result.duration_minutes, Some(45));
}

#[tokio::test]
async fn force_end_break_rejects_already_ended_break() {
    let repository = RecordingBreakEndRepository::default();
    *repository.break_period.lock().expect("break lock") = Some(BreakPeriod {
        break_end_time: Some(break_end_time()),
        duration_minutes: Some(45),
        ..active_break()
    });
    let use_case = ForceEndBreak::new(repository);

    let error = use_case
        .execute(force_command())
        .await
        .expect_err("already-ended break should fail");

    assert!(matches!(error, BreakEndError::BreakAlreadyEnded));
}
