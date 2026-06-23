use async_trait::async_trait;
use chrono::NaiveDate;
use timekeeper_app::user_workdays::{ListUserWorkdaysError, UserWorkdayReadRepository};
use timekeeper_app::work_schedules::{ResolvedWorkday, WorkdayOverrideKind};
use timekeeper_app::workday_overrides::{
    NewWorkdayOverride, StoredWorkdayOverride, WorkdayOverrideError, WorkdayOverrideRepository,
};
use uuid::Uuid;

use super::rows::OverrideRecordRow;
use super::WorkdayResolverPostgresRepository;

#[async_trait]
impl UserWorkdayReadRepository for WorkdayResolverPostgresRepository {
    async fn list_resolved_in_range(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ResolvedWorkday>, ListUserWorkdaysError> {
        super::load_resolved_in_range(self.pool(), user_id, from, to)
            .await
            .map_err(|error| ListUserWorkdaysError::Repository(error.to_string()))
    }
}

#[async_trait]
impl WorkdayOverrideRepository for WorkdayResolverPostgresRepository {
    async fn is_resolved_locked(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<bool, WorkdayOverrideError> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM resolved_workdays \
             WHERE user_id = $1 AND work_date = $2 AND locked_at IS NOT NULL)",
        )
        .bind(user_id)
        .bind(work_date)
        .fetch_one(self.pool())
        .await
        .map_err(|_| override_error("check resolved workday lock"))
    }

    async fn upsert_override(
        &self,
        input: NewWorkdayOverride,
    ) -> Result<StoredWorkdayOverride, WorkdayOverrideError> {
        let work_schedule_id = input
            .work_schedule_id
            .as_deref()
            .map(parse_schedule_id)
            .transpose()?;
        let row = sqlx::query_as::<_, OverrideRecordRow>(
            "INSERT INTO workday_overrides \
             (id, user_id, work_date, kind, work_schedule_id, reason, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (user_id, work_date) DO UPDATE SET \
                 kind = EXCLUDED.kind, \
                 work_schedule_id = EXCLUDED.work_schedule_id, \
                 reason = EXCLUDED.reason, \
                 updated_at = NOW() \
             RETURNING id, user_id, work_date, kind, work_schedule_id, reason, \
                 created_by, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(&input.user_id)
        .bind(input.work_date)
        .bind(kind_value(input.kind))
        .bind(work_schedule_id)
        .bind(&input.reason)
        .bind(&input.created_by)
        .fetch_one(self.pool())
        .await
        .map_err(map_upsert_error)?;
        StoredWorkdayOverride::try_from(row)
    }

    async fn delete_override(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<bool, WorkdayOverrideError> {
        let result =
            sqlx::query("DELETE FROM workday_overrides WHERE user_id = $1 AND work_date = $2")
                .bind(user_id)
                .bind(work_date)
                .execute(self.pool())
                .await
                .map_err(map_delete_error)?;
        Ok(result.rows_affected() > 0)
    }
}

fn kind_value(kind: WorkdayOverrideKind) -> &'static str {
    match kind {
        WorkdayOverrideKind::NonWorkingDay => "non_working_day",
        WorkdayOverrideKind::UseSchedule => "use_schedule",
    }
}

fn parse_schedule_id(value: &str) -> Result<Uuid, WorkdayOverrideError> {
    Uuid::parse_str(value).map_err(|_| {
        WorkdayOverrideError::InvalidInput("work_schedule_id must be a valid UUID".to_string())
    })
}

fn map_upsert_error(error: sqlx::Error) -> WorkdayOverrideError {
    if let Some(error) = map_locked_override_error(&error) {
        return error;
    }
    if let sqlx::Error::Database(db_error) = &error {
        // FK 違反: 参照する work_schedule / user が存在しない
        if db_error.code().as_deref() == Some("23503") {
            return WorkdayOverrideError::InvalidInput(
                "referenced user or work schedule does not exist".to_string(),
            );
        }
    }
    override_error("upsert workday override")
}

fn map_delete_error(error: sqlx::Error) -> WorkdayOverrideError {
    if let Some(error) = map_locked_override_error(&error) {
        return error;
    }
    override_error("delete workday override")
}

fn map_locked_override_error(error: &sqlx::Error) -> Option<WorkdayOverrideError> {
    let sqlx::Error::Database(db_error) = error else {
        return None;
    };
    if db_error.code().as_deref() == Some("23514")
        && db_error.message().contains("resolved workdays are locked")
    {
        return Some(WorkdayOverrideError::ResolvedWorkdayLocked);
    }
    None
}

fn override_error(operation: &str) -> WorkdayOverrideError {
    WorkdayOverrideError::Repository(format!("{operation} failed"))
}
