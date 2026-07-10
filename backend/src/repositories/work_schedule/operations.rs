use std::collections::{HashMap, HashSet};

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use timekeeper_contract::work_schedules::{
    MonthlyClosingStatus, MonthlyClosingWorkflowResponse, OvertimeMonitorResponse,
    OvertimeMonitorSettingsRequest, OvertimeMonitorSettingsResponse, OvertimeMonitorStatus,
    OvertimeMonitorUserResponse, WorkScheduleAnomalyKind, WorkScheduleAnomalyResponse,
    WorkScheduleCalendarAttendanceResponse, WorkScheduleCalendarLeaveResponse,
};
use uuid::Uuid;

use super::{map_database_error, RepositoryResult, WorkScheduleRepositoryError};

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
    schedule_type: String,
    expected_work_minutes: i32,
    late_grace_minutes: i32,
    early_leave_grace_minutes: i32,
}

#[derive(Debug, Clone, FromRow)]
struct ResolvedIntervalRow {
    user_id: String,
    work_date: NaiveDate,
    start_time: NaiveTime,
    start_day_offset: i16,
    end_time: NaiveTime,
    end_day_offset: i16,
}

#[derive(Debug, Clone, FromRow)]
struct BreakTotalRow {
    attendance_id: String,
    break_minutes: i64,
}

#[derive(Debug, Clone, FromRow)]
struct ApprovedOvertimeRow {
    user_id: String,
    date: NaiveDate,
    planned_minutes: i64,
}

#[derive(Debug, Clone, Copy, FromRow)]
struct OvertimeMonitorSettingsRow {
    fiscal_year_start_month: i16,
    monthly_limit_minutes: i32,
    yearly_limit_minutes: i32,
    rolling_average_limit_minutes: i32,
    warning_ratio_percent: i16,
    overtime_request_tolerance_minutes: i32,
    break_six_hour_threshold_minutes: i32,
    break_six_hour_minimum_minutes: i32,
    break_eight_hour_threshold_minutes: i32,
    break_eight_hour_minimum_minutes: i32,
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

/// Lists work-schedule anomalies (missing punches, punctuality, absence, breaks,
/// overtime, ...) for `user_ids` (or all users when `None`) between `from` and `to`.
///
/// `today` is the "current date" used to distinguish `Absent` (a scheduled past
/// workday with no attendance or approved leave) from `MissingClockIn` (a scheduled
/// workday that has not happened yet, or is still in progress). The repository layer
/// deliberately does not read the wall clock itself (no `Utc::now()`): callers must
/// resolve "today" in the business timezone (see `crate::utils::time::today_local`
/// with `state.config.time_zone`) and pass it in, so the boundary is both testable
/// and correct for a non-UTC business timezone.
pub async fn list_anomalies(
    pool: &PgPool,
    user_ids: Option<Vec<String>>,
    from: NaiveDate,
    to: NaiveDate,
    today: NaiveDate,
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
        "SELECT rw.user_id, rw.work_date, rw.day_kind, rw.schedule_type,
                rw.expected_work_minutes, wsv.late_grace_minutes, wsv.early_leave_grace_minutes
         FROM resolved_workdays rw
         JOIN work_schedule_versions wsv ON wsv.id = rw.work_schedule_version_id
         WHERE rw.user_id = ANY($1) AND rw.work_date BETWEEN $2 AND $3",
    )
    .bind(&users)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    let resolved: HashMap<_, _> = resolved_rows
        .into_iter()
        .map(|row| ((row.user_id.clone(), row.work_date), row))
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

    let intervals = list_resolved_intervals(pool, &users, from, to).await?;
    let break_totals = list_break_totals(
        pool,
        attendance.values().map(|row| row.id.clone()).collect(),
    )
    .await?;
    let overtime_requests = list_approved_overtime(pool, &users, from, to).await?;
    let settings = find_overtime_monitor_settings(pool, to).await?;

    let leave_rows = list_approved_leave_calendar_rows(pool, &users, from, to).await?;
    let mut leave_by_user_date: HashMap<(String, NaiveDate), ApprovedLeaveCalendarRow> =
        HashMap::new();
    for row in leave_rows {
        leave_by_user_date
            .entry((row.user_id.clone(), row.date))
            .or_insert(row);
    }

    let overtime_context = OvertimeAnomalyContext {
        break_totals: &break_totals,
        overtime_requests: &overtime_requests,
        settings,
    };

