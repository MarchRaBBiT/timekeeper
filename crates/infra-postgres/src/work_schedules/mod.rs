mod management;
mod persistence;
mod rows;

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::PgPool;
use std::collections::HashMap;
use timekeeper_app::work_schedules::{
    AssignmentTarget, NewResolvedWorkday, OrganizationHierarchy, ResolveWorkdayError,
    ResolvedWorkday, ScheduleAssignment, ScheduleDayRule, ScheduleVersion, WorkdayHolidayCalendar,
    WorkdayOverride, WorkdayResolutionRepository,
};
use timekeeper_domain::work_schedules::CoreTimeWindow;
use uuid::Uuid;

use rows::{
    assemble_day_rule, assemble_resolved_workday, AssignmentRow, CoreTimeWindowRow, DayRuleRow,
    IntervalRow, OverrideRow, ResolvedBreakRow, ResolvedCoreTimeWindowRow, ResolvedIntervalRow,
    ResolvedWorkdayRow, VersionRow,
};

const RESOLVED_COLUMNS: &str = "id, user_id, work_date, work_schedule_id, \
    work_schedule_version_id, source, source_id, day_kind, timezone, workday_boundary, \
    expected_work_minutes, schedule_type, resolved_at, locked_at";
const INTERVAL_COLUMNS: &str = "sequence, start_time, start_day_offset, end_time, end_day_offset";
const CORE_TIME_COLUMNS: &str = "weekday, start_time, start_day_offset, end_time, end_day_offset";

#[derive(Debug, Clone)]
pub struct WorkdayResolverPostgresRepository {
    pool: PgPool,
}

impl WorkdayResolverPostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl WorkdayResolutionRepository for WorkdayResolverPostgresRepository {
    async fn find_resolved(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<Option<ResolvedWorkday>, ResolveWorkdayError> {
        load_resolved(&self.pool, user_id, work_date).await
    }

    async fn find_override(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<Option<WorkdayOverride>, ResolveWorkdayError> {
        let row = sqlx::query_as::<_, OverrideRow>(
            "SELECT id, kind, work_schedule_id FROM workday_overrides \
             WHERE user_id = $1 AND work_date = $2",
        )
        .bind(user_id)
        .bind(work_date)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| database_error("load workday override"))?;
        row.map(WorkdayOverride::try_from).transpose()
    }

    async fn find_assignment(
        &self,
        target: &AssignmentTarget,
        work_date: NaiveDate,
    ) -> Result<Option<ScheduleAssignment>, ResolveWorkdayError> {
        let row = match target {
            AssignmentTarget::User(user_id) => {
                sqlx::query_as::<_, AssignmentRow>(
                    "SELECT id, work_schedule_id FROM work_schedule_assignments \
                 WHERE user_id = $1 AND valid_from <= $2 \
                 AND (valid_until IS NULL OR $2 < valid_until) LIMIT 1",
                )
                .bind(user_id)
                .bind(work_date)
                .fetch_optional(&self.pool)
                .await
            }
            AssignmentTarget::Department(department_id) => {
                sqlx::query_as::<_, AssignmentRow>(
                    "SELECT id, work_schedule_id FROM work_schedule_assignments \
                 WHERE department_id = $1 AND valid_from <= $2 \
                 AND (valid_until IS NULL OR $2 < valid_until) LIMIT 1",
                )
                .bind(department_id)
                .bind(work_date)
                .fetch_optional(&self.pool)
                .await
            }
            AssignmentTarget::Organization => {
                sqlx::query_as::<_, AssignmentRow>(
                    "SELECT id, work_schedule_id FROM work_schedule_assignments \
                 WHERE is_org_default AND valid_from <= $1 \
                 AND (valid_until IS NULL OR $1 < valid_until) LIMIT 1",
                )
                .bind(work_date)
                .fetch_optional(&self.pool)
                .await
            }
        }
        .map_err(|_| database_error("load work schedule assignment"))?;
        Ok(row.map(ScheduleAssignment::from))
    }

    async fn find_published_version(
        &self,
        work_schedule_id: &str,
        work_date: NaiveDate,
    ) -> Result<Option<ScheduleVersion>, ResolveWorkdayError> {
        let schedule_id = parse_uuid(work_schedule_id, "work_schedule_id")?;
        let row = sqlx::query_as::<_, VersionRow>(
            "SELECT id, work_schedule_id, timezone, workday_boundary, public_holiday_policy, \
             schedule_type FROM work_schedule_versions WHERE work_schedule_id = $1 \
             AND status = 'published' AND effective_from <= $2 \
             AND (effective_until IS NULL OR $2 < effective_until) \
             ORDER BY effective_from DESC LIMIT 1",
        )
        .bind(schedule_id)
        .bind(work_date)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| database_error("load published work schedule version"))?;
        row.map(ScheduleVersion::try_from).transpose()
    }

