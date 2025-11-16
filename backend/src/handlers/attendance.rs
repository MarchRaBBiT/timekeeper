use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder, Row};
use std::sync::Arc;

use crate::handlers::attendance_utils::{
    ensure_clock_in_exists, ensure_clocked_in, ensure_not_clocked_in, ensure_not_clocked_out,
    ensure_owned, error_response, fetch_attendance_by_id, fetch_attendance_by_user_date,
    insert_attendance_record, update_clock_in, update_clock_out,
};
use crate::{
    config::Config,
    models::{
        attendance::{
            Attendance, AttendanceResponse, AttendanceSummary, ClockInRequest, ClockOutRequest,
        },
        break_record::{BreakRecord, BreakRecordResponse},
        user::User,
    },
    services::holiday::HolidayService,
    utils::{csv::append_csv_row, time},
};

#[derive(Debug, Deserialize)]
pub struct AttendanceQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct AttendanceExportQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttendanceStatusResponse {
    pub status: String,
    pub attendance_id: Option<String>,
    pub active_break_id: Option<String>,
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
}

pub async fn clock_in(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<HolidayService>>,
    Json(payload): Json<ClockInRequest>,
) -> Result<Json<AttendanceResponse>, (StatusCode, Json<Value>)> {
    let user_id = user.id.as_str();

    let tz = &config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let date = payload.date.unwrap_or_else(|| now_local.date_naive());
    let clock_in_time = now_local.naive_local();

    reject_if_holiday(&holiday_service, date, user_id).await?;

    let attendance = match fetch_attendance_by_user_date(&pool, user_id, date).await? {
        Some(mut attendance) => {
            ensure_not_clocked_in(&attendance)?;
            attendance.clock_in_time = Some(clock_in_time);
            attendance.updated_at = now_utc;
            update_clock_in(&pool, &attendance).await?;
            attendance
        }
        None => {
            let mut attendance = Attendance::new(user_id.to_string(), date, now_utc);
            attendance.clock_in_time = Some(clock_in_time);
            insert_attendance_record(&pool, &attendance).await?;
            attendance
        }
    };

    let break_records = get_break_records(&pool, &attendance.id).await?;
    let response = build_attendance_response(attendance, break_records);

    Ok(Json(response))
}

pub async fn clock_out(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Extension(holiday_service): Extension<Arc<HolidayService>>,
    Json(payload): Json<ClockOutRequest>,
) -> Result<Json<AttendanceResponse>, (StatusCode, Json<Value>)> {
    let user_id = user.id.as_str();

    let tz = &config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let date = payload.date.unwrap_or_else(|| now_local.date_naive());
    let clock_out_time = now_local.naive_local();

    reject_if_holiday(&holiday_service, date, user_id).await?;

    let mut attendance = fetch_attendance_by_user_date(&pool, user_id, date)
        .await?
        .ok_or_else(|| {
            error_response(
                StatusCode::NOT_FOUND,
                "No attendance record found for today",
            )
        })?;

    ensure_not_clocked_out(&attendance)?;
    ensure_clock_in_exists(&attendance)?;

    attendance.clock_out_time = Some(clock_out_time);
    let break_minutes = total_break_minutes(&pool, &attendance.id).await?;
    attendance.calculate_work_hours(break_minutes);
    attendance.updated_at = now_utc;

    update_clock_out(&pool, &attendance).await?;

    let break_records = get_break_records(&pool, &attendance.id).await?;
    let response = build_attendance_response(attendance, break_records);

    Ok(Json(response))
}

pub async fn break_start(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<crate::models::attendance::BreakStartRequest>,
) -> Result<Json<BreakRecordResponse>, (StatusCode, Json<Value>)> {
    let tz = &config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let break_start_time = now_local.naive_local();

    // Check if attendance record exists and user is clocked in
    let attendance = fetch_attendance_by_id(&pool, &payload.attendance_id).await?;
    ensure_owned(&attendance, &user.id)?;
    ensure_clocked_in(&attendance)?;

    // Check if there's already an active break
    let active_break = sqlx::query_as::<_, BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE attendance_id = $1 AND break_end_time IS NULL"
    )
    .bind(&payload.attendance_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    if active_break.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Break already in progress"})),
        ));
    }

    let break_record = BreakRecord::new(payload.attendance_id, break_start_time, now_utc);

    sqlx::query(
                "INSERT INTO break_records (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(&break_record.id)
    .bind(&break_record.attendance_id)
    .bind(&break_record.break_start_time)
    .bind(&break_record.break_end_time)
    .bind(&break_record.duration_minutes)
    .bind(&break_record.created_at)
    .bind(&break_record.updated_at)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create break record"})),
        )
    })?;

    let response = BreakRecordResponse::from(break_record);
    Ok(Json(response))
}

