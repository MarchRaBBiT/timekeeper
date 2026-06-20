use sqlx::{PgPool, Postgres, Transaction};
use timekeeper_contract::work_schedules::{
    CreateWorkScheduleVersionRequest, DayKind, PublicHolidayPolicy,
    ReplaceWorkScheduleVersionRequest, WeekdayRuleInput, WorkScheduleVersionResponse,
};
use timekeeper_domain::work_schedules::{
    DayKind as DomainDayKind, PlannedBreak, PlannedWorkInterval, WeekdayRule,
};
use uuid::Uuid;

use super::{
    map_database_error,
    rows::{assemble_version, DayRuleRow, IntervalRow, VersionRow},
    RepositoryResult, WorkScheduleRepositoryError,
};

const VERSION_COLUMNS: &str = "id, work_schedule_id, version_number, status, effective_from, \
    effective_until, timezone, workday_boundary, public_holiday_policy, late_grace_minutes, \
    early_leave_grace_minutes, revision, published_by, published_at, created_at, updated_at";

pub async fn create_version(
    pool: &PgPool,
    schedule_id: Uuid,
    input: &CreateWorkScheduleVersionRequest,
) -> RepositoryResult<WorkScheduleVersionResponse> {
    let mut transaction = pool.begin().await?;
    let schedule_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM work_schedules WHERE id = $1 FOR UPDATE")
            .bind(schedule_id)
            .fetch_optional(&mut *transaction)
            .await?;
    let schedule_status = schedule_status.ok_or(WorkScheduleRepositoryError::NotFound)?;
    if schedule_status == "retired" {
        return Err(WorkScheduleRepositoryError::RetiredSchedule);
    }
    let version_number: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version_number), 0) + 1 FROM work_schedule_versions \
         WHERE work_schedule_id = $1",
    )
    .bind(schedule_id)
    .fetch_one(&mut *transaction)
    .await?;
    let version_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO work_schedule_versions (id, work_schedule_id, version_number, \
         effective_from, effective_until, timezone, workday_boundary, public_holiday_policy, \
         late_grace_minutes, early_leave_grace_minutes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(version_id)
    .bind(schedule_id)
    .bind(version_number)
    .bind(input.effective_from)
    .bind(input.effective_until)
    .bind(&input.timezone)
    .bind(input.workday_boundary)
    .bind(holiday_policy_value(input.public_holiday_policy))
    .bind(input.late_grace_minutes)
    .bind(input.early_leave_grace_minutes)
    .execute(&mut *transaction)
    .await
    .map_err(map_database_error)?;
    insert_days(&mut transaction, version_id, &input.days).await?;
    transaction.commit().await?;
    find_version(pool, schedule_id, version_id).await
}