    async fn find_day_rule(
        &self,
        version_id: &str,
        weekday: u8,
    ) -> Result<Option<ScheduleDayRule>, ResolveWorkdayError> {
        let version_id = parse_uuid(version_id, "work_schedule_version_id")?;
        let row = sqlx::query_as::<_, DayRuleRow>(
            "SELECT id, day_kind, expected_work_minutes FROM work_schedule_day_rules \
             WHERE version_id = $1 AND weekday = $2",
        )
        .bind(version_id)
        .bind(i16::from(weekday))
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| database_error("load work schedule day rule"))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let interval_sql = format!(
            "SELECT {INTERVAL_COLUMNS} FROM work_schedule_work_intervals \
             WHERE day_rule_id = $1 ORDER BY sequence"
        );
        let intervals = sqlx::query_as::<_, IntervalRow>(&interval_sql)
            .bind(row.id)
            .fetch_all(&self.pool)
            .await
            .map_err(|_| database_error("load work schedule intervals"))?;
        let break_sql = format!(
            "SELECT {INTERVAL_COLUMNS} FROM work_schedule_planned_breaks \
             WHERE day_rule_id = $1 ORDER BY sequence"
        );
        let breaks = sqlx::query_as::<_, IntervalRow>(&break_sql)
            .bind(row.id)
            .fetch_all(&self.pool)
            .await
            .map_err(|_| database_error("load planned breaks"))?;
        assemble_day_rule(row, intervals, breaks).map(Some)
    }

    async fn find_core_time_window(
        &self,
        version_id: &str,
        weekday: u8,
    ) -> Result<Option<CoreTimeWindow>, ResolveWorkdayError> {
        let version_id = parse_uuid(version_id, "work_schedule_version_id")?;
        let sql = format!(
            "SELECT {CORE_TIME_COLUMNS} FROM work_schedule_core_time_windows \
             WHERE version_id = $1 AND weekday = $2"
        );
        let row = sqlx::query_as::<_, CoreTimeWindowRow>(&sql)
            .bind(version_id)
            .bind(i16::from(weekday))
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| database_error("load work schedule core time window"))?;
        row.map(CoreTimeWindow::try_from).transpose()
    }

    async fn save_projection(
        &self,
        projection: NewResolvedWorkday,
    ) -> Result<ResolvedWorkday, ResolveWorkdayError> {
        persistence::upsert_projection(&self.pool, &projection).await?;
        load_resolved(&self.pool, &projection.user_id, projection.work_date)
            .await?
            .ok_or_else(|| database_error("reload resolved workday"))
    }
}

#[async_trait]
impl OrganizationHierarchy for WorkdayResolverPostgresRepository {
    async fn department_lineage(&self, user_id: &str) -> Result<Vec<String>, ResolveWorkdayError> {
        sqlx::query_scalar(
            "WITH RECURSIVE lineage AS ( \
                 SELECT d.id, d.parent_id, 0 AS depth, ARRAY[d.id] AS path \
                 FROM users u JOIN departments d ON d.id = u.department_id WHERE u.id = $1 \
                 UNION ALL \
                 SELECT parent.id, parent.parent_id, child.depth + 1, child.path || parent.id \
                 FROM departments parent JOIN lineage child ON parent.id = child.parent_id \
                 WHERE NOT parent.id = ANY(child.path) \
             ) SELECT id FROM lineage ORDER BY depth",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| database_error("load department lineage"))
    }
}

#[async_trait]
impl WorkdayHolidayCalendar for WorkdayResolverPostgresRepository {
    async fn is_public_holiday(&self, work_date: NaiveDate) -> Result<bool, ResolveWorkdayError> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM holidays WHERE holiday_date = $1)")
            .bind(work_date)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| database_error("check public holiday"))
    }
}

async fn load_resolved(
    pool: &PgPool,
    user_id: &str,
    work_date: NaiveDate,
) -> Result<Option<ResolvedWorkday>, ResolveWorkdayError> {
    let sql = format!(
        "SELECT {RESOLVED_COLUMNS} FROM resolved_workdays WHERE user_id = $1 AND work_date = $2"
    );
    let row = sqlx::query_as::<_, ResolvedWorkdayRow>(&sql)
        .bind(user_id)
        .bind(work_date)
        .fetch_optional(pool)
        .await
        .map_err(|_| database_error("load resolved workday"))?;
    let Some(row) = row else {
        return Ok(None);
    };
    let interval_sql = format!(
        "SELECT {INTERVAL_COLUMNS} FROM resolved_workday_intervals \
         WHERE resolved_workday_id = $1 ORDER BY sequence"
    );
    let intervals = sqlx::query_as::<_, IntervalRow>(&interval_sql)
        .bind(row.id)
        .fetch_all(pool)
        .await
        .map_err(|_| database_error("load resolved work intervals"))?;
    let break_sql = format!(
        "SELECT {INTERVAL_COLUMNS} FROM resolved_workday_breaks \
         WHERE resolved_workday_id = $1 ORDER BY sequence"
    );
    let breaks = sqlx::query_as::<_, IntervalRow>(&break_sql)
        .bind(row.id)
        .fetch_all(pool)
        .await
        .map_err(|_| database_error("load resolved planned breaks"))?;
    let core_time_sql = format!(
        "SELECT {CORE_TIME_COLUMNS} FROM resolved_workday_core_time_windows \
         WHERE resolved_workday_id = $1"
    );
    let core_time_windows = sqlx::query_as::<_, CoreTimeWindowRow>(&core_time_sql)
        .bind(row.id)
        .fetch_all(pool)
        .await
        .map_err(|_| database_error("load resolved core time windows"))?;
    assemble_resolved_workday(row, intervals, breaks, core_time_windows).map(Some)
}

