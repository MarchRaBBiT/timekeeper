use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use timekeeper_app::attendance::{ClockInError, ClockInWorkdayResolver};
use timekeeper_app::work_schedules::{
    AssignmentTarget, NewResolvedWorkday, OrganizationHierarchy, PublicHolidayPolicy,
    ResolveWorkday, ResolveWorkdayCommand, ResolveWorkdayError, ResolvedDayKind, ResolvedWorkday,
    ScheduleAssignment, ScheduleDayRule, ScheduleVersion, WorkScheduleSource,
    WorkdayHolidayCalendar, WorkdayOverride, WorkdayOverrideKind, WorkdayResolutionRepository,
};
use timekeeper_domain::{
    work_schedules::{DayKind, PlannedBreak, PlannedWorkInterval},
    WorkDate,
};

#[derive(Default)]
struct FakeRepository {
    existing: Mutex<Option<ResolvedWorkday>>,
    override_value: Mutex<Option<WorkdayOverride>>,
    assignments: Mutex<HashMap<AssignmentTarget, ScheduleAssignment>>,
    versions: Mutex<HashMap<String, ScheduleVersion>>,
    rules: Mutex<HashMap<String, ScheduleDayRule>>,
    targets: Arc<Mutex<Vec<AssignmentTarget>>>,
    saved: Arc<Mutex<Vec<NewResolvedWorkday>>>,
}

#[async_trait::async_trait]
impl WorkdayResolutionRepository for FakeRepository {
    async fn find_resolved(
        &self,
        _user_id: &str,
        _work_date: NaiveDate,
    ) -> Result<Option<ResolvedWorkday>, ResolveWorkdayError> {
        Ok(self.existing.lock().expect("existing lock").clone())
    }

    async fn find_override(
        &self,
        _user_id: &str,
        _work_date: NaiveDate,
    ) -> Result<Option<WorkdayOverride>, ResolveWorkdayError> {
        Ok(self.override_value.lock().expect("override lock").clone())
    }

    async fn find_assignment(
        &self,
        target: &AssignmentTarget,
        _work_date: NaiveDate,
    ) -> Result<Option<ScheduleAssignment>, ResolveWorkdayError> {
        self.targets
            .lock()
            .expect("targets lock")
            .push(target.clone());
        Ok(self
            .assignments
            .lock()
            .expect("assignments lock")
            .get(target)
            .cloned())
    }

    async fn find_published_version(
        &self,
        work_schedule_id: &str,
        _work_date: NaiveDate,
    ) -> Result<Option<ScheduleVersion>, ResolveWorkdayError> {
        Ok(self
            .versions
            .lock()
            .expect("versions lock")
            .get(work_schedule_id)
            .cloned())
    }

    async fn find_day_rule(
        &self,
        version_id: &str,
        _weekday: u8,
    ) -> Result<Option<ScheduleDayRule>, ResolveWorkdayError> {
        Ok(self
            .rules
            .lock()
            .expect("rules lock")
            .get(version_id)
            .cloned())
    }

    async fn save_projection(
        &self,
        projection: NewResolvedWorkday,
    ) -> Result<ResolvedWorkday, ResolveWorkdayError> {
        self.saved
            .lock()
            .expect("saved lock")
            .push(projection.clone());
        Ok(ResolvedWorkday::from_new(
            "resolved-1".to_string(),
            projection,
        ))
    }
}

struct FixedHierarchy {
    departments: Vec<String>,
}

#[async_trait::async_trait]
impl OrganizationHierarchy for FixedHierarchy {
    async fn department_lineage(&self, _user_id: &str) -> Result<Vec<String>, ResolveWorkdayError> {
        Ok(self.departments.clone())
    }
}

struct FixedHolidayCalendar(bool);

#[async_trait::async_trait]
impl WorkdayHolidayCalendar for FixedHolidayCalendar {
    async fn is_public_holiday(&self, _work_date: NaiveDate) -> Result<bool, ResolveWorkdayError> {
        Ok(self.0)
    }
}

fn work_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 7, 6).expect("valid Monday")
}

fn resolved_at() -> DateTime<Utc> {
    DateTime::from_timestamp(1_783_296_000, 0).expect("valid timestamp")
}