pub async fn replace_version(
    pool: &PgPool,
    schedule_id: Uuid,
    version_id: Uuid,
    input: &ReplaceWorkScheduleVersionRequest,
) -> RepositoryResult<WorkScheduleVersionResponse> {
    let mut transaction = pool.begin().await?;
    let current: Option<(String, i32)> = sqlx::query_as(
        "SELECT status, revision FROM work_schedule_versions \
         WHERE id = $1 AND work_schedule_id = $2 FOR UPDATE",
    )
    .bind(version_id)
    .bind(schedule_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let (status, revision) = current.ok_or(WorkScheduleRepositoryError::NotFound)?;
    if status == "published" {
        return Err(WorkScheduleRepositoryError::PublishedVersionImmutable);
    }
    if status != "draft" {
        return Err(WorkScheduleRepositoryError::PublishedVersionImmutable);
    }
    if revision != input.revision {
        return Err(WorkScheduleRepositoryError::RevisionConflict);
    }
    sqlx::query(
        "UPDATE work_schedule_versions SET effective_from = $3, effective_until = $4, \
         timezone = $5, workday_boundary = $6, public_holiday_policy = $7, \
         late_grace_minutes = $8, early_leave_grace_minutes = $9, \
         revision = revision + 1, updated_at = NOW() WHERE id = $1 AND work_schedule_id = $2",
    )
    .bind(version_id)
    .bind(schedule_id)
    .bind(input.effective_from)
    .bind(input.effective_until)
    .bind(&input.timezone)
    .bind(input.workday_boundary)
    .bind(holiday_policy_value(input.public_holiday_policy))
    .bind(input.late_grace_minutes)
    .bind(input.early_leave_grace_minutes)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("DELETE FROM work_schedule_day_rules WHERE version_id = $1")
        .bind(version_id)
        .execute(&mut *transaction)
        .await?;
    insert_days(&mut transaction, version_id, &input.days).await?;
    transaction.commit().await?;
    find_version(pool, schedule_id, version_id).await
}

pub async fn publish_version(
    pool: &PgPool,
    schedule_id: Uuid,
    version_id: Uuid,
    published_by: &str,
) -> RepositoryResult<WorkScheduleVersionResponse> {
    let result = sqlx::query(
        "UPDATE work_schedule_versions SET status = 'published', published_by = $3, \
         published_at = NOW(), updated_at = NOW() \
         WHERE id = $1 AND work_schedule_id = $2 AND status = 'draft'",
    )
    .bind(version_id)
    .bind(schedule_id)
    .bind(published_by)
    .execute(pool)
    .await
    .map_err(map_database_error)?;
    if result.rows_affected() == 0 {
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM work_schedule_versions WHERE id = $1 AND work_schedule_id = $2",
        )
        .bind(version_id)
        .bind(schedule_id)
        .fetch_optional(pool)
        .await?;
        return match status.as_deref() {
            None => Err(WorkScheduleRepositoryError::NotFound),
            Some(_) => Err(WorkScheduleRepositoryError::PublishedVersionImmutable),
        };
    }
    find_version(pool, schedule_id, version_id).await
}

pub async fn delete_version(
    pool: &PgPool,
    schedule_id: Uuid,
    version_id: Uuid,
) -> RepositoryResult<()> {
    let result = sqlx::query(
        "DELETE FROM work_schedule_versions \
         WHERE id = $1 AND work_schedule_id = $2 AND status = 'draft'",
    )
    .bind(version_id)
    .bind(schedule_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 1 {
        return Ok(());
    }
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM work_schedule_versions WHERE id = $1 AND work_schedule_id = $2",
    )
    .bind(version_id)
    .bind(schedule_id)
    .fetch_optional(pool)
    .await?;
    match status.as_deref() {
        None => Err(WorkScheduleRepositoryError::NotFound),
        Some(_) => Err(WorkScheduleRepositoryError::PublishedVersionImmutable),
    }
}

pub async fn find_version(
    pool: &PgPool,
    schedule_id: Uuid,
    version_id: Uuid,
) -> RepositoryResult<WorkScheduleVersionResponse> {
    let version_sql = format!(
        "SELECT {VERSION_COLUMNS} FROM work_schedule_versions \
         WHERE id = $1 AND work_schedule_id = $2"
    );
    let version = sqlx::query_as::<_, VersionRow>(&version_sql)
        .bind(version_id)
        .bind(schedule_id)
        .fetch_optional(pool)
        .await?
        .ok_or(WorkScheduleRepositoryError::NotFound)?;
    let days = sqlx::query_as::<_, DayRuleRow>(
        "SELECT id, weekday, day_kind, expected_work_minutes \
         FROM work_schedule_day_rules WHERE version_id = $1 ORDER BY weekday",
    )
    .bind(version_id)
    .fetch_all(pool)
    .await?;
    let intervals = sqlx::query_as::<_, IntervalRow>(
        "SELECT i.day_rule_id, i.sequence, i.start_time, i.start_day_offset, \
         i.end_time, i.end_day_offset FROM work_schedule_work_intervals i \
         JOIN work_schedule_day_rules d ON d.id = i.day_rule_id \
         WHERE d.version_id = $1 ORDER BY d.weekday, i.sequence",
    )
    .bind(version_id)
    .fetch_all(pool)
    .await?;
    let breaks = sqlx::query_as::<_, IntervalRow>(
        "SELECT b.day_rule_id, b.sequence, b.start_time, b.start_day_offset, \
         b.end_time, b.end_day_offset FROM work_schedule_planned_breaks b \
         JOIN work_schedule_day_rules d ON d.id = b.day_rule_id \
         WHERE d.version_id = $1 ORDER BY d.weekday, b.sequence",
    )
    .bind(version_id)
    .fetch_all(pool)
    .await?;
    assemble_version(version, days, intervals, breaks)
}

