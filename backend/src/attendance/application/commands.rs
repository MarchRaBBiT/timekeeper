use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use sqlx::PgPool;

use crate::repositories::attendance::AttendanceRepositoryTrait;
use crate::{
    attendance::application::helpers::{
        ensure_authorized_access, ensure_clock_in_exists, ensure_clocked_in, ensure_not_clocked_in,
        ensure_not_clocked_out, fetch_attendance_by_id, fetch_attendance_by_user_date,
        get_break_records, insert_attendance_record, update_clock_in, update_clock_out,
    },
    error::AppError,
    models::{
        attendance::AttendanceResponse,
        break_record::{BreakRecord, BreakRecordResponse},
    },
    repositories::{break_record::BreakRecordRepository, repository::Repository},
    services::holiday::HolidayServiceTrait,
    types::{AttendanceId, BreakRecordId, UserId},
};

pub async fn clock_in(
    write_pool: &PgPool,
    holiday_service: &dyn HolidayServiceTrait,
    user_id: UserId,
    date: NaiveDate,
    clock_in_time: NaiveDateTime,
    now_utc: DateTime<Utc>,
) -> Result<AttendanceResponse, AppError> {
    reject_if_holiday(holiday_service, date, user_id).await?;

    let attendance = match fetch_attendance_by_user_date(write_pool, user_id, date).await? {
        Some(mut attendance) => {
            ensure_not_clocked_in(&attendance)?;
            attendance.clock_in_time = Some(clock_in_time);
            attendance.updated_at = now_utc;
            update_clock_in(write_pool, &attendance).await?;
            attendance
        }
        None => {
            let mut attendance = crate::models::attendance::Attendance::new(user_id, date, now_utc);
            attendance.clock_in_time = Some(clock_in_time);
            insert_attendance_record(write_pool, &attendance).await?;
            attendance
        }
    };

    let break_records = get_break_records(write_pool, attendance.id).await?;
    Ok(build_attendance_response(attendance, break_records))
}

pub async fn clock_out(
    write_pool: &PgPool,
    holiday_service: &dyn HolidayServiceTrait,
    user_id: UserId,
    date: NaiveDate,
    clock_out_time: NaiveDateTime,
    now_utc: DateTime<Utc>,
) -> Result<AttendanceResponse, AppError> {
    reject_if_holiday(holiday_service, date, user_id).await?;

    let mut attendance = fetch_attendance_by_user_date(write_pool, user_id, date)
        .await?
        .ok_or_else(|| AppError::NotFound("No attendance record found for today".into()))?;

    ensure_not_clocked_out(&attendance)?;
    ensure_clock_in_exists(&attendance)?;

    let break_repo = BreakRecordRepository::new();
    let active_break = break_repo
        .find_active_break(write_pool, attendance.id)
        .await?;
    if active_break.is_some() {
        return Err(AppError::BadRequest(
            "Break in progress. End break before clocking out".into(),
        ));
    }

    attendance.clock_out_time = Some(clock_out_time);
    let break_minutes = total_break_minutes(write_pool, attendance.id).await?;
    attendance.calculate_work_hours(break_minutes);
    attendance.updated_at = now_utc;
    update_clock_out(write_pool, &attendance).await?;

    let break_records = get_break_records(write_pool, attendance.id).await?;
    Ok(build_attendance_response(attendance, break_records))
}

pub async fn break_start(
    write_pool: &PgPool,
    user_id: UserId,
    attendance_id: AttendanceId,
    break_start_time: NaiveDateTime,
    now_utc: DateTime<Utc>,
) -> Result<BreakRecordResponse, AppError> {
    let attendance = fetch_attendance_by_id(write_pool, attendance_id).await?;
    ensure_authorized_access(&attendance, user_id)?;
    ensure_clocked_in(&attendance)?;

    let break_repo = BreakRecordRepository::new();
    let active_break = break_repo
        .find_active_break(write_pool, attendance_id)
        .await?;
    if active_break.is_some() {
        return Err(AppError::BadRequest("Break already in progress".into()));
    }

    let break_record = BreakRecord::new(attendance_id, break_start_time, now_utc);
    break_repo.create(write_pool, &break_record).await?;
    Ok(BreakRecordResponse::from(break_record))
}

pub async fn break_end(
    write_pool: &PgPool,
    user_id: UserId,
    break_record_id: BreakRecordId,
    break_end_time: NaiveDateTime,
    now_utc: DateTime<Utc>,
) -> Result<BreakRecordResponse, AppError> {
    let break_repo = BreakRecordRepository::new();
    let mut break_record = break_repo.find_by_id(write_pool, break_record_id).await?;

    if !break_record.is_active() {
        return Err(AppError::BadRequest("Break already ended".into()));
    }

    let attendance = fetch_attendance_by_id(write_pool, break_record.attendance_id).await?;
    ensure_authorized_access(&attendance, user_id)?;

    break_record.end_break(break_end_time, now_utc);
    break_repo.update(write_pool, &break_record).await?;

    if attendance.clock_out_time.is_some() {
        recalculate_total_hours(write_pool, attendance, now_utc).await?;
    }

    Ok(BreakRecordResponse::from(break_record))
}

