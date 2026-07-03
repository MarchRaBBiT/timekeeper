use std::sync::Mutex;

use chrono::{NaiveDate, NaiveTime, Utc};
use timekeeper_app::user_workdays::{
    ListUserWorkdays, ListUserWorkdaysCommand, ListUserWorkdaysError, UserWorkdayReadRepository,
    MAX_WORKDAY_RANGE_DAYS,
};
use timekeeper_app::work_schedules::{
    ResolvedDayKind, ResolvedWorkday, ScheduleType, WorkScheduleSource,
};

#[derive(Default)]
struct FakeReadRepository {
    calls: Mutex<Vec<(String, NaiveDate, NaiveDate)>>,
    rows: Mutex<Vec<ResolvedWorkday>>,
}

#[async_trait::async_trait]
impl UserWorkdayReadRepository for FakeReadRepository {
    async fn list_resolved_in_range(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ResolvedWorkday>, ListUserWorkdaysError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push((user_id.to_string(), from, to));
        Ok(self.rows.lock().expect("rows lock").clone())
    }
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}

fn sample_workday(work_date: NaiveDate) -> ResolvedWorkday {
    ResolvedWorkday {
        id: "rw-1".to_string(),
        user_id: "user-1".to_string(),
        work_date,
        work_schedule_id: "ws-1".to_string(),
        work_schedule_version_id: "wsv-1".to_string(),
        source: WorkScheduleSource::User,
        source_id: "assignment-1".to_string(),
        day_kind: ResolvedDayKind::ScheduledWorkday,
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: NaiveTime::from_hms_opt(5, 0, 0).expect("boundary"),
        expected_work_minutes: 480,
        work_intervals: Vec::new(),
        planned_breaks: Vec::new(),
        schedule_type: ScheduleType::Fixed,
        core_time_windows: Vec::new(),
        resolved_at: Utc::now(),
        locked_at: None,
    }
}

#[tokio::test]
async fn returns_rows_for_a_valid_range() {
    let repository = FakeReadRepository::default();
    *repository.rows.lock().expect("rows lock") = vec![sample_workday(date(2026, 7, 1))];
    let use_case = ListUserWorkdays::new(repository);

    let result = use_case
        .execute(ListUserWorkdaysCommand {
            user_id: "user-1".to_string(),
            from: date(2026, 7, 1),
            to: date(2026, 7, 31),
        })
        .await
        .expect("list workdays");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].work_date, date(2026, 7, 1));
}

#[tokio::test]
async fn rejects_reversed_range() {
    let use_case = ListUserWorkdays::new(FakeReadRepository::default());

    let error = use_case
        .execute(ListUserWorkdaysCommand {
            user_id: "user-1".to_string(),
            from: date(2026, 7, 31),
            to: date(2026, 7, 1),
        })
        .await
        .expect_err("reversed range rejected");

    assert_eq!(error, ListUserWorkdaysError::InvalidRange);
}

#[tokio::test]
async fn rejects_range_larger_than_maximum() {
    let use_case = ListUserWorkdays::new(FakeReadRepository::default());
    let from = date(2026, 1, 1);
    let to = from + chrono::Duration::days(MAX_WORKDAY_RANGE_DAYS);

    let error = use_case
        .execute(ListUserWorkdaysCommand {
            user_id: "user-1".to_string(),
            from,
            to,
        })
        .await
        .expect_err("oversized range rejected");

    assert_eq!(error, ListUserWorkdaysError::RangeTooLarge);
}

#[tokio::test]
async fn allows_single_day_range() {
    let repository = FakeReadRepository::default();
    let use_case = ListUserWorkdays::new(repository);

    let result = use_case
        .execute(ListUserWorkdaysCommand {
            user_id: "user-9".to_string(),
            from: date(2026, 7, 10),
            to: date(2026, 7, 10),
        })
        .await
        .expect("single day allowed");

    assert!(result.is_empty());
}