    let mut items = Vec::new();
    for user_id in users {
        for work_date in dates_inclusive(from, to)? {
            let key = (user_id.clone(), work_date);
            let Some(resolved_day) = resolved.get(&key) else {
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
            if resolved_day.day_kind == "scheduled_workday" {
                match attendance {
                    None if work_date < today => items.push(anomaly(
                        &user_id,
                        work_date,
                        WorkScheduleAnomalyKind::Absent,
                        "scheduled past workday has no attendance or approved leave",
                    )),
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
                    Some(row) => {
                        add_punctuality_anomalies(
                            &mut items,
                            &user_id,
                            work_date,
                            resolved_day,
                            row,
                            &intervals,
                        );
                        add_break_anomalies(
                            &mut items,
                            &user_id,
                            work_date,
                            row,
                            &break_totals,
                            settings,
                        );
                        add_overtime_anomalies(
                            &mut items,
                            &user_id,
                            work_date,
                            resolved_day,
                            row,
                            &overtime_context,
                        );
                    }
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

async fn list_resolved_intervals(
    pool: &PgPool,
    user_ids: &[String],
    from: NaiveDate,
    to: NaiveDate,
) -> RepositoryResult<HashMap<(String, NaiveDate), Vec<ResolvedIntervalRow>>> {
    let rows = sqlx::query_as::<_, ResolvedIntervalRow>(
        "SELECT rw.user_id, rw.work_date, rwi.start_time, rwi.start_day_offset,
                rwi.end_time, rwi.end_day_offset
         FROM resolved_workday_intervals rwi
         JOIN resolved_workdays rw ON rw.id = rwi.resolved_workday_id
         WHERE rw.user_id = ANY($1) AND rw.work_date BETWEEN $2 AND $3
         ORDER BY rw.user_id, rw.work_date, rwi.sequence",
    )
    .bind(user_ids)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    let mut grouped: HashMap<(String, NaiveDate), Vec<ResolvedIntervalRow>> = HashMap::new();
    for row in rows {
        grouped
            .entry((row.user_id.clone(), row.work_date))
            .or_default()
            .push(row);
    }
    Ok(grouped)
}

async fn list_break_totals(
    pool: &PgPool,
    attendance_ids: Vec<String>,
) -> RepositoryResult<HashMap<String, i64>> {
    if attendance_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, BreakTotalRow>(
        "SELECT attendance_id,
                COALESCE(SUM(EXTRACT(EPOCH FROM (break_end_time - break_start_time)) / 60), 0)::BIGINT
                    AS break_minutes
         FROM break_records
         WHERE attendance_id = ANY($1) AND break_end_time IS NOT NULL
         GROUP BY attendance_id",
    )
    .bind(&attendance_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.attendance_id, row.break_minutes))
        .collect())
}

async fn list_approved_overtime(
    pool: &PgPool,
    user_ids: &[String],
    from: NaiveDate,
    to: NaiveDate,
) -> RepositoryResult<HashMap<(String, NaiveDate), i64>> {
    let rows = sqlx::query_as::<_, ApprovedOvertimeRow>(
        "SELECT user_id, date, ROUND(planned_hours * 60)::BIGINT AS planned_minutes
         FROM overtime_requests
         WHERE user_id = ANY($1) AND status = 'approved' AND date BETWEEN $2 AND $3",
    )
    .bind(user_ids)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| ((row.user_id, row.date), row.planned_minutes))
        .collect())
}

async fn find_overtime_monitor_settings(
    pool: &PgPool,
    as_of: NaiveDate,
) -> RepositoryResult<OvertimeMonitorSettingsRow> {
    sqlx::query_as::<_, OvertimeMonitorSettingsRow>(
        "SELECT fiscal_year_start_month, monthly_limit_minutes, yearly_limit_minutes,
                rolling_average_limit_minutes, warning_ratio_percent,
                overtime_request_tolerance_minutes, break_six_hour_threshold_minutes,
                break_six_hour_minimum_minutes, break_eight_hour_threshold_minutes,
                break_eight_hour_minimum_minutes
         FROM overtime_monitor_settings
         WHERE valid_from <= $1
         ORDER BY valid_from DESC
         LIMIT 1",
    )
    .bind(as_of)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

fn add_punctuality_anomalies(
    items: &mut Vec<WorkScheduleAnomalyResponse>,
    user_id: &str,
    work_date: NaiveDate,
    resolved_day: &ResolvedStateRow,
    attendance: &AttendanceCalendarRow,
    intervals: &HashMap<(String, NaiveDate), Vec<ResolvedIntervalRow>>,
) {
    if resolved_day.schedule_type != "fixed" {
        return;
    }
    let Some(day_intervals) = intervals.get(&(user_id.to_string(), work_date)) else {
        return;
    };
    let Some((planned_start, planned_end)) = planned_span(work_date, day_intervals) else {
        return;
    };
    if let Some(clock_in) = attendance.clock_in_time {
        let late_after =
            planned_start + Duration::minutes(i64::from(resolved_day.late_grace_minutes));
        if clock_in > late_after {
            items.push(anomaly(
                user_id,
                work_date,
                WorkScheduleAnomalyKind::Late,
                "clock-in was later than the planned start time",
            ));
        }
    }
    if let Some(clock_out) = attendance.clock_out_time {
        let early_before =
            planned_end - Duration::minutes(i64::from(resolved_day.early_leave_grace_minutes));
        if clock_out < early_before {
            items.push(anomaly(
                user_id,
                work_date,
                WorkScheduleAnomalyKind::EarlyLeave,
                "clock-out was earlier than the planned end time",
            ));
        }
    }
}

fn add_break_anomalies(
    items: &mut Vec<WorkScheduleAnomalyResponse>,
    user_id: &str,
    work_date: NaiveDate,
    attendance: &AttendanceCalendarRow,
    break_totals: &HashMap<String, i64>,
    settings: OvertimeMonitorSettingsRow,
) {
    let Some(work_minutes) = raw_work_minutes(attendance) else {
        return;
    };
    let break_minutes = break_totals.get(&attendance.id).copied().unwrap_or(0);
    let required = if work_minutes > i64::from(settings.break_eight_hour_threshold_minutes) {
        i64::from(settings.break_eight_hour_minimum_minutes)
    } else if work_minutes > i64::from(settings.break_six_hour_threshold_minutes) {
        i64::from(settings.break_six_hour_minimum_minutes)
    } else {
        0
    };
    if required > 0 && break_minutes < required {
        items.push(anomaly(
            user_id,
            work_date,
            WorkScheduleAnomalyKind::InsufficientBreak,
            "recorded break time is below the configured minimum",
        ));
    }
}

/// Bundles the per-request-scoped lookup tables and settings used by
/// [`add_overtime_anomalies`] so the function stays within a reasonable
/// argument count while still being computed once per `list_anomalies` call.
struct OvertimeAnomalyContext<'a> {
    break_totals: &'a HashMap<String, i64>,
    overtime_requests: &'a HashMap<(String, NaiveDate), i64>,
    settings: OvertimeMonitorSettingsRow,
}

fn add_overtime_anomalies(
    items: &mut Vec<WorkScheduleAnomalyResponse>,
    user_id: &str,
    work_date: NaiveDate,
    resolved_day: &ResolvedStateRow,
    attendance: &AttendanceCalendarRow,
    context: &OvertimeAnomalyContext,
) {
    // `expected_work_minutes` is a net (break-deducted) figure and is only a
    // meaningful contracted duration for fixed schedules; flex schedules use it
    // to represent the flex band width, so overtime cannot be judged against it
    // here. Real flex overtime requires settlement-period-based accounting,
    // which is out of scope for this anomaly detector.
    if resolved_day.schedule_type != "fixed" {
        return;
    }
    let Some(raw_minutes) = raw_work_minutes(attendance) else {
        return;
    };
    let break_minutes = context
        .break_totals
        .get(&attendance.id)
        .copied()
        .unwrap_or(0);
    let work_minutes = (raw_minutes - break_minutes).max(0);
    let daily_overtime = (work_minutes - i64::from(resolved_day.expected_work_minutes)).max(0);
    if daily_overtime <= 0 {
        return;
    }
    let approved = context
        .overtime_requests
        .get(&(user_id.to_string(), work_date))
        .copied()
        .unwrap_or(0);
    let tolerance = i64::from(context.settings.overtime_request_tolerance_minutes);
    if approved == 0 {
        items.push(anomaly(
            user_id,
            work_date,
            WorkScheduleAnomalyKind::UnapprovedOvertime,
            "overtime was recorded without an approved overtime request",
        ));
    } else if daily_overtime > approved + tolerance {
        items.push(anomaly(
            user_id,
            work_date,
            WorkScheduleAnomalyKind::OvertimeExceedsRequest,
            "recorded overtime exceeds the approved request",
        ));
    }
}

fn raw_work_minutes(attendance: &AttendanceCalendarRow) -> Option<i64> {
    let start = attendance.clock_in_time?;
    let end = attendance.clock_out_time?;
    (end > start).then(|| (end - start).num_minutes())
}

fn planned_span(
    work_date: NaiveDate,
    intervals: &[ResolvedIntervalRow],
) -> Option<(NaiveDateTime, NaiveDateTime)> {
    let first = intervals.first()?;
    let last = intervals.last()?;
    Some((
        combine_workday_time(work_date, first.start_time, first.start_day_offset),
        combine_workday_time(work_date, last.end_time, last.end_day_offset),
    ))
}

fn combine_workday_time(work_date: NaiveDate, time: NaiveTime, day_offset: i16) -> NaiveDateTime {
    (work_date + Duration::days(i64::from(day_offset))).and_time(time)
}

pub async fn close_month(
    pool: &PgPool,
    year: i32,
    month: u32,
    user_ids: &[String],
    closed_by: &str,
    reason: Option<&str>,
) -> RepositoryResult<(NaiveDate, NaiveDate, i64)> {
    let mut transaction = pool.begin().await?;
    let result = close_month_tx(&mut transaction, year, month, user_ids, closed_by, reason).await?;
    transaction.commit().await?;
    Ok(result)
}

/// Core `close_month` logic against an already-open transaction. Callers own the
/// begin/commit lifecycle so this can be composed with other writes (e.g. the
/// monthly closing workflow transition) inside a single atomic transaction.
async fn close_month_tx(
    transaction: &mut Transaction<'_, Postgres>,
    year: i32,
    month: u32,
    user_ids: &[String],
    closed_by: &str,
    reason: Option<&str>,
) -> RepositoryResult<(NaiveDate, NaiveDate, i64)> {
    let from = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| WorkScheduleRepositoryError::CorruptData("invalid close month".into()))?;
    let to = end_of_month(from)?;
    if !user_ids.is_empty() {
        let requested_count = i64::try_from(user_ids.iter().collect::<HashSet<_>>().len())
            .map_err(|_| WorkScheduleRepositoryError::CorruptData("too many user ids".into()))?;
        let existing_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ANY($1)")
                .bind(user_ids)
                .fetch_one(&mut **transaction)
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
        .execute(&mut **transaction)
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
        .execute(&mut **transaction)
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
        .execute(&mut **transaction)
        .await?;
    }

    Ok((from, to, locked_count))
}

pub async fn list_overtime_monitor(
    pool: &PgPool,
    user_ids: &[String],
    year: i32,
    month: u32,
) -> RepositoryResult<OvertimeMonitorResponse> {
    let month_start = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
        WorkScheduleRepositoryError::CorruptData("invalid overtime monitor month".into())
    })?;
    let month_end = end_of_month(month_start)?;
    let settings = find_overtime_monitor_settings(pool, month_end).await?;
    let fiscal_start = fiscal_year_start(year, month, settings.fiscal_year_start_month)?;
    let rolling_from = rolling_window_start(month_start, 6)?;
    // The fiscal-year total is computed from this same fetch below, so the
    // fetch window must start no later than the fiscal year start — otherwise
    // months between fiscal_start and rolling_from (e.g. querying in month 12
    // of a fiscal year that started in month 4) would be silently dropped from
    // the fiscal-year total.
    let fetch_from = fiscal_start.min(rolling_from);
    // Aggregated to one row per (user, month) in SQL via GROUP BY, instead of
    // transferring one row per attendance day and summing in Rust. For a
    // 13-month fetch window across many users this avoids shipping and
    // materializing daily rows the caller never needs individually.
    let rows = sqlx::query_as::<_, (String, NaiveDate, i64)>(
        "SELECT a.user_id, date_trunc('month', a.date)::date AS month_start,
                SUM(
                    GREATEST(
                        ROUND(EXTRACT(EPOCH FROM (a.clock_out_time - a.clock_in_time)) / 60)::BIGINT
                        - COALESCE(br.break_minutes, 0)
                        - rw.expected_work_minutes,
                        0
                    )
                )::BIGINT AS overtime_minutes
         FROM attendance a
         JOIN resolved_workdays rw ON rw.user_id = a.user_id AND rw.work_date = a.date
         LEFT JOIN (
             SELECT attendance_id,
                    ROUND(SUM(EXTRACT(EPOCH FROM (break_end_time - break_start_time)) / 60))::BIGINT
                        AS break_minutes
             FROM break_records
             WHERE break_end_time IS NOT NULL
             GROUP BY attendance_id
         ) br ON br.attendance_id = a.id
         WHERE a.user_id = ANY($1)
           AND a.date BETWEEN $2 AND $3
           AND a.clock_in_time IS NOT NULL
           AND a.clock_out_time IS NOT NULL
           AND rw.schedule_type = 'fixed'
         GROUP BY a.user_id, date_trunc('month', a.date)",
    )
    .bind(user_ids)
    .bind(fetch_from)
    .bind(month_end)
    .fetch_all(pool)
    .await?;

    // Grouped by user first so that computing each user's fiscal/rolling
    // totals below only scans that user's own (small, bounded by the
    // fetch window's ~13 months) entries, instead of re-scanning every
    // user's rows for every user (which made the previous shape O(U^2*M)).
    let mut by_user: HashMap<String, HashMap<(i32, u32), i64>> = HashMap::new();
    for (user_id, month_start_date, minutes) in rows {
        by_user
            .entry(user_id)
            .or_default()
            .insert((month_start_date.year(), month_start_date.month()), minutes);
    }

    let mut items = Vec::new();
    for user_id in user_ids {
        let user_months = by_user.get(user_id);
        let month_minutes = user_months
            .and_then(|months| months.get(&(year, month)))
            .copied()
            .unwrap_or(0);
        let fiscal_minutes: i64 = user_months
            .map(|months| {
                months
                    .iter()
                    .filter(|((row_year, row_month), _)| {
                        month_start_for(*row_year, *row_month)
                            .is_some_and(|date| date >= fiscal_start && date <= month_start)
                    })
                    .map(|(_, value)| *value)
                    .sum()
            })
            .unwrap_or(0);
        let rolling_months = months_between_inclusive(rolling_from, month_start)?;
        let rolling_minutes: i64 = user_months
            .map(|months| {
                months
                    .iter()
                    .filter(|((row_year, row_month), _)| {
                        month_start_for(*row_year, *row_month)
                            .is_some_and(|date| date >= rolling_from && date <= month_start)
                    })
                    .map(|(_, value)| *value)
                    .sum()
            })
            .unwrap_or(0);
        let rolling_average = if rolling_months == 0 {
            0
        } else {
            rolling_minutes / rolling_months
        };
        items.push(OvertimeMonitorUserResponse {
            user_id: user_id.clone(),
            month_statutory_excess_minutes: month_minutes,
            fiscal_year_statutory_excess_minutes: fiscal_minutes,
            rolling_average_statutory_excess_minutes: rolling_average,
            monthly_status: threshold_status(
                month_minutes,
                i64::from(settings.monthly_limit_minutes),
                settings.warning_ratio_percent,
            ),
            yearly_status: threshold_status(
                fiscal_minutes,
                i64::from(settings.yearly_limit_minutes),
                settings.warning_ratio_percent,
            ),
            rolling_average_status: threshold_status(
                rolling_average,
                i64::from(settings.rolling_average_limit_minutes),
                settings.warning_ratio_percent,
            ),
        });
    }
    items.sort_by(|left, right| left.user_id.cmp(&right.user_id));
    Ok(OvertimeMonitorResponse {
        year,
        month,
        fiscal_year_start_month: u32::try_from(settings.fiscal_year_start_month).map_err(|_| {
            WorkScheduleRepositoryError::CorruptData("invalid fiscal year start month".into())
        })?,
        items,
    })
}

