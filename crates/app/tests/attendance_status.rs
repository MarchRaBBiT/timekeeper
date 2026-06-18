use std::sync::Mutex;

use chrono::{NaiveDate, NaiveDateTime};
use timekeeper_app::attendance::{
    AttendanceDay, AttendanceStatusError, AttendanceStatusQuery, AttendanceStatusReadRepository,
    GetAttendanceStatus,
};
use timekeeper_domain::WorkDate;

#[derive(Default)]
struct RecordingStatusRepository {
    attendance: Mutex<Option<AttendanceDay>>,
    active_break_id: Mutex<Option<String>>,
}

#[async_trait::async_trait]
impl AttendanceStatusReadRepository for RecordingStatusRepository {
    async fn find_by_user_and_date(
        &self,
        user_id: &str,
        work_date: WorkDate,
    ) -> Result<Option<AttendanceDay>, AttendanceStatusError> {
        Ok(self
            .attendance
            .lock()
            .expect("attendance lock")
            .clone()
            .filter(|day| day.user_id == user_id && day.work_date == work_date))
    }

    async fn active_break_id(
        &self,
        attendance_id: &str,
    ) -> Result<Option<String>, AttendanceStatusError> {
        let active_break_id = self.active_break_id.lock().expect("active break lock");
        Ok(active_break_id
            .clone()
            .filter(|_| attendance_id == "attendance-1"))
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

fn clock_out_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 12)
        .expect("date")
        .and_hms_opt(18, 0, 0)
        .expect("clock out")
}

fn attendance(clock_in_time: Option<NaiveDateTime>) -> AttendanceDay {
    AttendanceDay {
        attendance_id: "attendance-1".to_string(),
        user_id: "user-1".to_string(),
        work_date: work_date(),
        clock_in_time,
        clock_out_time: None,
    }
}

fn query() -> AttendanceStatusQuery {
    AttendanceStatusQuery {
        user_id: "user-1".to_string(),
        work_date: work_date(),
    }
}

#[tokio::test]
async fn status_is_not_started_when_no_attendance_exists() {
    let use_case = GetAttendanceStatus::new(RecordingStatusRepository::default());

    let status = use_case.execute(query()).await.expect("status succeeds");

    assert_eq!(status.status, "not_started");
    assert_eq!(status.attendance_id, None);
    assert_eq!(status.active_break_id, None);
}

#[tokio::test]
async fn status_is_not_started_when_attendance_has_no_clock_in() {
    let repository = RecordingStatusRepository::default();
    *repository.attendance.lock().expect("attendance lock") = Some(attendance(None));
    let use_case = GetAttendanceStatus::new(repository);

    let status = use_case.execute(query()).await.expect("status succeeds");

    assert_eq!(status.status, "not_started");
    assert_eq!(status.attendance_id.as_deref(), Some("attendance-1"));
    assert_eq!(status.clock_in_time, None);
}

#[tokio::test]
async fn status_is_clocked_out_when_clock_out_exists() {
    let repository = RecordingStatusRepository::default();
    *repository.attendance.lock().expect("attendance lock") = Some(AttendanceDay {
        clock_out_time: Some(clock_out_time()),
        ..attendance(Some(clock_in_time()))
    });
    let use_case = GetAttendanceStatus::new(repository);

    let status = use_case.execute(query()).await.expect("status succeeds");

    assert_eq!(status.status, "clocked_out");
    assert_eq!(status.clock_in_time, Some(clock_in_time()));
    assert_eq!(status.clock_out_time, Some(clock_out_time()));
}

#[tokio::test]
async fn status_is_on_break_when_active_break_exists() {
    let repository = RecordingStatusRepository::default();
    *repository.attendance.lock().expect("attendance lock") =
        Some(attendance(Some(clock_in_time())));
    *repository
        .active_break_id
        .lock()
        .expect("active break lock") = Some("break-1".to_string());
    let use_case = GetAttendanceStatus::new(repository);

    let status = use_case.execute(query()).await.expect("status succeeds");

    assert_eq!(status.status, "on_break");
    assert_eq!(status.active_break_id.as_deref(), Some("break-1"));
    assert_eq!(status.clock_out_time, None);
}

#[tokio::test]
async fn status_is_clocked_in_when_clocked_in_without_active_break() {
    let repository = RecordingStatusRepository::default();
    *repository.attendance.lock().expect("attendance lock") =
        Some(attendance(Some(clock_in_time())));
    let use_case = GetAttendanceStatus::new(repository);

    let status = use_case.execute(query()).await.expect("status succeeds");

    assert_eq!(status.status, "clocked_in");
    assert_eq!(status.active_break_id, None);
    assert_eq!(status.clock_in_time, Some(clock_in_time()));
}