pub async fn break_end(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<crate::models::attendance::BreakEndRequest>,
) -> Result<Json<BreakRecordResponse>, (StatusCode, Json<Value>)> {
    let tz = &config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let break_end_time = now_local.naive_local();

    // Find the break record
    let mut break_record = sqlx::query_as::<_, BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE id = $1"
    )
    .bind(&payload.break_record_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Break record not found"})),
        )
    })?;

    if !break_record.is_active() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Break already ended"})),
        ));
    }
    let att = fetch_attendance_by_id(&pool, &break_record.attendance_id).await?;
    ensure_owned(&att, &user.id)?;

    break_record.end_break(break_end_time, now_utc);

    sqlx::query(
        "UPDATE break_records SET break_end_time = $1, duration_minutes = $2, updated_at = $3 WHERE id = $4"
    )
    .bind(&break_record.break_end_time)
    .bind(&break_record.duration_minutes)
    .bind(&break_record.updated_at)
    .bind(&break_record.id)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update break record"})),
        )
    })?;

    if att.clock_out_time.is_some() {
        recalculate_total_hours(&pool, att, now_utc).await?;
    }

    let response = BreakRecordResponse::from(break_record);
    Ok(Json(response))
}

pub async fn get_my_attendance(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceQuery>,
) -> Result<Json<Vec<AttendanceResponse>>, (StatusCode, Json<Value>)> {
    let user_id = user.id.as_str();

    // from/to range takes precedence over year/month
    let attendances = if params.from.is_some() || params.to.is_some() {
        let tz = &config.time_zone;
        let from = params.from.unwrap_or_else(|| time::today_local(tz));
        let to = params.to.unwrap_or_else(|| time::today_local(tz));
        if from > to {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "from must be <= to"})),
            ));
        }
        sqlx::query_as::<_, Attendance>(
            "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE user_id = $1 AND date BETWEEN $2 AND $3 ORDER BY date DESC"
        )
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_all(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
    } else {
        let tz = &config.time_zone;
        let now_local = time::now_in_timezone(tz);
        let year = params.year.unwrap_or_else(|| now_local.year());
        let month = params.month.unwrap_or_else(|| now_local.month());
        let Some(first_day) = NaiveDate::from_ymd_opt(year, month, 1) else {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid year/month provided"})),
            ));
        };
        let Some(last_day) = first_day
            .checked_add_months(Months::new(1))
            .and_then(|d| d.checked_sub_signed(Duration::days(1)))
        else {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid year/month provided"})),
            ));
        };
        sqlx::query_as::<_, Attendance>(
            "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE user_id = $1 AND date BETWEEN $2 AND $3 ORDER BY date DESC"
        )
        .bind(user_id)
        .bind(first_day)
        .bind(last_day)
        .fetch_all(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
    };

    let mut responses = Vec::new();
    for attendance in attendances {
        let break_records = get_break_records(&pool, &attendance.id).await?;
        responses.push(build_attendance_response(attendance, break_records));
    }

    Ok(Json(responses))
}

