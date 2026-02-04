use crate::models::break_record::BreakRecordResponse;
use crate::repositories::attendance::{AttendanceRepository, AttendanceRepositoryTrait};
use crate::repositories::break_record::BreakRecordRepository;
use crate::types::{AttendanceId, UserId};
use crate::{error::AppError, models::attendance::Attendance};
use chrono::NaiveDate;
use sqlx::PgPool;
use std::collections::HashMap;

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
    let repo = AttendanceRepository::new();
    repo.find_by_user_and_date(pool, user_id, date).await
}

pub async fn fetch_attendance_by_id(
    pool: &PgPool,
    attendance_id: AttendanceId,
) -> Result<Attendance, AppError> {
    let repo = AttendanceRepository::new();
    repo.find_by_id(pool, attendance_id).await
}

pub async fn insert_attendance_record(
    pool: &PgPool,
    attendance: &Attendance,
) -> Result<(), AppError> {
    let repo = AttendanceRepository::new();
    repo.create(pool, attendance).await?;
    Ok(())
}

pub async fn update_clock_in(pool: &PgPool, attendance: &Attendance) -> Result<(), AppError> {
    let repo = AttendanceRepository::new();
    repo.update(pool, attendance).await?;
    Ok(())
}

pub async fn update_clock_out(pool: &PgPool, attendance: &Attendance) -> Result<(), AppError> {
    let repo = AttendanceRepository::new();
    repo.update(pool, attendance).await?;
    Ok(())
}

pub async fn get_break_records(
    pool: &PgPool,
    attendance_id: AttendanceId,
) -> Result<Vec<BreakRecordResponse>, AppError> {
    let repo = BreakRecordRepository::new();
    let break_records = repo.find_by_attendance(pool, attendance_id).await?;

    Ok(break_records
        .into_iter()
        .map(BreakRecordResponse::from)
        .collect())
}

pub async fn get_break_records_map(
    pool: &PgPool,
    attendance_ids: &[AttendanceId],
) -> Result<HashMap<AttendanceId, Vec<BreakRecordResponse>>, AppError> {
    if attendance_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let repo = BreakRecordRepository::new();
    let break_records = repo.find_by_attendance_ids(pool, attendance_ids).await?;

    let mut map = HashMap::new();
    for rec in break_records {
        let att_id = rec.attendance_id;
        map.entry(att_id)
            .or_insert_with(Vec::new)
            .push(BreakRecordResponse::from(rec));
    }
    Ok(map)
}