pub(crate) async fn load_resolved_in_range(
    pool: &PgPool,
    user_id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<ResolvedWorkday>, ResolveWorkdayError> {
    load_resolved_for_users_in_range(pool, &[user_id.to_string()], from, to).await
}

pub(crate) async fn load_resolved_for_users_in_range(
    pool: &PgPool,
    user_ids: &[String],
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<ResolvedWorkday>, ResolveWorkdayError> {
    let sql = format!(
        "SELECT {RESOLVED_COLUMNS} FROM resolved_workdays \
         WHERE user_id = ANY($1) AND work_date BETWEEN $2 AND $3 ORDER BY user_id, work_date"
    );
    let rows = sqlx::query_as::<_, ResolvedWorkdayRow>(&sql)
        .bind(user_ids)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(|_| database_error("load resolved workday range"))?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let resolved_ids: Vec<_> = rows.iter().map(|row| row.id).collect();
    let interval_sql = format!(
        "SELECT resolved_workday_id, {INTERVAL_COLUMNS} FROM resolved_workday_intervals \
         WHERE resolved_workday_id = ANY($1) ORDER BY resolved_workday_id, sequence"
    );
    let interval_rows = sqlx::query_as::<_, ResolvedIntervalRow>(&interval_sql)
        .bind(&resolved_ids)
        .fetch_all(pool)
        .await
        .map_err(|_| database_error("load resolved work intervals"))?;
    let break_sql = format!(
        "SELECT resolved_workday_id, {INTERVAL_COLUMNS} FROM resolved_workday_breaks \
         WHERE resolved_workday_id = ANY($1) ORDER BY resolved_workday_id, sequence"
    );
    let break_rows = sqlx::query_as::<_, ResolvedBreakRow>(&break_sql)
        .bind(&resolved_ids)
        .fetch_all(pool)
        .await
        .map_err(|_| database_error("load resolved planned breaks"))?;
    let core_time_sql = format!(
        "SELECT resolved_workday_id, {CORE_TIME_COLUMNS} FROM resolved_workday_core_time_windows \
         WHERE resolved_workday_id = ANY($1)"
    );
    let core_time_rows = sqlx::query_as::<_, ResolvedCoreTimeWindowRow>(&core_time_sql)
        .bind(&resolved_ids)
        .fetch_all(pool)
        .await
        .map_err(|_| database_error("load resolved core time windows"))?;

    let mut intervals_by_workday: HashMap<_, Vec<IntervalRow>> = HashMap::new();
    for row in interval_rows {
        intervals_by_workday
            .entry(row.resolved_workday_id)
            .or_default()
            .push(IntervalRow {
                start_time: row.start_time,
                start_day_offset: row.start_day_offset,
                end_time: row.end_time,
                end_day_offset: row.end_day_offset,
            });
    }

    let mut breaks_by_workday: HashMap<_, Vec<IntervalRow>> = HashMap::new();
    for row in break_rows {
        breaks_by_workday
            .entry(row.resolved_workday_id)
            .or_default()
            .push(IntervalRow {
                start_time: row.start_time,
                start_day_offset: row.start_day_offset,
                end_time: row.end_time,
                end_day_offset: row.end_day_offset,
            });
    }

    let mut core_time_windows_by_workday: HashMap<_, Vec<CoreTimeWindowRow>> = HashMap::new();
    for row in core_time_rows {
        core_time_windows_by_workday
            .entry(row.resolved_workday_id)
            .or_default()
            .push(CoreTimeWindowRow {
                weekday: row.weekday,
                start_time: row.start_time,
                start_day_offset: row.start_day_offset,
                end_time: row.end_time,
                end_day_offset: row.end_day_offset,
            });
    }

    let mut workdays = Vec::with_capacity(rows.len());
    for row in rows {
        let intervals = intervals_by_workday.remove(&row.id).unwrap_or_default();
        let breaks = breaks_by_workday.remove(&row.id).unwrap_or_default();
        let core_time_windows = core_time_windows_by_workday
            .remove(&row.id)
            .unwrap_or_default();
        let workday = assemble_resolved_workday(row, intervals, breaks, core_time_windows)
            .map_err(|error| ResolveWorkdayError::Repository(error.to_string()))?;
        workdays.push(workday);
    }
    Ok(workdays)
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, ResolveWorkdayError> {
    Uuid::parse_str(value).map_err(|_| {
        ResolveWorkdayError::InvalidScheduleData(format!("{field} must be a valid UUID"))
    })
}

fn database_error(operation: &str) -> ResolveWorkdayError {
    ResolveWorkdayError::Repository(format!("{operation} failed"))
}