fn command() -> ResolveWorkdayCommand {
    ResolveWorkdayCommand {
        user_id: "user-1".to_string(),
        work_date: work_date(),
        resolved_at: resolved_at(),
    }
}

fn assignment(id: &str, schedule_id: &str) -> ScheduleAssignment {
    ScheduleAssignment {
        id: id.to_string(),
        work_schedule_id: schedule_id.to_string(),
    }
}

fn version(schedule_id: &str, policy: PublicHolidayPolicy) -> ScheduleVersion {
    ScheduleVersion {
        id: format!("version-{schedule_id}"),
        work_schedule_id: schedule_id.to_string(),
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: NaiveTime::from_hms_opt(5, 0, 0).expect("boundary"),
        public_holiday_policy: policy,
    }
}

fn night_rule() -> ScheduleDayRule {
    ScheduleDayRule {
        day_kind: DayKind::WorkingDay,
        expected_work_minutes: 480,
        work_intervals: vec![PlannedWorkInterval {
            start_time: NaiveTime::from_hms_opt(22, 0, 0).expect("start"),
            start_day_offset: 0,
            end_time: NaiveTime::from_hms_opt(7, 0, 0).expect("end"),
            end_day_offset: 1,
        }],
        planned_breaks: vec![PlannedBreak {
            start_time: NaiveTime::from_hms_opt(2, 0, 0).expect("break start"),
            start_day_offset: 1,
            end_time: NaiveTime::from_hms_opt(3, 0, 0).expect("break end"),
            end_day_offset: 1,
        }],
    }
}

fn configure_schedule(repository: &FakeRepository, schedule_id: &str, policy: PublicHolidayPolicy) {
    let version = version(schedule_id, policy);
    repository
        .rules
        .lock()
        .expect("rules lock")
        .insert(version.id.clone(), night_rule());
    repository
        .versions
        .lock()
        .expect("versions lock")
        .insert(schedule_id.to_string(), version);
}

fn resolver(
    repository: FakeRepository,
    departments: Vec<&str>,
    holiday: bool,
) -> ResolveWorkday<FakeRepository, FixedHierarchy, FixedHolidayCalendar> {
    ResolveWorkday::new(
        repository,
        FixedHierarchy {
            departments: departments.into_iter().map(str::to_string).collect(),
        },
        FixedHolidayCalendar(holiday),
    )
}

#[tokio::test]
async fn returns_locked_projection_without_re_resolving() {
    let repository = FakeRepository::default();
    let locked = ResolvedWorkday {
        id: "locked-1".to_string(),
        user_id: "user-1".to_string(),
        work_date: work_date(),
        work_schedule_id: "schedule-old".to_string(),
        work_schedule_version_id: "version-old".to_string(),
        source: WorkScheduleSource::Organization,
        source_id: "assignment-old".to_string(),
        day_kind: ResolvedDayKind::ScheduledWorkday,
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: NaiveTime::from_hms_opt(5, 0, 0).expect("boundary"),
        expected_work_minutes: 480,
        work_intervals: vec![],
        planned_breaks: vec![],
        resolved_at: resolved_at(),
        locked_at: Some(resolved_at()),
    };
    *repository.existing.lock().expect("existing lock") = Some(locked.clone());
    let targets = Arc::clone(&repository.targets);
    let saved = Arc::clone(&repository.saved);

    let result = resolver(repository, vec!["dept-1"], true)
        .execute(command())
        .await
        .expect("locked projection");

    assert_eq!(result, locked);
    assert!(targets.lock().expect("targets lock").is_empty());
    assert!(saved.lock().expect("saved lock").is_empty());
}

#[tokio::test]
async fn user_assignment_wins_and_non_working_holiday_clears_intervals() {
    let repository = FakeRepository::default();
    repository
        .assignments
        .lock()
        .expect("assignments lock")
        .insert(
            AssignmentTarget::User("user-1".to_string()),
            assignment("assignment-user", "schedule-user"),
        );
    configure_schedule(
        &repository,
        "schedule-user",
        PublicHolidayPolicy::NonWorking,
    );

    let result = resolver(repository, vec!["dept-child", "dept-parent"], true)
        .execute(command())
        .await
        .expect("resolved user schedule");

    assert_eq!(result.source, WorkScheduleSource::User);
    assert_eq!(result.source_id, "assignment-user");
    assert_eq!(result.day_kind, ResolvedDayKind::PublicHoliday);
    assert_eq!(result.expected_work_minutes, 0);
    assert!(result.work_intervals.is_empty());
    assert!(result.planned_breaks.is_empty());
}