pub async fn get_overtime_monitor_settings(
    pool: &PgPool,
    as_of: NaiveDate,
) -> RepositoryResult<OvertimeMonitorSettingsResponse> {
    let row = sqlx::query_as::<_, (Uuid, NaiveDate, i16, i32, i32, i32, i32, i16, i32)>(
        "SELECT id, valid_from, fiscal_year_start_month, monthly_limit_minutes,
                yearly_limit_minutes, rolling_average_limit_minutes,
                single_month_absolute_limit_minutes, warning_ratio_percent,
                overtime_request_tolerance_minutes
         FROM overtime_monitor_settings
         WHERE valid_from <= $1
         ORDER BY valid_from DESC
         LIMIT 1",
    )
    .bind(as_of)
    .fetch_one(pool)
    .await?;
    overtime_settings_response(row)
}

pub async fn upsert_overtime_monitor_settings(
    pool: &PgPool,
    request: &OvertimeMonitorSettingsRequest,
) -> RepositoryResult<OvertimeMonitorSettingsResponse> {
    let row =
        sqlx::query_as::<_, (Uuid, NaiveDate, i16, i32, i32, i32, i32, i16, i32)>(
            "INSERT INTO overtime_monitor_settings (
             valid_from, fiscal_year_start_month, monthly_limit_minutes, yearly_limit_minutes,
             rolling_average_limit_minutes, single_month_absolute_limit_minutes,
             warning_ratio_percent, overtime_request_tolerance_minutes
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (valid_from) DO UPDATE SET
             fiscal_year_start_month = EXCLUDED.fiscal_year_start_month,
             monthly_limit_minutes = EXCLUDED.monthly_limit_minutes,
             yearly_limit_minutes = EXCLUDED.yearly_limit_minutes,
             rolling_average_limit_minutes = EXCLUDED.rolling_average_limit_minutes,
             single_month_absolute_limit_minutes = EXCLUDED.single_month_absolute_limit_minutes,
             warning_ratio_percent = EXCLUDED.warning_ratio_percent,
             overtime_request_tolerance_minutes = EXCLUDED.overtime_request_tolerance_minutes,
             updated_at = NOW()
         RETURNING id, valid_from, fiscal_year_start_month, monthly_limit_minutes,
             yearly_limit_minutes, rolling_average_limit_minutes,
             single_month_absolute_limit_minutes, warning_ratio_percent,
             overtime_request_tolerance_minutes",
        )
        .bind(request.valid_from)
        .bind(i16::try_from(request.fiscal_year_start_month).map_err(|_| {
            WorkScheduleRepositoryError::CorruptData("invalid fiscal year start month".into())
        })?)
        .bind(i32::try_from(request.monthly_limit_minutes).map_err(|_| {
            WorkScheduleRepositoryError::CorruptData("monthly limit overflow".into())
        })?)
        .bind(i32::try_from(request.yearly_limit_minutes).map_err(|_| {
            WorkScheduleRepositoryError::CorruptData("yearly limit overflow".into())
        })?)
        .bind(
            i32::try_from(request.rolling_average_limit_minutes).map_err(|_| {
                WorkScheduleRepositoryError::CorruptData("rolling average limit overflow".into())
            })?,
        )
        .bind(
            i32::try_from(request.single_month_absolute_limit_minutes).map_err(|_| {
                WorkScheduleRepositoryError::CorruptData("single month limit overflow".into())
            })?,
        )
        .bind(i16::try_from(request.warning_ratio_percent).map_err(|_| {
            WorkScheduleRepositoryError::CorruptData("warning ratio overflow".into())
        })?)
        .bind(
            i32::try_from(request.overtime_request_tolerance_minutes).map_err(|_| {
                WorkScheduleRepositoryError::CorruptData("overtime tolerance overflow".into())
            })?,
        )
        .fetch_one(pool)
        .await?;
    overtime_settings_response(row)
}

