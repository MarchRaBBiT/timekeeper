use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use timekeeper_app::attendance::{
    AttendanceDay, AttendanceRepository, ClockIn, ClockInCommand, ClockInError, ClockInWorkday,
    ClockInWorkdayResolver, ExistingClockIn, NewClockIn,
};
use timekeeper_domain::{work_schedules::ResolvedDayKind, WorkDate};

#[derive(Default)]
struct RecordingAttendanceRepository {
    existing: Mutex<Option<AttendanceDay>>,
    find_calls: Mutex<usize>,
    created: Arc<Mutex<Vec<NewClockIn>>>,
    updated: Arc<Mutex<Vec<ExistingClockIn>>>,
}

#[async_trait::async_trait]
impl AttendanceRepository for RecordingAttendanceRepository {
    async fn find_by_user_and_date(
        &self,
        _user_id: &str,
        _work_date: WorkDate,
    ) -> Result<Option<AttendanceDay>, ClockInError> {
        *self.find_calls.lock().expect("find calls lock") += 1;
        Ok(self.existing.lock().expect("existing lock").clone())
    }

    async fn create_clock_in(&self, record: NewClockIn) -> Result<AttendanceDay, ClockInError> {
        self.created
            .lock()
            .expect("created lock")
            .push(record.clone());
        Ok(AttendanceDay {
            attendance_id: "attendance-created".to_string(),
            user_id: record.user_id,
            work_date: record.work_date,
            clock_in_time: Some(record.clock_in_time),
            clock_out_time: None,
        })
    }

    async fn update_clock_in(
        &self,
        record: ExistingClockIn,
    ) -> Result<AttendanceDay, ClockInError> {
        self.updated
            .lock()
            .expect("updated lock")
            .push(record.clone());
        Ok(AttendanceDay {
            attendance_id: record.attendance_id,
            user_id: record.user_id,
            work_date: record.work_date,
            clock_in_time: Some(record.clock_in_time),
            clock_out_time: record.clock_out_time,
        })
    }
}

struct FixedWorkdayResolver {
    workday: ClockInWorkday,
    calls: Arc<Mutex<usize>>,
}

#[async_trait::async_trait]
impl ClockInWorkdayResolver for FixedWorkdayResolver {
    async fn resolve_for_punch(
        &self,
        _user_id: &str,
        _requested_work_date: Option<WorkDate>,
        _punch_time: chrono::NaiveDateTime,
        _resolved_at: DateTime<Utc>,
    ) -> Result<ClockInWorkday, ClockInError> {
        *self.calls.lock().expect("calls lock") += 1;
        Ok(self.workday.clone())
    }
}

fn workday_resolver(day_kind: ResolvedDayKind) -> FixedWorkdayResolver {
    FixedWorkdayResolver {
        workday: ClockInWorkday {
            resolved_workday_id: "resolved-1".to_string(),
            work_date: WorkDate::from_ymd(2026, 6, 12).expect("valid work date"),
            day_kind,
        },
        calls: Arc::new(Mutex::new(0)),
    }
}

fn command() -> ClockInCommand {
    ClockInCommand {
        user_id: "user-1".to_string(),
        requested_work_date: Some(WorkDate::from_ymd(2026, 6, 12).expect("valid work date")),
        clock_in_time: NaiveDate::from_ymd_opt(2026, 6, 12)
            .expect("date")
            .and_hms_opt(9, 0, 0)
            .expect("clock in"),
        recorded_at: DateTime::<Utc>::from_timestamp(1_781_251_200, 0).expect("recorded at"),
    }
}

#[tokio::test]
async fn clock_in_creates_a_record_when_the_day_does_not_exist() {
    let repository = RecordingAttendanceRepository::default();
    let created = Arc::clone(&repository.created);
    let use_case = ClockIn::new(
        repository,
        workday_resolver(ResolvedDayKind::ScheduledWorkday),
    );

    let result = use_case
        .execute(command())
        .await
        .expect("clock in succeeds");

    assert_eq!(result.user_id, "user-1");
    assert_eq!(result.clock_in_time, Some(command().clock_in_time));
    let created = created.lock().expect("created lock");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].work_date.to_string(), "2026-06-12");
    assert_eq!(created[0].resolved_workday_id, "resolved-1");
    assert_eq!(created[0].recorded_at, command().recorded_at);
}

#[tokio::test]
async fn clock_in_updates_an_existing_day_without_clock_in_time() {
    let repository = RecordingAttendanceRepository::default();
    *repository.existing.lock().expect("existing lock") = Some(AttendanceDay {
        attendance_id: "attendance-1".to_string(),
        user_id: "user-1".to_string(),
        work_date: command().requested_work_date.expect("work date"),
        clock_in_time: None,
        clock_out_time: None,
    });
    let updated = Arc::clone(&repository.updated);
    let use_case = ClockIn::new(
        repository,
        workday_resolver(ResolvedDayKind::ScheduledWorkday),
    );

    use_case
        .execute(command())
        .await
        .expect("clock in succeeds");

    let updated = updated.lock().expect("updated lock");
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].attendance_id, "attendance-1");
    assert_eq!(updated[0].clock_in_time, command().clock_in_time);
    assert_eq!(updated[0].resolved_workday_id, "resolved-1");
}

#[tokio::test]
async fn clock_in_rejects_an_existing_clock_in_without_persisting() {
    let repository = RecordingAttendanceRepository::default();
    *repository.existing.lock().expect("existing lock") = Some(AttendanceDay {
        attendance_id: "attendance-1".to_string(),
        user_id: "user-1".to_string(),
        work_date: command().requested_work_date.expect("work date"),
        clock_in_time: Some(command().clock_in_time),
        clock_out_time: None,
    });
    let created = Arc::clone(&repository.created);
    let updated = Arc::clone(&repository.updated);
    let resolver = workday_resolver(ResolvedDayKind::ScheduledWorkday);
    let calls = Arc::clone(&resolver.calls);
    let use_case = ClockIn::new(repository, resolver);

    let error = use_case
        .execute(command())
        .await
        .expect_err("existing clock in should fail");

    assert!(matches!(error, ClockInError::AlreadyClockedIn));
    assert!(created.lock().expect("created lock").is_empty());
    assert!(updated.lock().expect("updated lock").is_empty());
    assert_eq!(*calls.lock().expect("calls lock"), 1);
}

#[tokio::test]
async fn clock_in_allows_public_holiday_and_links_the_resolved_workday() {
    let repository = RecordingAttendanceRepository::default();
    let created = Arc::clone(&repository.created);
    let use_case = ClockIn::new(repository, workday_resolver(ResolvedDayKind::PublicHoliday));

    let result = use_case
        .execute(command())
        .await
        .expect("holiday punch is an attendance fact");

    assert_eq!(result.work_date.to_string(), "2026-06-12");
    assert_eq!(
        created.lock().expect("created lock")[0].resolved_workday_id,
        "resolved-1"
    );
}
