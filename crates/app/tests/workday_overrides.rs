use std::sync::Mutex;

use chrono::{NaiveDate, Utc};
use timekeeper_app::work_schedules::WorkdayOverrideKind;
use timekeeper_app::workday_overrides::{
    DeleteWorkdayOverride, NewWorkdayOverride, SetWorkdayOverride, SetWorkdayOverrideCommand,
    StoredWorkdayOverride, WorkdayOverrideError, WorkdayOverrideRepository,
};

#[derive(Default)]
struct FakeOverrideRepository {
    locked: Mutex<bool>,
    upserts: Mutex<Vec<NewWorkdayOverride>>,
    deletes: Mutex<Vec<(String, NaiveDate)>>,
    delete_hits: Mutex<bool>,
}

#[async_trait::async_trait]
impl WorkdayOverrideRepository for FakeOverrideRepository {
    async fn is_resolved_locked(
        &self,
        _user_id: &str,
        _work_date: NaiveDate,
    ) -> Result<bool, WorkdayOverrideError> {
        Ok(*self.locked.lock().expect("locked lock"))
    }

    async fn upsert_override(
        &self,
        input: NewWorkdayOverride,
    ) -> Result<StoredWorkdayOverride, WorkdayOverrideError> {
        let stored = StoredWorkdayOverride {
            id: "override-1".to_string(),
            user_id: input.user_id.clone(),
            work_date: input.work_date,
            kind: input.kind,
            work_schedule_id: input.work_schedule_id.clone(),
            reason: input.reason.clone(),
            created_by: input.created_by.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.upserts.lock().expect("upserts lock").push(input);
        Ok(stored)
    }

    async fn delete_override(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<bool, WorkdayOverrideError> {
        self.deletes
            .lock()
            .expect("deletes lock")
            .push((user_id.to_string(), work_date));
        Ok(*self.delete_hits.lock().expect("delete hits lock"))
    }
}

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 7, 15).expect("valid date")
}

fn set_command(
    kind: WorkdayOverrideKind,
    schedule: Option<&str>,
    reason: &str,
) -> SetWorkdayOverrideCommand {
    SetWorkdayOverrideCommand {
        user_id: "user-1".to_string(),
        work_date: date(),
        kind,
        work_schedule_id: schedule.map(str::to_string),
        reason: reason.to_string(),
        created_by: "manager-1".to_string(),
    }
}

#[tokio::test]
async fn upserts_non_working_day_override() {
    let use_case = SetWorkdayOverride::new(FakeOverrideRepository::default());

    let stored = use_case
        .execute(set_command(
            WorkdayOverrideKind::NonWorkingDay,
            None,
            " 創立記念日 ",
        ))
        .await
        .expect("override stored");

    assert_eq!(stored.kind, WorkdayOverrideKind::NonWorkingDay);
    assert_eq!(stored.reason, "創立記念日");
    assert_eq!(stored.work_schedule_id, None);
}

#[tokio::test]
async fn rejects_use_schedule_without_schedule_id() {
    let use_case = SetWorkdayOverride::new(FakeOverrideRepository::default());

    let error = use_case
        .execute(set_command(
            WorkdayOverrideKind::UseSchedule,
            None,
            "特別勤務",
        ))
        .await
        .expect_err("missing schedule rejected");

    assert!(matches!(error, WorkdayOverrideError::InvalidInput(_)));
}

#[tokio::test]
async fn rejects_non_working_day_with_schedule_id() {
    let use_case = SetWorkdayOverride::new(FakeOverrideRepository::default());

    let error = use_case
        .execute(set_command(
            WorkdayOverrideKind::NonWorkingDay,
            Some("ws-1"),
            "矛盾",
        ))
        .await
        .expect_err("conflicting schedule rejected");

    assert!(matches!(error, WorkdayOverrideError::InvalidInput(_)));
}

#[tokio::test]
async fn rejects_blank_reason() {
    let use_case = SetWorkdayOverride::new(FakeOverrideRepository::default());

    let error = use_case
        .execute(set_command(WorkdayOverrideKind::NonWorkingDay, None, "   "))
        .await
        .expect_err("blank reason rejected");

    assert!(matches!(error, WorkdayOverrideError::InvalidInput(_)));
}

#[tokio::test]
async fn rejects_upsert_when_resolved_workday_locked() {
    let repository = FakeOverrideRepository::default();
    *repository.locked.lock().expect("locked lock") = true;
    let use_case = SetWorkdayOverride::new(repository);

    let error = use_case
        .execute(set_command(
            WorkdayOverrideKind::NonWorkingDay,
            None,
            "締め後",
        ))
        .await
        .expect_err("locked workday rejected");

    assert_eq!(error, WorkdayOverrideError::ResolvedWorkdayLocked);
}

#[tokio::test]
async fn deletes_existing_override() {
    let repository = FakeOverrideRepository::default();
    *repository.delete_hits.lock().expect("delete hits lock") = true;
    let use_case = DeleteWorkdayOverride::new(repository);

    use_case
        .execute("user-1", date())
        .await
        .expect("delete succeeds");
}

#[tokio::test]
async fn delete_missing_override_returns_not_found() {
    let repository = FakeOverrideRepository::default();
    let use_case = DeleteWorkdayOverride::new(repository);

    let error = use_case
        .execute("user-1", date())
        .await
        .expect_err("missing override rejected");

    assert_eq!(error, WorkdayOverrideError::NotFound);
}

#[tokio::test]
async fn delete_rejected_when_resolved_workday_locked() {
    let repository = FakeOverrideRepository::default();
    *repository.locked.lock().expect("locked lock") = true;
    *repository.delete_hits.lock().expect("delete hits lock") = true;
    let use_case = DeleteWorkdayOverride::new(repository);

    let error = use_case
        .execute("user-1", date())
        .await
        .expect_err("locked delete rejected");

    assert_eq!(error, WorkdayOverrideError::ResolvedWorkdayLocked);
}
