use sqlx::{PgPool, Postgres, Transaction};
use timekeeper_app::work_schedules::{
    NewResolvedWorkday, ResolveWorkdayError, ResolvedDayKind, ScheduleType, WorkScheduleSource,
};
use timekeeper_domain::work_schedules::{CoreTimeWindow, PlannedBreak, PlannedWorkInterval};
use uuid::Uuid;

use super::{database_error, parse_uuid};

pub(super) async fn upsert_projection(
    pool: &PgPool,
    projection: &NewResolvedWorkday,
) -> Result<(), ResolveWorkdayError> {
    let work_schedule_id = parse_uuid(&projection.work_schedule_id, "work_schedule_id")?;
    let version_id = parse_uuid(
        &projection.work_schedule_version_id,
        "work_schedule_version_id",
    )?;
    let source_id = parse_uuid(&projection.source_id, "source_id")?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| database_error("begin projection transaction"))?;
    let resolved_workday_id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO resolved_workdays \
         (id, user_id, work_date, work_schedule_id, work_schedule_version_id, source, source_id, \
          day_kind, timezone, workday_boundary, expected_work_minutes, schedule_type, \
          resolved_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
         ON CONFLICT (user_id, work_date) DO UPDATE SET \
             work_schedule_id = EXCLUDED.work_schedule_id, \
             work_schedule_version_id = EXCLUDED.work_schedule_version_id, \
             source = EXCLUDED.source, source_id = EXCLUDED.source_id, \
             day_kind = EXCLUDED.day_kind, timezone = EXCLUDED.timezone, \
             workday_boundary = EXCLUDED.workday_boundary, \
             expected_work_minutes = EXCLUDED.expected_work_minutes, \
             schedule_type = EXCLUDED.schedule_type, \
             resolved_at = EXCLUDED.resolved_at \
         WHERE resolved_workdays.locked_at IS NULL \
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(&projection.user_id)
    .bind(projection.work_date)
    .bind(work_schedule_id)
    .bind(version_id)
    .bind(source_value(projection.source))
    .bind(source_id)
    .bind(day_kind_value(projection.day_kind))
    .bind(&projection.timezone)
    .bind(projection.workday_boundary)
    .bind(projection.expected_work_minutes)
    .bind(schedule_type_value(projection.schedule_type))
    .bind(projection.resolved_at)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| database_error("upsert resolved workday"))?;

    let Some(resolved_workday_id) = resolved_workday_id else {
        transaction
            .commit()
            .await
            .map_err(|_| database_error("finish locked projection lookup"))?;
        return Ok(());
    };
    replace_children(
        &mut transaction,
        resolved_workday_id,
        &projection.work_intervals,
        &projection.planned_breaks,
        &projection.core_time_windows,
    )
    .await?;
    transaction
        .commit()
        .await
        .map_err(|_| database_error("commit resolved workday"))
}

async fn replace_children(
    transaction: &mut Transaction<'_, Postgres>,
    resolved_workday_id: Uuid,
    intervals: &[PlannedWorkInterval],
    breaks: &[PlannedBreak],
    core_time_windows: &[CoreTimeWindow],
) -> Result<(), ResolveWorkdayError> {
    sqlx::query("DELETE FROM resolved_workday_intervals WHERE resolved_workday_id = $1")
        .bind(resolved_workday_id)
        .execute(&mut **transaction)
        .await
        .map_err(|_| database_error("replace resolved work intervals"))?;
    sqlx::query("DELETE FROM resolved_workday_breaks WHERE resolved_workday_id = $1")
        .bind(resolved_workday_id)
        .execute(&mut **transaction)
        .await
        .map_err(|_| database_error("replace resolved planned breaks"))?;
    sqlx::query("DELETE FROM resolved_workday_core_time_windows WHERE resolved_workday_id = $1")
        .bind(resolved_workday_id)
        .execute(&mut **transaction)
        .await
        .map_err(|_| database_error("replace resolved core time windows"))?;
    for (index, interval) in intervals.iter().enumerate() {
        sqlx::query(
            "INSERT INTO resolved_workday_intervals \
             (resolved_workday_id, sequence, start_time, start_day_offset, \
              end_time, end_day_offset) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(resolved_workday_id)
        .bind(sequence(index)?)
        .bind(interval.start_time)
        .bind(i16::from(interval.start_day_offset))
        .bind(interval.end_time)
        .bind(i16::from(interval.end_day_offset))
        .execute(&mut **transaction)
        .await
        .map_err(|_| database_error("insert resolved work interval"))?;
    }
    for (index, planned_break) in breaks.iter().enumerate() {
        sqlx::query(
            "INSERT INTO resolved_workday_breaks \
             (resolved_workday_id, sequence, start_time, start_day_offset, \
              end_time, end_day_offset) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(resolved_workday_id)
        .bind(sequence(index)?)
        .bind(planned_break.start_time)
        .bind(i16::from(planned_break.start_day_offset))
        .bind(planned_break.end_time)
        .bind(i16::from(planned_break.end_day_offset))
        .execute(&mut **transaction)
        .await
        .map_err(|_| database_error("insert resolved planned break"))?;
    }
    for window in core_time_windows {
        sqlx::query(
            "INSERT INTO resolved_workday_core_time_windows \
             (resolved_workday_id, weekday, start_time, start_day_offset, \
              end_time, end_day_offset) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(resolved_workday_id)
        .bind(i16::from(window.weekday))
        .bind(window.start_time)
        .bind(i16::from(window.start_day_offset))
        .bind(window.end_time)
        .bind(i16::from(window.end_day_offset))
        .execute(&mut **transaction)
        .await
        .map_err(|_| database_error("insert resolved core time window"))?;
    }
    Ok(())
}

fn sequence(index: usize) -> Result<i32, ResolveWorkdayError> {
    i32::try_from(index + 1)
        .map_err(|_| ResolveWorkdayError::InvalidScheduleData("too many intervals".to_string()))
}

fn source_value(source: WorkScheduleSource) -> &'static str {
    match source {
        WorkScheduleSource::Override => "override",
        WorkScheduleSource::User => "user",
        WorkScheduleSource::Department => "department",
        WorkScheduleSource::Organization => "organization",
    }
}

fn day_kind_value(day_kind: ResolvedDayKind) -> &'static str {
    match day_kind {
        ResolvedDayKind::ScheduledWorkday => "scheduled_workday",
        ResolvedDayKind::ScheduledNonWorkingDay => "scheduled_non_working_day",
        ResolvedDayKind::PublicHoliday => "public_holiday",
    }
}

fn schedule_type_value(schedule_type: ScheduleType) -> &'static str {
    match schedule_type {
        ScheduleType::Fixed => "fixed",
        ScheduleType::Flex => "flex",
    }
}
