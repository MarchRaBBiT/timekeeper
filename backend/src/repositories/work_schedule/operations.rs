use std::collections::{HashMap, HashSet};

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use sqlx::{FromRow, PgPool};
use timekeeper_contract::work_schedules::{
    WorkScheduleAnomalyKind, WorkScheduleAnomalyResponse, WorkScheduleCalendarAttendanceResponse,
    WorkScheduleCalendarLeaveResponse,
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

#[derive(Debug, Clone, FromRow)]
struct ApprovedLeaveCalendarRow {
    user_id: String,
    date: NaiveDate,
    leave_request_id: String,
    leave_type: String,
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

pub async fn list_user_leave_calendar(
    pool: &PgPool,
    user_id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> RepositoryResult<HashMap<NaiveDate, WorkScheduleCalendarLeaveResponse>> {
    let rows = list_approved_leave_calendar_rows(pool, &[user_id.to_string()], from, to).await?;
    let mut leave_by_date = HashMap::new();
    for row in rows {
        leave_by_date
            .entry(row.date)
            .or_insert(WorkScheduleCalendarLeaveResponse {
                leave_request_id: row.leave_request_id,
                leave_type: row.leave_type,
            });
    }
    Ok(leave_by_date)
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

    let leave_rows = list_approved_leave_calendar_rows(pool, &users, from, to).await?;
    let mut leave_by_user_date: HashMap<(String, NaiveDate), ApprovedLeaveCalendarRow> =
        HashMap::new();
    for row in leave_rows {
        leave_by_user_date
            .entry((row.user_id.clone(), row.date))
            .or_insert(row);
    }

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
            let has_approved_leave = leave_by_user_date.contains_key(&key);
            if has_approved_leave {
                if attendance.is_some_and(|row| row.clock_in_time.is_some()) {
                    items.push(anomaly(
                        &user_id,
                        work_date,
                        WorkScheduleAnomalyKind::LeaveConflict,
                        "clock-in was recorded on an approved leave day",
                    ));
                }
                continue;
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

async fn list_approved_leave_calendar_rows(
    pool: &PgPool,
    user_ids: &[String],
    from: NaiveDate,
    to: NaiveDate,
) -> RepositoryResult<Vec<ApprovedLeaveCalendarRow>> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<_, ApprovedLeaveCalendarRow>(
        "SELECT lr.user_id, d.day::date AS date, lr.id AS leave_request_id, lr.leave_type
         FROM leave_requests lr
         CROSS JOIN LATERAL generate_series(
             lr.start_date::timestamp, lr.end_date::timestamp, interval '1 day'
         ) AS d(day)
         WHERE lr.user_id = ANY($1)
           AND lr.status = 'approved'
           AND d.day::date BETWEEN $2 AND $3
         ORDER BY lr.user_id, d.day::date, lr.created_at, lr.id",
    )
    .bind(user_ids)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows)
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
    let mut transaction = pool.begin().await?;
    if !user_ids.is_empty() {
        let requested_count = i64::try_from(user_ids.iter().collect::<HashSet<_>>().len())
            .map_err(|_| WorkScheduleRepositoryError::CorruptData("too many user ids".into()))?;
        let existing_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ANY($1)")
                .bind(user_ids)
                .fetch_one(&mut *transaction)
                .await?;
        if existing_count != requested_count {
            return Err(WorkScheduleRepositoryError::InvalidReference);
        }
    }
    let rows_affected = if user_ids.is_empty() {
        sqlx::query(
            "UPDATE resolved_workdays SET locked_at = NOW() \
             WHERE work_date BETWEEN $1 AND $2 AND locked_at IS NULL",
        )
        .bind(from)
        .bind(to)
        .execute(&mut *transaction)
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
        .execute(&mut *transaction)
        .await?
        .rows_affected()
    };
    let locked_count = i64::try_from(rows_affected).map_err(|_| {
        WorkScheduleRepositoryError::CorruptData("locked row count overflow".into())
    })?;

    if locked_count > 0 {
        let month = i32::try_from(month)
            .map_err(|_| WorkScheduleRepositoryError::CorruptData("invalid close month".into()))?;
        sqlx::query(
            "INSERT INTO work_schedule_monthly_closures \
             (id, year, month, period_start, period_end, user_ids, locked_count, closed_by, reason) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(Uuid::new_v4())
        .bind(year)
        .bind(month)
        .bind(from)
        .bind(to)
        .bind(user_ids)
        .bind(locked_count)
        .bind(closed_by)
        .bind(reason)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;
    Ok((from, to, locked_count))
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
        WorkScheduleAnomalyKind::LeaveConflict => 4,
    }
}