pub async fn total_break_minutes(
    pool: &PgPool,
    attendance_id: AttendanceId,
) -> Result<i64, AppError> {
    let repo = BreakRecordRepository::new();
    repo.get_total_duration(pool, attendance_id).await
}

pub async fn recalculate_total_hours(
    pool: &PgPool,
    mut attendance: crate::models::attendance::Attendance,
    updated_at: DateTime<Utc>,
) -> Result<(), AppError> {
    if attendance.clock_in_time.is_none() || attendance.clock_out_time.is_none() {
        return Ok(());
    }

    let break_repo = BreakRecordRepository::new();
    let break_minutes = break_repo.get_total_duration(pool, attendance.id).await?;

    attendance.calculate_work_hours(break_minutes);
    attendance.updated_at = updated_at;

    let att_repo = crate::repositories::attendance::AttendanceRepository::new();
    att_repo.update(pool, &attendance).await?;
    Ok(())
}

pub async fn reject_if_holiday(
    holiday_service: &dyn HolidayServiceTrait,
    date: NaiveDate,
    user_id: UserId,
) -> Result<(), AppError> {
    let decision = holiday_service
        .is_holiday(date, Some(&user_id.to_string()))
        .await?;

    if decision.is_holiday {
        let reason = decision.reason.label();
        return Err(AppError::Forbidden(format!(
            "{} is a {}. Submit an overtime request before clocking in/out.",
            date, reason
        )));
    }

    Ok(())
}

pub fn build_attendance_response(
    attendance: crate::models::attendance::Attendance,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::attendance::{Attendance, AttendanceStatus},
        services::holiday::{HolidayCalendarEntry, HolidayDecision, HolidayReason},
    };
    use chrono::NaiveDate;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;

    struct FixedHolidayService {
        decision: HolidayDecision,
    }

    #[async_trait::async_trait]
    impl HolidayServiceTrait for FixedHolidayService {
        async fn is_holiday(
            &self,
            _date: NaiveDate,
            _user_id: Option<&str>,
        ) -> sqlx::Result<HolidayDecision> {
            Ok(self.decision.clone())
        }

        async fn list_month(
            &self,
            _year: i32,
            _month: u32,
            _user_id: Option<&str>,
        ) -> sqlx::Result<Vec<HolidayCalendarEntry>> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn reject_if_holiday_allows_working_day() {
        let service = Arc::new(FixedHolidayService {
            decision: HolidayDecision {
                is_holiday: false,
                reason: HolidayReason::None,
            },
        });
        let date = NaiveDate::from_ymd_opt(2026, 2, 4).unwrap();
        assert!(reject_if_holiday(service.as_ref(), date, UserId::new())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn reject_if_holiday_rejects_holiday_with_reason() {
        let service = Arc::new(FixedHolidayService {
            decision: HolidayDecision {
                is_holiday: true,
                reason: HolidayReason::PublicHoliday,
            },
        });
        let date = NaiveDate::from_ymd_opt(2026, 2, 11).unwrap();
        let err = reject_if_holiday(service.as_ref(), date, UserId::new())
            .await
            .expect_err("holiday");
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[test]
    fn build_attendance_response_keeps_core_fields() {
        let now = Utc::now();
        let attendance = Attendance {
            id: AttendanceId::new(),
            user_id: UserId::new(),
            date: NaiveDate::from_ymd_opt(2026, 2, 4).unwrap(),
            clock_in_time: Some(
                chrono::NaiveDateTime::parse_from_str("2026-02-04T09:00:00", "%Y-%m-%dT%H:%M:%S")
                    .unwrap(),
            ),
            clock_out_time: None,
            status: AttendanceStatus::Present,
            total_work_hours: None,
            created_at: now,
            updated_at: now,
        };

        let response = build_attendance_response(attendance.clone(), Vec::new());
        assert_eq!(response.id, attendance.id);
        assert_eq!(response.user_id, attendance.user_id);
        assert_eq!(response.date, attendance.date);
        assert_eq!(response.clock_in_time, attendance.clock_in_time);
        assert_eq!(response.clock_out_time, attendance.clock_out_time);
        assert!(response.break_records.is_empty());
    }

    #[tokio::test]
    async fn recalculate_total_hours_returns_early_when_times_missing() {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://127.0.0.1:15432/timekeeper")
            .expect("lazy pool");
        let now = Utc::now();
        let mut attendance = Attendance::new(
            UserId::new(),
            NaiveDate::from_ymd_opt(2026, 2, 4).unwrap(),
            now,
        );
        attendance.clock_in_time = None;
        attendance.clock_out_time = None;
        assert!(recalculate_total_hours(&pool, attendance, now)
            .await
            .is_ok());
    }
}
