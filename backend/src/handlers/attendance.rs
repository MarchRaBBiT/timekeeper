use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{Datelike, Duration, Months, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    config::Config,
    middleware::auth::get_current_user,
    models::{
        attendance::{
            Attendance, AttendanceResponse, AttendanceSummary, ClockInRequest, ClockOutRequest,
        },
        break_record::{BreakRecord, BreakRecordResponse},
        user::User,
    },
    utils::time,
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
    Json(payload): Json<ClockInRequest>,
) -> Result<Json<AttendanceResponse>, (StatusCode, Json<Value>)> {
    let user_id = user.id.as_str();

    let tz = &config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let date = payload.date.unwrap_or_else(|| now_local.date_naive());
    let clock_in_time = now_local.naive_local();

    // Check if attendance record already exists for this date
    let existing_attendance = sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE user_id = $1 AND date = $2"
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    let attendance = match existing_attendance {
        Some(mut attendance) => {
            if attendance.clock_in_time.is_some() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Already clocked in today"})),
                ));
            }
            attendance.clock_in_time = Some(clock_in_time);
            attendance.updated_at = now_utc;

            sqlx::query("UPDATE attendance SET clock_in_time = $1, updated_at = $2 WHERE id = $3")
                .bind(&attendance.clock_in_time)
                .bind(&attendance.updated_at)
                .bind(&attendance.id)
                .execute(&pool)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Failed to update attendance"})),
                    )
                })?;

            attendance
        }
        None => {
            let mut attendance = Attendance::new(user_id.to_string(), date, now_utc);
            attendance.clock_in_time = Some(clock_in_time);

            sqlx::query(
                "INSERT INTO attendance (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
            )
            .bind(&attendance.id)
            .bind(&attendance.user_id)
            .bind(&attendance.date)
            .bind(&attendance.clock_in_time)
            .bind(&attendance.clock_out_time)
            .bind(status_to_str(&attendance.status))
            .bind(&attendance.total_work_hours)
            .bind(&attendance.created_at)
            .bind(&attendance.updated_at)
            .execute(&pool)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Failed to create attendance record"})),
                )
            })?;

            attendance
        }
    };

    // Get break records for this attendance
    let break_records = get_break_records(&pool, &attendance.id).await?;

    let response = AttendanceResponse {
        id: attendance.id,
        user_id: attendance.user_id,
        date: attendance.date,
        clock_in_time: attendance.clock_in_time,
        clock_out_time: attendance.clock_out_time,
        status: attendance.status,
        total_work_hours: attendance.total_work_hours,
        break_records,
    };

    Ok(Json(response))
}