pub async fn get_attendance_status(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AttendanceStatusResponse>, (StatusCode, Json<Value>)> {
    let user_id = user.id.as_str();
    let date = params
        .get("date")
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| time::today_local(&config.time_zone));

    let attendance = sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE user_id = $1 AND date = $2"
    )
    .bind(&user_id)
    .bind(&date)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))?;

    if let Some(att) = attendance {
        // Check active break
        let active_break = sqlx::query(
            "SELECT id FROM break_records WHERE attendance_id = $1 AND break_end_time IS NULL ORDER BY break_start_time DESC LIMIT 1"
        )
        .bind(&att.id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))?;

        let resp = if att.clock_in_time.is_none() {
            AttendanceStatusResponse {
                status: "not_started".into(),
                attendance_id: Some(att.id),
                active_break_id: None,
                clock_in_time: None,
                clock_out_time: None,
            }
        } else if att.is_clocked_out() {
            AttendanceStatusResponse {
                status: "clocked_out".into(),
                attendance_id: Some(att.id),
                active_break_id: None,
                clock_in_time: att.clock_in_time,
                clock_out_time: att.clock_out_time,
            }
        } else if let Some(b) = active_break {
            let bid: Option<String> = b.try_get("id").ok();
            AttendanceStatusResponse {
                status: "on_break".into(),
                attendance_id: Some(att.id),
                active_break_id: bid,
                clock_in_time: att.clock_in_time,
                clock_out_time: None,
            }
        } else {
            AttendanceStatusResponse {
                status: "clocked_in".into(),
                attendance_id: Some(att.id),
                active_break_id: None,
                clock_in_time: att.clock_in_time,
                clock_out_time: None,
            }
        };
        Ok(Json(resp))
    } else {
        Ok(Json(AttendanceStatusResponse {
            status: "not_started".into(),
            attendance_id: None,
            active_break_id: None,
            clock_in_time: None,
            clock_out_time: None,
        }))
    }
}

pub async fn get_breaks_by_attendance(
    State((pool, _config)): State<(PgPool, Config)>,
    Path(attendance_id): Path<String>,
) -> Result<Json<Vec<BreakRecordResponse>>, (StatusCode, Json<Value>)> {
    let records = get_break_records(&pool, &attendance_id).await?;
    Ok(Json(records))
}

pub async fn get_my_summary(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceQuery>,
) -> Result<Json<AttendanceSummary>, (StatusCode, Json<Value>)> {
    let user_id = user.id.as_str();

    let now_local = time::now_in_timezone(&config.time_zone);
    let year = params.year.unwrap_or_else(|| now_local.year());
    let month = params.month.unwrap_or_else(|| now_local.month());

    use sqlx::Row;

    let Some(first_day) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid year/month provided"})),
        ));
    };
    let Some(last_day) = first_day
        .checked_add_months(Months::new(1))
        .and_then(|d| d.checked_sub_signed(Duration::days(1)))
    else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid year/month provided"})),
        ));
    };

    let row = sqlx::query(
        "SELECT COALESCE(SUM(total_work_hours), 0) as total_hours, COUNT(*) as total_days FROM attendance WHERE user_id = $1 AND date BETWEEN $2 AND $3 AND total_work_hours IS NOT NULL"
    )
    .bind(user_id)
    .bind(first_day)
    .bind(last_day)
    .fetch_one(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    let total_work_hours: f64 = row.try_get::<f64, _>("total_hours").unwrap_or(0.0);
    let total_work_days: i32 = row.try_get::<i64, _>("total_days").unwrap_or(0) as i32;
    let average_daily_hours = if total_work_days > 0 {
        total_work_hours / total_work_days as f64
    } else {
        0.0
    };

    let summary = AttendanceSummary {
        month,
        year,
        total_work_hours,
        total_work_days,
        average_daily_hours,
    };

    Ok(Json(summary))
}

