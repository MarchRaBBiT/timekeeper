use async_trait::async_trait;
use chrono::NaiveDate;
use timekeeper_app::user_workdays::{ListUserWorkdaysError, UserWorkdayReadRepository};
use timekeeper_app::work_schedules::{ResolvedWorkday, WorkdayOverrideKind};
use timekeeper_app::workday_overrides::{
    NewWorkdayOverride, StoredWorkdayOverride, WorkdayOverrideError, WorkdayOverrideRepository,
};
use uuid::Uuid;

use super::rows::{assemble_resolved_workday, IntervalRow, OverrideRecordRow, ResolvedWorkdayRow};
use super::{WorkdayResolverPostgresRepository, INTERVAL_COLUMNS, RESOLVED_COLUMNS};

#[async_trait]
impl UserWorkdayReadRepository for WorkdayResolverPostgresRepository {
    async fn list_resolved_in_range(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ResolvedWorkday>, ListUserWorkdaysError> {
        let sql = format!(
            "SELECT {RESOLVED_COLUMNS} FROM resolved_workdays \
             WHERE user_id = $1 AND work_date BETWEEN $2 AND $3 ORDER BY work_date"
        );
        let rows = sqlx::query_as::<_, ResolvedWorkdayRow>(&sql)
            .bind(user_id)
            .bind(from)
            .bind(to)
            .fetch_all(self.pool())
            .await
            .map_err(|_| read_error("load resolved workday range"))?;

        let mut workdays = Vec::with_capacity(rows.len());
        for row in rows {
            let interval_sql = format!(
                "SELECT {INTERVAL_COLUMNS} FROM resolved_workday_intervals \
                 WHERE resolved_workday_id = $1 ORDER BY sequence"
            );
            let intervals = sqlx::query_as::<_, IntervalRow>(&interval_sql)
                .bind(row.id)
                .fetch_all(self.pool())
                .await
                .map_err(|_| read_error("load resolved work intervals"))?;
            let break_sql = format!(
                "SELECT {INTERVAL_COLUMNS} FROM resolved_workday_breaks \
                 WHERE resolved_workday_id = $1 ORDER BY sequence"
            );
            let breaks = sqlx::query_as::<_, IntervalRow>(&break_sql)
                .bind(row.id)
                .fetch_all(self.pool())
                .await
                .map_err(|_| read_error("load resolved planned breaks"))?;
            let workday = assemble_resolved_workday(row, intervals, breaks)
                .map_err(|error| ListUserWorkdaysError::Repository(error.to_string()))?;
            workdays.push(workday);
        }
        Ok(workdays)
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
                .map_err(|_| override_error("delete workday override"))?;
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

fn read_error(operation: &str) -> ListUserWorkdaysError {
    ListUserWorkdaysError::Repository(format!("{operation} failed"))
}

fn override_error(operation: &str) -> WorkdayOverrideError {
    WorkdayOverrideError::Repository(format!("{operation} failed"))
}
