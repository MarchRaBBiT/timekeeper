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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::attendance::{Attendance, AttendanceStatus};
    use chrono::{NaiveDate, NaiveDateTime, Utc};
    use sqlx::postgres::PgPoolOptions;

    fn sample_attendance(user_id: UserId) -> Attendance {
        Attendance {
            id: AttendanceId::new(),
            user_id,
            date: NaiveDate::from_ymd_opt(2026, 2, 4).expect("date"),
            clock_in_time: Some(
                NaiveDateTime::parse_from_str("2026-02-04T09:00:00", "%Y-%m-%dT%H:%M:%S")
                    .expect("clock in"),
            ),
            clock_out_time: None,
            status: AttendanceStatus::Present,
            total_work_hours: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn ensure_authorized_access_accepts_same_user() {
        let user_id = UserId::new();
        let attendance = sample_attendance(user_id);
        assert!(ensure_authorized_access(&attendance, user_id).is_ok());
    }

    #[test]
    fn ensure_authorized_access_rejects_other_user() {
        let attendance = sample_attendance(UserId::new());
        let err = ensure_authorized_access(&attendance, UserId::new()).expect_err("forbidden");
        match err {
            AppError::Forbidden(message) => assert_eq!(message, "Forbidden"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn ensure_not_clocked_in_rejects_existing_clock_in() {
        let attendance = sample_attendance(UserId::new());
        let err = ensure_not_clocked_in(&attendance).expect_err("already clocked in");
        match err {
            AppError::BadRequest(message) => assert!(message.contains("Already clocked in")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn ensure_not_clocked_out_rejects_when_clock_out_exists() {
        let mut attendance = sample_attendance(UserId::new());
        attendance.clock_out_time = attendance.clock_in_time;
        let err = ensure_not_clocked_out(&attendance).expect_err("already clocked out");
        match err {
            AppError::BadRequest(message) => assert!(message.contains("Already clocked out")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn ensure_clock_in_exists_rejects_missing_clock_in() {
        let mut attendance = sample_attendance(UserId::new());
        attendance.clock_in_time = None;
        let err = ensure_clock_in_exists(&attendance).expect_err("missing clock in");
        match err {
            AppError::BadRequest(message) => assert!(message.contains("Must clock in")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn ensure_clocked_in_rejects_when_not_clocked_in() {
        let mut attendance = sample_attendance(UserId::new());
        attendance.clock_in_time = None;
        let err = ensure_clocked_in(&attendance).expect_err("not clocked in");
        match err {
            AppError::BadRequest(message) => assert!(message.contains("Must be clocked in")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_break_records_map_returns_empty_for_empty_ids() {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://127.0.0.1:15432/timekeeper")
            .expect("lazy pool");
        let map = get_break_records_map(&pool, &[]).await.expect("empty map");
        assert!(map.is_empty());
    }
}