async fn insert_days(
    transaction: &mut Transaction<'_, Postgres>,
    version_id: Uuid,
    days: &[WeekdayRuleInput],
) -> RepositoryResult<()> {
    for day in days {
        let day_rule_id = Uuid::new_v4();
        let domain_day = domain_day(day);
        sqlx::query(
            "INSERT INTO work_schedule_day_rules \
             (id, version_id, weekday, day_kind, expected_work_minutes) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(day_rule_id)
        .bind(version_id)
        .bind(i16::from(day.weekday))
        .bind(day_kind_value(day.day_kind))
        .bind(domain_day.expected_work_minutes())
        .execute(&mut **transaction)
        .await?;
        for (index, interval) in day.work_intervals.iter().enumerate() {
            sqlx::query(
                "INSERT INTO work_schedule_work_intervals \
                 (day_rule_id, sequence, start_time, start_day_offset, end_time, end_day_offset) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(day_rule_id)
            .bind(sequence(index)?)
            .bind(interval.start_time)
            .bind(i16::from(interval.start_day_offset))
            .bind(interval.end_time)
            .bind(i16::from(interval.end_day_offset))
            .execute(&mut **transaction)
            .await?;
        }
        for (index, planned_break) in day.planned_breaks.iter().enumerate() {
            sqlx::query(
                "INSERT INTO work_schedule_planned_breaks \
                 (day_rule_id, sequence, start_time, start_day_offset, end_time, end_day_offset) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(day_rule_id)
            .bind(sequence(index)?)
            .bind(planned_break.start_time)
            .bind(i16::from(planned_break.start_day_offset))
            .bind(planned_break.end_time)
            .bind(i16::from(planned_break.end_day_offset))
            .execute(&mut **transaction)
            .await?;
        }
    }
    Ok(())
}

fn domain_day(day: &WeekdayRuleInput) -> WeekdayRule {
    WeekdayRule {
        weekday: day.weekday,
        day_kind: match day.day_kind {
            DayKind::WorkingDay => DomainDayKind::WorkingDay,
            DayKind::NonWorkingDay => DomainDayKind::NonWorkingDay,
        },
        work_intervals: day
            .work_intervals
            .iter()
            .map(|interval| PlannedWorkInterval {
                start_time: interval.start_time,
                start_day_offset: interval.start_day_offset,
                end_time: interval.end_time,
                end_day_offset: interval.end_day_offset,
            })
            .collect(),
        planned_breaks: day
            .planned_breaks
            .iter()
            .map(|planned_break| PlannedBreak {
                start_time: planned_break.start_time,
                start_day_offset: planned_break.start_day_offset,
                end_time: planned_break.end_time,
                end_day_offset: planned_break.end_day_offset,
            })
            .collect(),
    }
}

fn sequence(index: usize) -> RepositoryResult<i32> {
    i32::try_from(index + 1).map_err(|_| {
        WorkScheduleRepositoryError::CorruptData("too many schedule intervals".to_string())
    })
}

fn holiday_policy_value(value: PublicHolidayPolicy) -> &'static str {
    match value {
        PublicHolidayPolicy::NonWorking => "non_working",
        PublicHolidayPolicy::FollowWeeklyPattern => "follow_weekly_pattern",
    }
}

fn day_kind_value(value: DayKind) -> &'static str {
    match value {
        DayKind::WorkingDay => "working_day",
        DayKind::NonWorkingDay => "non_working_day",
    }
}
