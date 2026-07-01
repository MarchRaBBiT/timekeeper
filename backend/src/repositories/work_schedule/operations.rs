use std::collections::HashMap;

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use sqlx::{FromRow, PgPool};
use timekeeper_contract::work_schedules::{
    WorkScheduleAnomalyKind, WorkScheduleAnomalyResponse, WorkScheduleCalendarAttendanceResponse,
};
use uuid::Uuid;

use super::{RepositoryResult, WorkScheduleRepositoryError};

#[derive(Debug, Clone, FromRow)]
struct AttendanceCalendarRow {
    id: String,
    user_id: String,
    date: NaiveDate,
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
    is_unscheduled_work: bool,
}

#[derive(Debug, Clone, FromRow)]
struct ResolvedStateRow {
    user_id: String,
    work_date: NaiveDate,
    day_kind: String,
}

pub async fn list_user_attendance_calendar(
    pool: &PgPool,
    user_id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> RepositoryResult<HashMap<NaiveDate, WorkScheduleCalendarAttendanceResponse>> {
    let rows = sqlx::query_as::<_, AttendanceCalendarRow>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, is_unscheduled_work \
         FROM attendance WHERE user_id = $1 AND date BETWEEN $2 AND $3 ORDER BY date",
    )
    .bind(user_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.date,
                WorkScheduleCalendarAttendanceResponse {
                    id: row.id,
                    clock_in_time: row.clock_in_time,
                    clock_out_time: row.clock_out_time,
                    is_unscheduled_work: row.is_unscheduled_work,
                },
            )
        })
        .collect())
}

pub async fn list_anomalies(
    pool: &PgPool,
    user_ids: Option<Vec<String>>,
    from: NaiveDate,
    to: NaiveDate,
) -> RepositoryResult<Vec<WorkScheduleAnomalyResponse>> {
    let users = match user_ids {
        Some(ids) => ids,
        None => {
            sqlx::query_scalar::<_, String>("SELECT id FROM users ORDER BY id")
                .fetch_all(pool)
                .await?
        }
    };
    if users.is_empty() {
        return Ok(Vec::new());
    }

    let resolved_rows = sqlx::query_as::<_, ResolvedStateRow>(
        "SELECT user_id, work_date, day_kind FROM resolved_workdays \
         WHERE user_id = ANY($1) AND work_date BETWEEN $2 AND $3",
    )
    .bind(&users)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    let resolved: HashMap<_, _> = resolved_rows
        .into_iter()
        .map(|row| ((row.user_id, row.work_date), row.day_kind))
        .collect();

    let attendance_rows = sqlx::query_as::<_, AttendanceCalendarRow>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, is_unscheduled_work \
         FROM attendance WHERE user_id = ANY($1) AND date BETWEEN $2 AND $3",
    )
    .bind(&users)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    let attendance: HashMap<_, _> = attendance_rows
        .into_iter()
        .map(|row| ((row.user_id.clone(), row.date), row))
        .collect();

    let mut items = Vec::new();
    for user_id in users {
        for work_date in dates_inclusive(from, to)? {
            let key = (user_id.clone(), work_date);
            let Some(day_kind) = resolved.get(&key) else {
                items.push(anomaly(
                    &user_id,
                    work_date,
                    WorkScheduleAnomalyKind::ScheduleNotConfigured,
                    "work schedule is not configured",
                ));
                continue;
            };
            let attendance = attendance.get(&key);
            if attendance.is_some_and(|row| row.is_unscheduled_work) {
                items.push(anomaly(
                    &user_id,
                    work_date,
                    WorkScheduleAnomalyKind::UnscheduledWork,
                    "attendance was recorded on an unscheduled workday",
                ));
            }
            if day_kind == "scheduled_workday" {
                match attendance {
                    None => items.push(anomaly(
                        &user_id,
                        work_date,
                        WorkScheduleAnomalyKind::MissingClockIn,
                        "scheduled workday has no clock-in",
                    )),
                    Some(row) if row.clock_in_time.is_none() => items.push(anomaly(
                        &user_id,
                        work_date,
                        WorkScheduleAnomalyKind::MissingClockIn,
                        "scheduled workday has no clock-in",
                    )),
                    Some(row) if row.clock_out_time.is_none() => items.push(anomaly(
                        &user_id,
                        work_date,
                        WorkScheduleAnomalyKind::MissingClockOut,
                        "clock-in has no matching clock-out",
                    )),
                    Some(_) => {}
                }
            }
        }
    }
    items.sort_by(|left, right| {
        (
            left.user_id.as_str(),
            left.work_date,
            anomaly_rank(left.kind),
        )
            .cmp(&(
                right.user_id.as_str(),
                right.work_date,
                anomaly_rank(right.kind),
            ))
    });
    Ok(items)
}