pub async fn transition_monthly_closing(
    pool: &PgPool,
    user_id: &str,
    year: i32,
    month: u32,
    to_status: MonthlyClosingStatus,
    actor_id: &str,
    reason: Option<&str>,
) -> RepositoryResult<MonthlyClosingWorkflowResponse> {
    let mut transaction = pool.begin().await?;
    let response = transition_monthly_closing_tx(
        &mut transaction,
        user_id,
        year,
        month,
        to_status,
        actor_id,
        reason,
    )
    .await?;
    transaction.commit().await?;
    Ok(response)
}

/// Core `transition_monthly_closing` logic against an already-open transaction.
/// Callers own the begin/commit lifecycle so this can be composed with other
/// writes (e.g. locking resolved workdays on close) inside a single atomic
/// transaction.
async fn transition_monthly_closing_tx(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: &str,
    year: i32,
    month: u32,
    to_status: MonthlyClosingStatus,
    actor_id: &str,
    reason: Option<&str>,
) -> RepositoryResult<MonthlyClosingWorkflowResponse> {
    let month_i32 = i32::try_from(month)
        .map_err(|_| WorkScheduleRepositoryError::CorruptData("invalid close month".into()))?;
    let existing = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, status FROM monthly_closing_workflows
         WHERE user_id = $1 AND year = $2 AND month = $3
         FOR UPDATE",
    )
    .bind(user_id)
    .bind(year)
    .bind(month_i32)
    .fetch_optional(&mut **transaction)
    .await?;
    let (workflow_id, from_status) = match existing {
        Some((id, status)) => (id, status),
        None => {
            // Two concurrent first-requests for the same user/year/month (e.g. a
            // double-click on self-confirm) can both observe `existing = None`
            // above, since there is no row yet for `SELECT ... FOR UPDATE` to
            // lock. Use `ON CONFLICT DO NOTHING` instead of a bare INSERT so the
            // loser of that race doesn't surface a raw unique-violation 500; it
            // instead falls through to re-reading the row the winner created,
            // under `FOR UPDATE`, and continues through the normal transition
            // check below (which will correctly reject/accept based on the
            // now-current status).
            let inserted = sqlx::query_as::<_, (Uuid, String)>(
                "INSERT INTO monthly_closing_workflows (id, user_id, year, month, status)
                 VALUES ($1, $2, $3, $4, 'open')
                 ON CONFLICT (user_id, year, month) DO NOTHING
                 RETURNING id, status",
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(year)
            .bind(month_i32)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(map_database_error)?;
            match inserted {
                Some((id, status)) => (id, status),
                None => {
                    sqlx::query_as::<_, (Uuid, String)>(
                        "SELECT id, status FROM monthly_closing_workflows
                     WHERE user_id = $1 AND year = $2 AND month = $3
                     FOR UPDATE",
                    )
                    .bind(user_id)
                    .bind(year)
                    .bind(month_i32)
                    .fetch_one(&mut **transaction)
                    .await?
                }
            }
        }
    };
    let to_status_db = monthly_status_to_db(to_status);
    if !is_valid_monthly_transition(&from_status, to_status_db) {
        return Err(WorkScheduleRepositoryError::InvalidStateTransition);
    }
    let row = sqlx::query_as::<_, (Uuid, String, i32, i32, String, Option<String>)>(
        "UPDATE monthly_closing_workflows
         SET status = $1,
             self_confirmed_by = CASE WHEN $1 = 'self_confirmed' THEN $2 ELSE self_confirmed_by END,
             self_confirmed_at = CASE WHEN $1 = 'self_confirmed' THEN NOW() ELSE self_confirmed_at END,
             approved_by = CASE WHEN $1 = 'approved' THEN $2 ELSE approved_by END,
             approved_at = CASE WHEN $1 = 'approved' THEN NOW() ELSE approved_at END,
             closed_by = CASE WHEN $1 = 'closed' THEN $2 ELSE closed_by END,
             closed_at = CASE WHEN $1 = 'closed' THEN NOW() ELSE closed_at END,
             reopened_by = CASE WHEN $1 = 'reopened' THEN $2 ELSE reopened_by END,
             reopened_at = CASE WHEN $1 = 'reopened' THEN NOW() ELSE reopened_at END,
             reason = $3,
             updated_at = NOW()
         WHERE id = $4
         RETURNING id, user_id, year, month, status, reason",
    )
    .bind(to_status_db)
    .bind(actor_id)
    .bind(reason)
    .bind(workflow_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    sqlx::query(
        "INSERT INTO monthly_closing_workflow_events
         (workflow_id, from_status, to_status, acted_by, reason)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(workflow_id)
    .bind(&from_status)
    .bind(to_status_db)
    .bind(actor_id)
    .bind(reason)
    .execute(&mut **transaction)
    .await?;
    monthly_workflow_response(row)
}

/// Transitions a monthly closing workflow to `closed` and locks the
/// corresponding resolved workdays in a single transaction. This guarantees
/// the design invariant that a `closed` workflow always has its resolved
/// workdays locked: if either half fails, the whole transition rolls back
/// instead of leaving the workflow `closed` with unlocked workdays (which
/// would be unrecoverable via the API, since `closed -> closed` is not a
/// valid transition).
pub async fn close_monthly_closing_workflow(
    pool: &PgPool,
    user_id: &str,
    year: i32,
    month: u32,
    actor_id: &str,
    reason: Option<&str>,
) -> RepositoryResult<MonthlyClosingWorkflowResponse> {
    let mut transaction = pool.begin().await?;
    let response = transition_monthly_closing_tx(
        &mut transaction,
        user_id,
        year,
        month,
        MonthlyClosingStatus::Closed,
        actor_id,
        reason,
    )
    .await?;
    let user_ids = [user_id.to_string()];
    close_month_tx(&mut transaction, year, month, &user_ids, actor_id, reason).await?;
    transaction.commit().await?;
    Ok(response)
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
        WorkScheduleAnomalyKind::UnapprovedOvertime => 5,
        WorkScheduleAnomalyKind::OvertimeExceedsRequest => 6,
        WorkScheduleAnomalyKind::Late => 7,
        WorkScheduleAnomalyKind::EarlyLeave => 8,
        WorkScheduleAnomalyKind::Absent => 9,
        WorkScheduleAnomalyKind::InsufficientBreak => 10,
    }
}

fn threshold_status(value: i64, limit: i64, warning_ratio_percent: i16) -> OvertimeMonitorStatus {
    if value >= limit {
        return OvertimeMonitorStatus::Exceeded;
    }
    let warning_at = limit.saturating_mul(i64::from(warning_ratio_percent)) / 100;
    if value >= warning_at {
        OvertimeMonitorStatus::Warning
    } else {
        OvertimeMonitorStatus::Ok
    }
}

fn fiscal_year_start(
    year: i32,
    month: u32,
    fiscal_start_month: i16,
) -> RepositoryResult<NaiveDate> {
    let fiscal_month = u32::try_from(fiscal_start_month).map_err(|_| {
        WorkScheduleRepositoryError::CorruptData("invalid fiscal year start month".into())
    })?;
    let fiscal_year = if month < fiscal_month { year - 1 } else { year };
    NaiveDate::from_ymd_opt(fiscal_year, fiscal_month, 1).ok_or_else(|| {
        WorkScheduleRepositoryError::CorruptData("invalid fiscal year start month".into())
    })
}

fn rolling_window_start(month_start: NaiveDate, months: u32) -> RepositoryResult<NaiveDate> {
    let mut year = month_start.year();
    let mut month = i32::try_from(month_start.month())
        .map_err(|_| WorkScheduleRepositoryError::CorruptData("invalid month".into()))?;
    for _ in 1..months {
        month -= 1;
        if month == 0 {
            month = 12;
            year -= 1;
        }
    }
    NaiveDate::from_ymd_opt(
        year,
        u32::try_from(month)
            .map_err(|_| WorkScheduleRepositoryError::CorruptData("invalid month".into()))?,
        1,
    )
    .ok_or_else(|| WorkScheduleRepositoryError::CorruptData("invalid rolling window".into()))
}

fn month_start_for(year: i32, month: u32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, 1)
}