#[tokio::test]
async fn nearest_available_department_assignment_wins_before_organization() {
    let repository = FakeRepository::default();
    repository
        .assignments
        .lock()
        .expect("assignments lock")
        .insert(
            AssignmentTarget::Department("dept-parent".to_string()),
            assignment("assignment-parent", "schedule-parent"),
        );
    repository
        .assignments
        .lock()
        .expect("assignments lock")
        .insert(
            AssignmentTarget::Organization,
            assignment("assignment-org", "schedule-org"),
        );
    configure_schedule(
        &repository,
        "schedule-parent",
        PublicHolidayPolicy::FollowWeeklyPattern,
    );
    let targets = Arc::clone(&repository.targets);

    let result = resolver(repository, vec!["dept-child", "dept-parent"], false)
        .execute(command())
        .await
        .expect("resolved department schedule");

    assert_eq!(result.source, WorkScheduleSource::Department);
    assert_eq!(result.source_id, "assignment-parent");
    assert_eq!(result.work_intervals[0].end_day_offset, 1);
    assert_eq!(
        *targets.lock().expect("targets lock"),
        vec![
            AssignmentTarget::User("user-1".to_string()),
            AssignmentTarget::Department("dept-child".to_string()),
            AssignmentTarget::Department("dept-parent".to_string()),
        ]
    );
}

#[tokio::test]
async fn organization_fallback_can_follow_weekly_pattern_on_public_holiday() {
    let repository = FakeRepository::default();
    repository
        .assignments
        .lock()
        .expect("assignments lock")
        .insert(
            AssignmentTarget::Organization,
            assignment("assignment-org", "schedule-org"),
        );
    configure_schedule(
        &repository,
        "schedule-org",
        PublicHolidayPolicy::FollowWeeklyPattern,
    );

    let result = resolver(repository, vec![], true)
        .execute(command())
        .await
        .expect("resolved organization schedule");

    assert_eq!(result.source, WorkScheduleSource::Organization);
    assert_eq!(result.day_kind, ResolvedDayKind::ScheduledWorkday);
    assert_eq!(result.expected_work_minutes, 480);
    assert_eq!(result.planned_breaks.len(), 1);
}

#[tokio::test]
async fn use_schedule_override_wins_and_suppresses_public_holiday_policy() {
    let repository = FakeRepository::default();
    *repository.override_value.lock().expect("override lock") = Some(WorkdayOverride {
        id: "override-1".to_string(),
        kind: WorkdayOverrideKind::UseSchedule,
        work_schedule_id: Some("schedule-override".to_string()),
    });
    configure_schedule(
        &repository,
        "schedule-override",
        PublicHolidayPolicy::NonWorking,
    );

    let result = resolver(repository, vec![], true)
        .execute(command())
        .await
        .expect("resolved override schedule");

    assert_eq!(result.source, WorkScheduleSource::Override);
    assert_eq!(result.source_id, "override-1");
    assert_eq!(result.day_kind, ResolvedDayKind::ScheduledWorkday);
    assert_eq!(result.expected_work_minutes, 480);
}

#[tokio::test]
async fn non_working_override_uses_base_schedule_provenance_and_clears_plan() {
    let repository = FakeRepository::default();
    *repository.override_value.lock().expect("override lock") = Some(WorkdayOverride {
        id: "override-1".to_string(),
        kind: WorkdayOverrideKind::NonWorkingDay,
        work_schedule_id: None,
    });
    repository
        .assignments
        .lock()
        .expect("assignments lock")
        .insert(
            AssignmentTarget::Organization,
            assignment("assignment-org", "schedule-org"),
        );
    configure_schedule(
        &repository,
        "schedule-org",
        PublicHolidayPolicy::FollowWeeklyPattern,
    );

    let result = resolver(repository, vec![], false)
        .execute(command())
        .await
        .expect("resolved non-working override");

    assert_eq!(result.source, WorkScheduleSource::Override);
    assert_eq!(result.work_schedule_id, "schedule-org");
    assert_eq!(result.day_kind, ResolvedDayKind::ScheduledNonWorkingDay);
    assert_eq!(result.expected_work_minutes, 0);
    assert!(result.work_intervals.is_empty());
}