pub async fn close_month(
    pool: &PgPool,
    year: i32,
    month: u32,
    user_ids: &[String],
    closed_by: &str,
    reason: Option<&str>,
) -> RepositoryResult<(NaiveDate, NaiveDate, i64)> {
    let from = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| WorkScheduleRepositoryError::CorruptData("invalid close month".into()))?;
    let to = end_of_month(from)?;
    let rows_affected = if user_ids.is_empty() {
        sqlx::query(
            "UPDATE resolved_workdays SET locked_at = NOW() \
             WHERE work_date BETWEEN $1 AND $2 AND locked_at IS NULL",
        )
        .bind(from)
        .bind(to)
        .execute(pool)
        .await?
        .rows_affected()
    } else {
        sqlx::query(
            "UPDATE resolved_workdays SET locked_at = NOW() \
             WHERE user_id = ANY($1) AND work_date BETWEEN $2 AND $3 AND locked_at IS NULL",
        )
        .bind(user_ids)
        .bind(from)
        .bind(to)
        .execute(pool)
        .await?
        .rows_affected()
    };

    sqlx::query(
        "INSERT INTO work_schedule_monthly_closures \
         (id, year, month, period_start, period_end, user_ids, locked_count, closed_by, reason) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(Uuid::new_v4())
    .bind(year)
    .bind(i32::try_from(month).unwrap_or(0))
    .bind(from)
    .bind(to)
    .bind(user_ids)
    .bind(i64::try_from(rows_affected).unwrap_or(i64::MAX))
    .bind(closed_by)
    .bind(reason)
    .execute(pool)
    .await?;

    Ok((from, to, i64::try_from(rows_affected).unwrap_or(i64::MAX)))
}

fn dates_inclusive(from: NaiveDate, to: NaiveDate) -> RepositoryResult<Vec<NaiveDate>> {
    if from > to {
        return Err(WorkScheduleRepositoryError::CorruptData(
            "from must be on or before to".into(),
        ));
    }
    let mut dates = Vec::new();
    let mut current = from;
    while current <= to {
        dates.push(current);
        current = current
            .succ_opt()
            .ok_or_else(|| WorkScheduleRepositoryError::CorruptData("date overflow".into()))?;
    }
    Ok(dates)
}

fn end_of_month(first_day: NaiveDate) -> RepositoryResult<NaiveDate> {
    let (next_year, next_month) = if first_day.month() == 12 {
        (first_day.year() + 1, 1)
    } else {
        (first_day.year(), first_day.month() + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|date| date.pred_opt())
        .ok_or_else(|| WorkScheduleRepositoryError::CorruptData("invalid close month".into()))
}

fn anomaly(
    user_id: &str,
    work_date: NaiveDate,
    kind: WorkScheduleAnomalyKind,
    message: &str,
) -> WorkScheduleAnomalyResponse {
    WorkScheduleAnomalyResponse {
        user_id: user_id.to_string(),
        work_date,
        kind,
        message: message.to_string(),
    }
}

fn anomaly_rank(kind: WorkScheduleAnomalyKind) -> u8 {
    match kind {
        WorkScheduleAnomalyKind::ScheduleNotConfigured => 0,
        WorkScheduleAnomalyKind::UnscheduledWork => 1,
        WorkScheduleAnomalyKind::MissingClockIn => 2,
        WorkScheduleAnomalyKind::MissingClockOut => 3,
    }
}