fn months_between_inclusive(from: NaiveDate, to: NaiveDate) -> RepositoryResult<i64> {
    if from > to {
        return Err(WorkScheduleRepositoryError::CorruptData(
            "invalid rolling window".into(),
        ));
    }
    Ok(
        i64::from(to.year() - from.year()) * 12 + i64::from(to.month()) - i64::from(from.month())
            + 1,
    )
}

fn overtime_settings_response(
    row: (Uuid, NaiveDate, i16, i32, i32, i32, i32, i16, i32),
) -> RepositoryResult<OvertimeMonitorSettingsResponse> {
    Ok(OvertimeMonitorSettingsResponse {
        id: row.0.to_string(),
        valid_from: row.1,
        fiscal_year_start_month: u32::try_from(row.2).map_err(|_| {
            WorkScheduleRepositoryError::CorruptData("invalid fiscal year start month".into())
        })?,
        monthly_limit_minutes: i64::from(row.3),
        yearly_limit_minutes: i64::from(row.4),
        rolling_average_limit_minutes: i64::from(row.5),
        single_month_absolute_limit_minutes: i64::from(row.6),
        warning_ratio_percent: i32::from(row.7),
        overtime_request_tolerance_minutes: i64::from(row.8),
    })
}

fn monthly_status_to_db(status: MonthlyClosingStatus) -> &'static str {
    match status {
        MonthlyClosingStatus::Open => "open",
        MonthlyClosingStatus::SelfConfirmed => "self_confirmed",
        MonthlyClosingStatus::Approved => "approved",
        MonthlyClosingStatus::Closed => "closed",
        MonthlyClosingStatus::Reopened => "reopened",
    }
}

