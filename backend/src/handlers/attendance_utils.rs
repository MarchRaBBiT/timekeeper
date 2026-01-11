use crate::types::{AttendanceId, UserId};
use crate::{
    error::AppError,
    models::attendance::{Attendance, AttendanceStatus},
};
use chrono::NaiveDate;
use sqlx::PgPool;

pub fn ensure_authorized_access(attendance: &Attendance, user_id: UserId) -> Result<(), AppError> {
    if attendance.user_id == user_id {
        Ok(())
    } else {
        Err(AppError::Forbidden("Forbidden".into()))
    }
}

pub fn ensure_not_clocked_in(attendance: &Attendance) -> Result<(), AppError> {
    if attendance.clock_in_time.is_some() {
        Err(AppError::BadRequest("Already clocked in today".into()))
    } else {
        Ok(())
    }
}

pub fn ensure_not_clocked_out(attendance: &Attendance) -> Result<(), AppError> {
    if attendance.is_clocked_out() {
        Err(AppError::BadRequest("Already clocked out today".into()))
    } else {
        Ok(())
    }
}

pub fn ensure_clock_in_exists(attendance: &Attendance) -> Result<(), AppError> {
    if attendance.clock_in_time.is_none() {
        Err(AppError::BadRequest(
            "Must clock in before clocking out".into(),
        ))
    } else {
        Ok(())
    }
}

pub fn ensure_clocked_in(attendance: &Attendance) -> Result<(), AppError> {
    if attendance.is_clocked_in() {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "Must be clocked in to start break".into(),
        ))
    }
}

pub async fn fetch_attendance_by_user_date(
    pool: &PgPool,
    user_id: UserId,
    date: NaiveDate,
) -> Result<Option<Attendance>, AppError> {
    Ok(sqlx::query_as::<sqlx::Postgres, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at \
         FROM attendance WHERE user_id = $1 AND date = $2",
    )
    .bind(user_id.to_string())
    .bind(date)
    .fetch_optional(pool)
    .await?)
}

pub async fn fetch_attendance_by_id(
    pool: &PgPool,
    attendance_id: AttendanceId,
) -> Result<Attendance, AppError> {
    sqlx::query_as::<_, Attendance>(
        "SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at \
         FROM attendance WHERE id = $1",
    )
    .bind(attendance_id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Attendance record not found".into()))
}

pub async fn insert_attendance_record(
    pool: &PgPool,
    attendance: &Attendance,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO attendance (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(attendance.id.to_string())
    .bind(attendance.user_id.to_string())
    .bind(attendance.date)
    .bind(attendance.clock_in_time)
    .bind(attendance.clock_out_time)
    .bind(status_to_str(&attendance.status))
    .bind(attendance.total_work_hours)
    .bind(attendance.created_at)
    .bind(attendance.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_clock_in(pool: &PgPool, attendance: &Attendance) -> Result<(), AppError> {
    sqlx::query("UPDATE attendance SET clock_in_time = $1, updated_at = $2 WHERE id = $3")
        .bind(attendance.clock_in_time)
        .bind(attendance.updated_at)
        .bind(attendance.id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_clock_out(pool: &PgPool, attendance: &Attendance) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE attendance SET clock_out_time = $1, total_work_hours = $2, updated_at = $3 WHERE id = $4",
    )
    .bind(attendance.clock_out_time)
    .bind(attendance.total_work_hours)
    .bind(attendance.updated_at)
    .bind(attendance.id.to_string())
    .execute(pool)
    .await?;

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