pub async fn clock_out(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<ClockOutRequest>,
) -> Result<Json<AttendanceResponse>, (StatusCode, Json<Value>)> {
    let user_id = user.id.as_str();

    let tz = &config.time_zone;
    let now_local = time::now_in_timezone(tz);
    let now_utc = now_local.with_timezone(&Utc);
    let date = payload.date.unwrap_or_else(|| now_local.date_naive());
    let clock_out_time = now_local.naive_local();

    // Find attendance record for this date
    let mut attendance = sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE user_id = ? AND date = ?"
    )
    .bind(user_id)
    .bind(date)
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
            Json(json!({"error": "No attendance record found for today"})),
        )
    })?;

    if attendance.clock_out_time.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Already clocked out today"})),
        ));
    }

    if attendance.clock_in_time.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Must clock in before clocking out"})),
        ));
    }

    attendance.clock_out_time = Some(clock_out_time);
    attendance.calculate_work_hours();
    attendance.updated_at = now_utc;

    sqlx::query(
        "UPDATE attendance SET clock_out_time = ?, total_work_hours = ?, updated_at = ? WHERE id = ?"
    )
    .bind(&attendance.clock_out_time)
    .bind(&attendance.total_work_hours)
    .bind(&attendance.updated_at)
    .bind(&attendance.id)
    .execute(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update attendance"})),
        )
    })?;

    // Get break records for this attendance
    let break_records = get_break_records(&pool, &attendance.id).await?;

    let response = AttendanceResponse {
        id: attendance.id,
        user_id: attendance.user_id,
        date: attendance.date,
        clock_in_time: attendance.clock_in_time,
        clock_out_time: attendance.clock_out_time,
        status: attendance.status,
        total_work_hours: attendance.total_work_hours,
        break_records,
    };

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
    let attendance = sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE id = ?"
    )
    .bind(&payload.attendance_id)
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
            Json(json!({"error": "Attendance record not found"})),
        )
    })?;

    if attendance.user_id != user.id {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }
    if !attendance.is_clocked_in() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Must be clocked in to start break"})),
        ));
    }

    // Check if there's already an active break
    let active_break = sqlx::query_as::<_, BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE attendance_id = ? AND break_end_time IS NULL"
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
        "INSERT INTO break_records (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
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
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE id = ?"
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

    if break_record.break_end_time.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Break already ended"})),
        ));
    }
    // Check ownership
    let att = sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE id = ?"
    )
    .bind(&break_record.attendance_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error":"Attendance record not found"}))))?;
    if att.user_id != user.id {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error":"Forbidden"}))));
    }

    break_record.end_break(break_end_time, now_utc);

    sqlx::query(
        "UPDATE break_records SET break_end_time = ?, duration_minutes = ?, updated_at = ? WHERE id = ?"
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
            "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE user_id = ? AND date BETWEEN ? AND ? ORDER BY date DESC"
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
            "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE user_id = ? AND date BETWEEN ? AND ? ORDER BY date DESC"
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
        let response = AttendanceResponse {
            id: attendance.id,
            user_id: attendance.user_id,
            date: attendance.date,
            clock_in_time: attendance.clock_in_time,
            clock_out_time: attendance.clock_out_time,
            status: attendance.status,
            total_work_hours: attendance.total_work_hours,
            break_records,
        };
        responses.push(response);
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
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at FROM attendance WHERE user_id = ? AND date = ?"
    )
    .bind(&user_id)
    .bind(&date)
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Database error"}))))?;

    if let Some(att) = attendance {
        // Check active break
        let active_break = sqlx::query(
            "SELECT id FROM break_records WHERE attendance_id = ? AND break_end_time IS NULL ORDER BY break_start_time DESC LIMIT 1"
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
        } else if att.clock_out_time.is_some() {
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
        "SELECT COALESCE(SUM(total_work_hours), 0) as total_hours, COUNT(*) as total_days FROM attendance WHERE user_id = ? AND date BETWEEN ? AND ? AND total_work_hours IS NOT NULL"
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

    let mut sql = String::from(
        "SELECT u.username, u.full_name, a.date, a.clock_in_time, a.clock_out_time, \
                a.total_work_hours, a.status \
         FROM attendance a \
         JOIN users u ON a.user_id = u.id \
         WHERE a.user_id = ?",
    );
    if from.is_some() {
        sql.push_str(" AND a.date >= ?");
    }
    if to.is_some() {
        sql.push_str(" AND a.date <= ?");
    }
    sql.push_str(" ORDER BY a.date DESC");

    let mut query = sqlx::query(&sql);
    query = query.bind(&user.id);
    if let Some(f) = from {
        query = query.bind(f);
    }
    if let Some(t) = to {
        query = query.bind(t);
    }

    let rows = query.fetch_all(&pool).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })?;

    let mut csv_data =
        String::from("Username,Full Name,Date,Clock In,Clock Out,Total Hours,Status\n");
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
        let total_hours = row.try_get::<f64, _>("total_work_hours").unwrap_or(0.0);
        let status = row.try_get::<String, _>("status").unwrap_or_default();

        csv_data.push_str(&format!(
            "{},{},{},{},{},{:.2},{}\n",
            username, full_name, date, clock_in, clock_out, total_hours, status
        ));
    }

    Ok(Json(json!({
        "csv_data": csv_data,
        "filename": format!(
            "my_attendance_export_{}.csv",
            time::now_in_timezone(&config.time_zone).format("%Y%m%d_%H%M%S")
        )
    })))
}

async fn get_break_records(
    pool: &PgPool,
    attendance_id: &str,
) -> Result<Vec<BreakRecordResponse>, (StatusCode, Json<Value>)> {
    let break_records = sqlx::query_as::<_, BreakRecord>(
        "SELECT id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at FROM break_records WHERE attendance_id = ? ORDER BY break_start_time"
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

fn status_to_str(s: &crate::models::attendance::AttendanceStatus) -> &'static str {
    use crate::models::attendance::AttendanceStatus::*;
    match s {
        Present => "present",
        Absent => "absent",
        Late => "late",
        HalfDay => "half_day",
    }
}