fn monthly_status_from_db(value: &str) -> RepositoryResult<MonthlyClosingStatus> {
    match value {
        "open" => Ok(MonthlyClosingStatus::Open),
        "self_confirmed" => Ok(MonthlyClosingStatus::SelfConfirmed),
        "approved" => Ok(MonthlyClosingStatus::Approved),
        "closed" => Ok(MonthlyClosingStatus::Closed),
        "reopened" => Ok(MonthlyClosingStatus::Reopened),
        _ => Err(WorkScheduleRepositoryError::CorruptData(
            "invalid monthly closing status".into(),
        )),
    }
}

fn is_valid_monthly_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("open", "self_confirmed")
            | ("self_confirmed", "approved")
            | ("approved", "closed")
            | ("closed", "reopened")
            | ("reopened", "closed")
    )
}

fn monthly_workflow_response(
    row: (Uuid, String, i32, i32, String, Option<String>),
) -> RepositoryResult<MonthlyClosingWorkflowResponse> {
    Ok(MonthlyClosingWorkflowResponse {
        id: row.0.to_string(),
        user_id: row.1,
        year: row.2,
        month: u32::try_from(row.3)
            .map_err(|_| WorkScheduleRepositoryError::CorruptData("invalid month".into()))?,
        status: monthly_status_from_db(&row.4)?,
        reason: row.5,
    })
}