#[tokio::test]
async fn returns_not_configured_without_creating_an_implicit_schedule() {
    let repository = FakeRepository::default();
    let saved = Arc::clone(&repository.saved);

    let error = resolver(repository, vec!["dept-1"], false)
        .execute(command())
        .await
        .expect_err("missing assignment must fail");

    assert_eq!(error, ResolveWorkdayError::WorkScheduleNotConfigured);
    assert!(saved.lock().expect("saved lock").is_empty());
}

#[tokio::test]
async fn punch_before_workday_boundary_resolves_to_previous_work_date() {
    let repository = FakeRepository::default();
    repository
        .assignments
        .lock()
        .expect("assignments lock")
        .insert(
            AssignmentTarget::Organization,
            assignment("assignment-org", "schedule-org"),
        );
    configure_schedule(
        &repository,
        "schedule-org",
        PublicHolidayPolicy::FollowWeeklyPattern,
    );
    let resolver = resolver(repository, vec![], false);
    let punch_time = NaiveDate::from_ymd_opt(2026, 7, 7)
        .expect("date")
        .and_hms_opt(2, 0, 0)
        .expect("punch time");

    let result = resolver
        .resolve_for_punch("user-1", None, punch_time, resolved_at())
        .await
        .expect("previous work date");

    assert_eq!(result.work_date.to_string(), "2026-07-06");
}

#[tokio::test]
async fn explicit_punch_work_date_bypasses_boundary_derivation() {
    let repository = FakeRepository::default();
    repository
        .assignments
        .lock()
        .expect("assignments lock")
        .insert(
            AssignmentTarget::Organization,
            assignment("assignment-org", "schedule-org"),
        );
    configure_schedule(
        &repository,
        "schedule-org",
        PublicHolidayPolicy::FollowWeeklyPattern,
    );
    let resolver = resolver(repository, vec![], false);
    let punch_time = NaiveDate::from_ymd_opt(2026, 7, 7)
        .expect("date")
        .and_hms_opt(2, 0, 0)
        .expect("punch time");

    let result = resolver
        .resolve_for_punch(
            "user-1",
            Some(WorkDate::from_ymd(2026, 7, 7).expect("work date")),
            punch_time,
            resolved_at(),
        )
        .await
        .expect("explicit work date");

    assert_eq!(result.work_date.to_string(), "2026-07-07");
}

#[tokio::test]
async fn punch_after_workday_boundary_uses_current_work_date() {
    let repository = FakeRepository::default();
    repository
        .assignments
        .lock()
        .expect("assignments lock")
        .insert(
            AssignmentTarget::Organization,
            assignment("assignment-org", "schedule-org"),
        );
    configure_schedule(
        &repository,
        "schedule-org",
        PublicHolidayPolicy::FollowWeeklyPattern,
    );
    let resolver = resolver(repository, vec![], false);
    let punch_time = NaiveDate::from_ymd_opt(2026, 7, 7)
        .expect("date")
        .and_hms_opt(9, 0, 0)
        .expect("punch time");

    let result = resolver
        .resolve_for_punch("user-1", None, punch_time, resolved_at())
        .await
        .expect("current work date");

    assert_eq!(result.work_date.to_string(), "2026-07-07");
}

#[tokio::test]
async fn punch_without_any_schedule_returns_not_configured() {
    let resolver = resolver(FakeRepository::default(), vec![], false);
    let punch_time = NaiveDate::from_ymd_opt(2026, 7, 7)
        .expect("date")
        .and_hms_opt(9, 0, 0)
        .expect("punch time");

    let error = resolver
        .resolve_for_punch("user-1", None, punch_time, resolved_at())
        .await
        .expect_err("missing schedule");

    assert!(matches!(error, ClockInError::WorkScheduleNotConfigured));
}