pub async fn export_my_attendance(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Query(params): Query<AttendanceExportQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let from = params.from;
    let to = params.to;

    if let (Some(f), Some(t)) = (from, to) {
        if f > t {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "from must be <= to"})),
            ));
        }
    }

    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT u.username, u.full_name, a.date, a.clock_in_time, a.clock_out_time, \
                a.total_work_hours, a.status \
         FROM attendance a \
         JOIN users u ON a.user_id = u.id \
         WHERE a.user_id = ",
    );
    builder.push_bind(&user.id);
    if let Some(f) = from {
        builder.push(" AND a.date >= ").push_bind(f);
    }
    if let Some(t) = to {
        builder.push(" AND a.date <= ").push_bind(t);
    }
    builder.push(" ORDER BY a.date DESC");

    let rows = builder.build().fetch_all(&pool).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    let mut csv_data = String::new();
    append_csv_row(
        &mut csv_data,
        &[
            "Username".to_string(),
            "Full Name".to_string(),
            "Date".to_string(),
            "Clock In".to_string(),
            "Clock Out".to_string(),
            "Total Hours".to_string(),
            "Status".to_string(),
        ],
    );
    for row in rows {
        let username = row.try_get::<String, _>("username").unwrap_or_default();
        let full_name = row.try_get::<String, _>("full_name").unwrap_or_default();
        let date = row
            .try_get::<NaiveDate, _>("date")
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        let clock_in = row
            .try_get::<Option<NaiveDateTime>, _>("clock_in_time")
            .ok()
            .flatten()
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_default();
        let clock_out = row
            .try_get::<Option<NaiveDateTime>, _>("clock_out_time")
            .ok()
            .flatten()
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_default();
        let total_hours = row
            .try_get::<f64, _>("total_work_hours")
            .map(|h| format!("{:.2}", h))
            .unwrap_or_else(|_| "0.00".to_string());
        let status = row.try_get::<String, _>("status").unwrap_or_default();

        append_csv_row(
            &mut csv_data,
            &[
                username,
                full_name,
                date,
                clock_in,
                clock_out,
                total_hours,
                status,
            ],
        );
    }

    Ok(Json(json!({
        "csv_data": csv_data,
        "filename": format!(
            "my_attendance_export_{}.csv",
            time::now_in_timezone(&config.time_zone).format("%Y%m%d_%H%M%S")
        )
    })))
}

fn build_attendance_response(
    attendance: Attendance,
    break_records: Vec<BreakRecordResponse>,
) -> AttendanceResponse {
    AttendanceResponse {
        id: attendance.id,
        user_id: attendance.user_id,
        date: attendance.date,
        clock_in_time: attendance.clock_in_time,
        clock_out_time: attendance.clock_out_time,
        status: attendance.status,
        total_work_hours: attendance.total_work_hours,
        break_records,
    }
}

async fn get_break_records(
    pool: &PgPool,
    attendance_id: &str,
) -> Result<Vec<BreakRecordResponse>, (StatusCode, Json<Value>)> {
    let break_records = sqlx::query_as::<_, BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE attendance_id = $1 ORDER BY break_start_time"
    )
    .bind(attendance_id)
    .fetch_all(pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    Ok(break_records
        .into_iter()
        .map(BreakRecordResponse::from)
        .collect())
}

pub(crate) async fn total_break_minutes(
    pool: &PgPool,
    attendance_id: &str,
) -> Result<i64, (StatusCode, Json<Value>)> {
    let row = sqlx::query(
        "SELECT COALESCE(SUM(duration_minutes), 0) AS minutes FROM break_records WHERE attendance_id = $1 AND duration_minutes IS NOT NULL",
    )
    .bind(attendance_id)
    .fetch_one(pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    let minutes = row.try_get::<i64, _>("minutes").unwrap_or(0);
    Ok(minutes.max(0))
}

pub(crate) async fn recalculate_total_hours(
    pool: &PgPool,
    mut attendance: Attendance,
    updated_at: DateTime<Utc>,
) -> Result<(), (StatusCode, Json<Value>)> {
    if attendance.clock_in_time.is_none() || attendance.clock_out_time.is_none() {
        return Ok(());
    }

    let break_minutes = total_break_minutes(pool, &attendance.id).await?;
    attendance.calculate_work_hours(break_minutes);
    attendance.updated_at = updated_at;

    sqlx::query("UPDATE attendance SET total_work_hours = $1, updated_at = $2 WHERE id = $3")
        .bind(&attendance.total_work_hours)
        .bind(&attendance.updated_at)
        .bind(&attendance.id)
        .execute(pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update attendance"})),
            )
        })?;

    Ok(())
}

async fn reject_if_holiday(
    holiday_service: &HolidayService,
    date: NaiveDate,
    user_id: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    let decision = holiday_service
        .is_holiday(date, Some(user_id))
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Failed to evaluate holiday calendar"})),
            )
        })?;

    if decision.is_holiday {
        let reason = decision.reason.label();
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": format!(
                "{} is a {}. Submit an overtime request before clocking in/out.",
                date, reason
            )})),
        ));
    }

    Ok(())
}
