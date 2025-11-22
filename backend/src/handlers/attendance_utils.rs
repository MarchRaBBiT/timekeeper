use axum::{http::StatusCode, Json};
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::models::attendance::{Attendance, AttendanceStatus};

pub fn error_response(code: StatusCode, message: &'static str) -> (StatusCode, Json<Value>) {
    (code, Json(json!({ "error": message })))
}

pub fn db_error_response() -> (StatusCode, Json<Value>) {
    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
}

pub fn forbidden_response() -> (StatusCode, Json<Value>) {
    error_response(StatusCode::FORBIDDEN, "Forbidden")
}

pub fn ensure_owned(
    attendance: &Attendance,
    user_id: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    if attendance.user_id == user_id {
        Ok(())
    } else {
        Err(forbidden_response())
    }
}

pub fn ensure_not_clocked_in(attendance: &Attendance) -> Result<(), (StatusCode, Json<Value>)> {
    if attendance.clock_in_time.is_some() {
        Err(error_response(
            StatusCode::BAD_REQUEST,
            "Already clocked in today",
        ))
    } else {
        Ok(())
    }
}

pub fn ensure_not_clocked_out(attendance: &Attendance) -> Result<(), (StatusCode, Json<Value>)> {
    if attendance.is_clocked_out() {
        Err(error_response(
            StatusCode::BAD_REQUEST,
            "Already clocked out today",
        ))
    } else {
        Ok(())
    }
}

pub fn ensure_clock_in_exists(attendance: &Attendance) -> Result<(), (StatusCode, Json<Value>)> {
    if attendance.clock_in_time.is_none() {
        Err(error_response(
            StatusCode::BAD_REQUEST,
            "Must clock in before clocking out",
        ))
    } else {
        Ok(())
    }
}

pub fn ensure_clocked_in(attendance: &Attendance) -> Result<(), (StatusCode, Json<Value>)> {
    if attendance.is_clocked_in() {
        Ok(())
    } else {
        Err(error_response(
            StatusCode::BAD_REQUEST,
            "Must be clocked in to start break",
        ))
    }
}

pub async fn fetch_attendance_by_user_date(
    pool: &PgPool,
    user_id: &str,
    date: NaiveDate,
) -> Result<Option<Attendance>, (StatusCode, Json<Value>)> {
    sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at \
         FROM attendance WHERE user_id = $1 AND date = $2",
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(pool)
    .await
    .map_err(|_| db_error_response())
}

pub async fn fetch_attendance_by_id(
    pool: &PgPool,
    attendance_id: &str,
) -> Result<Attendance, (StatusCode, Json<Value>)> {
    sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at \
         FROM attendance WHERE id = $1",
    )
    .bind(attendance_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| db_error_response())?
    .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "Attendance record not found"))
}

pub async fn insert_attendance_record(
    pool: &PgPool,
    attendance: &Attendance,
) -> Result<(), (StatusCode, Json<Value>)> {
    sqlx::query(
        "INSERT INTO attendance (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&attendance.id)
    .bind(&attendance.user_id)
    .bind(attendance.date)
    .bind(attendance.clock_in_time)
    .bind(attendance.clock_out_time)
    .bind(status_to_str(&attendance.status))
    .bind(attendance.total_work_hours)
    .bind(attendance.created_at)
    .bind(attendance.updated_at)
    .execute(pool)
    .await
    .map_err(|_| db_error_response())?;

    Ok(())
}

pub async fn update_clock_in(
    pool: &PgPool,
    attendance: &Attendance,
) -> Result<(), (StatusCode, Json<Value>)> {
    sqlx::query("UPDATE attendance SET clock_in_time = $1, updated_at = $2 WHERE id = $3")
        .bind(attendance.clock_in_time)
        .bind(attendance.updated_at)
        .bind(&attendance.id)
        .execute(pool)
        .await
        .map_err(|_| db_error_response())?;

    Ok(())
}

pub async fn update_clock_out(
    pool: &PgPool,
    attendance: &Attendance,
) -> Result<(), (StatusCode, Json<Value>)> {
    sqlx::query(
        "UPDATE attendance SET clock_out_time = $1, total_work_hours = $2, updated_at = $3 WHERE id = $4",
    )
    .bind(attendance.clock_out_time)
    .bind(attendance.total_work_hours)
    .bind(attendance.updated_at)
    .bind(&attendance.id)
    .execute(pool)
    .await
    .map_err(|_| db_error_response())?;

    Ok(())
}

pub fn status_to_str(status: &AttendanceStatus) -> &'static str {
    use AttendanceStatus::*;
    match status {
        Present => "present",
        Absent => "absent",
        Late => "late",
        HalfDay => "half_day",
    }
}
