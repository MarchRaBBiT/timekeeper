use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use timekeeper_app::attendance::{
    AttendanceDay, AttendanceRepository, ClockIn, ClockInCommand, ClockInError, ExistingClockIn,
    HolidayCalendar, HolidayDecision, NewClockIn,
};
use timekeeper_domain::WorkDate;

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

struct FixedHolidayCalendar {
    decision: HolidayDecision,
}

#[async_trait::async_trait]
impl HolidayCalendar for FixedHolidayCalendar {
    async fn decision_for(
        &self,
        _user_id: &str,
        _work_date: WorkDate,
    ) -> Result<HolidayDecision, ClockInError> {
        Ok(self.decision.clone())
    }
}

fn working_day_calendar() -> FixedHolidayCalendar {
    FixedHolidayCalendar {
        decision: HolidayDecision::WorkingDay,
    }
}

fn holiday_calendar() -> FixedHolidayCalendar {
    FixedHolidayCalendar {
        decision: HolidayDecision::Holiday {
            reason: "public holiday".to_string(),
        },
    }
}

fn command() -> ClockInCommand {
    ClockInCommand {
        user_id: "user-1".to_string(),
        work_date: WorkDate::from_ymd(2026, 6, 12).expect("valid work date"),
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
    let use_case = ClockIn::new(repository, working_day_calendar());

    let result = use_case
        .execute(command())
        .await
        .expect("clock in succeeds");

    assert_eq!(result.user_id, "user-1");
    assert_eq!(result.clock_in_time, Some(command().clock_in_time));
    let created = created.lock().expect("created lock");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].work_date.to_string(), "2026-06-12");
    assert_eq!(created[0].recorded_at, command().recorded_at);
}

#[tokio::test]
async fn clock_in_updates_an_existing_day_without_clock_in_time() {
    let repository = RecordingAttendanceRepository::default();
    *repository.existing.lock().expect("existing lock") = Some(AttendanceDay {
        attendance_id: "attendance-1".to_string(),
        user_id: "user-1".to_string(),
        work_date: command().work_date,
        clock_in_time: None,
        clock_out_time: None,
    });
    let updated = Arc::clone(&repository.updated);
    let use_case = ClockIn::new(repository, working_day_calendar());

    use_case
        .execute(command())
        .await
        .expect("clock in succeeds");

    let updated = updated.lock().expect("updated lock");
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].attendance_id, "attendance-1");
    assert_eq!(updated[0].clock_in_time, command().clock_in_time);
}

#[tokio::test]
async fn clock_in_rejects_an_existing_clock_in_without_persisting() {
    let repository = RecordingAttendanceRepository::default();
    *repository.existing.lock().expect("existing lock") = Some(AttendanceDay {
        attendance_id: "attendance-1".to_string(),
        user_id: "user-1".to_string(),
        work_date: command().work_date,
        clock_in_time: Some(command().clock_in_time),
        clock_out_time: None,
    });
    let created = Arc::clone(&repository.created);
    let updated = Arc::clone(&repository.updated);
    let use_case = ClockIn::new(repository, working_day_calendar());

    let error = use_case
        .execute(command())
        .await
        .expect_err("existing clock in should fail");

    assert!(matches!(error, ClockInError::AlreadyClockedIn));
    assert!(created.lock().expect("created lock").is_empty());
    assert!(updated.lock().expect("updated lock").is_empty());
}

#[tokio::test]
async fn clock_in_rejects_holidays_before_repository_lookup() {
    let repository = RecordingAttendanceRepository::default();
    let use_case = ClockIn::new(repository, holiday_calendar());

    let error = use_case
        .execute(command())
        .await
        .expect_err("holiday should fail");

    assert!(matches!(error, ClockInError::Holiday { .. }));
    assert_eq!(
        *use_case
            .repository()
            .find_calls
            .lock()
            .expect("find calls lock"),
        0
    );
}
