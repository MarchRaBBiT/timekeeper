use crate::{
    error::AppError,
    handlers::attendance::recalculate_total_hours,
    handlers::attendance_utils::{get_break_records, get_break_records_map},
    models::{
        attendance::{Attendance, AttendanceResponse},
        break_record::{BreakRecord, BreakRecordResponse},
        user::User,
        PaginatedResponse,
    },
    repositories::{
        attendance::{AttendanceRepository, AttendanceRepositoryTrait},
        break_record::BreakRecordRepository,
        repository::Repository,
        transaction,
    },
    types::{AttendanceId, BreakRecordId, UserId},
    utils::time,
};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use serde::Deserialize;
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct AdminAttendanceUpsertInput {
    pub user_id: String,
    pub date: String,
    pub clock_in_time: String,
    pub clock_out_time: Option<String>,
    pub breaks: Option<Vec<AdminBreakItemInput>>,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct AdminBreakItemInput {
    pub break_start_time: String,
    pub break_end_time: Option<String>,
}

pub async fn get_all_attendance(
    read_pool: &sqlx::PgPool,
    user: &User,
    limit: i64,
    offset: i64,
) -> Result<PaginatedResponse<AttendanceResponse>, AppError> {
    ensure_system_admin(user)?;

    let repo = AttendanceRepository::new();
    let total = repo.count_all(read_pool).await?;
    let attendances = repo.list_paginated(read_pool, limit, offset).await?;

    let attendance_ids: Vec<AttendanceId> = attendances.iter().map(|a| a.id).collect();
    let mut break_map = get_break_records_map(read_pool, &attendance_ids).await?;

    let data = attendances
        .into_iter()
        .map(|attendance| {
            let break_records = break_map.remove(&attendance.id).unwrap_or_default();
            build_attendance_response(attendance, break_records)
        })
        .collect();

    Ok(PaginatedResponse::new(data, total, limit, offset))
}

pub async fn upsert_attendance(
    write_pool: &sqlx::PgPool,
    time_zone: &chrono_tz::Tz,
    user: &User,
    body: AdminAttendanceUpsertInput,
) -> Result<AttendanceResponse, AppError> {
    ensure_system_admin(user)?;

    let parsed = parse_admin_attendance_upsert(body)?;
    let now_utc = time::now_utc(time_zone);
    let mut tx = transaction::begin_transaction(write_pool).await?;

    let attendance_repo = AttendanceRepository::new();
    let break_repo = BreakRecordRepository::new();

    attendance_repo
        .delete_by_user_and_date(&mut tx, parsed.user_id, parsed.date)
        .await?;

    let mut attendance = Attendance::new(parsed.user_id, parsed.date, now_utc);
    attendance.clock_in_time = Some(parsed.clock_in_time);
    attendance.clock_out_time = parsed.clock_out_time;

    let mut total_break_minutes: i64 = 0;
    let mut pending_breaks: Vec<BreakRecord> = Vec::new();
    for item in parsed.breaks {
        let mut break_record = BreakRecord::new(attendance.id, item.break_start_time, now_utc);
        if let Some(end_time) = item.break_end_time {
            break_record.break_end_time = Some(end_time);
            let duration = end_time.signed_duration_since(item.break_start_time);
            let minutes = duration.num_minutes().max(0);
            break_record.duration_minutes = Some(minutes as i32);
            break_record.updated_at = now_utc;
            total_break_minutes += minutes;
        }
        pending_breaks.push(break_record);
    }

    attendance.calculate_work_hours(total_break_minutes);
    attendance_repo
        .create_in_transaction(&mut tx, &attendance)
        .await?;

    for break_record in pending_breaks {
        break_repo
            .create_in_transaction(&mut tx, &break_record)
            .await?;
    }

    transaction::commit_transaction(tx).await?;

    let breaks = get_break_records(write_pool, attendance.id).await?;
    Ok(build_attendance_response(attendance, breaks))
}

pub async fn force_end_break(
    write_pool: &sqlx::PgPool,
    time_zone: &chrono_tz::Tz,
    user: &User,
    break_id: &str,
) -> Result<BreakRecordResponse, AppError> {
    ensure_system_admin(user)?;

    let break_record_id = BreakRecordId::from_str(break_id)
        .map_err(|_| AppError::BadRequest("Invalid break record ID format".into()))?;
    let now_local = time::now_in_timezone(time_zone);
    let now_utc = now_local.with_timezone(&Utc);
    let now = now_local.naive_local();

    let break_repo = BreakRecordRepository::new();
    let mut rec = break_repo.find_by_id(write_pool, break_record_id).await?;

    if rec.break_end_time.is_some() {
        return Err(AppError::BadRequest("Break already ended".into()));
    }

    rec.break_end_time = Some(now);
    let duration = now.signed_duration_since(rec.break_start_time);
    rec.duration_minutes = Some(duration.num_minutes() as i32);
    rec.updated_at = now_utc;
    break_repo.update(write_pool, &rec).await?;

    let attendance_repo = AttendanceRepository::new();
    if let Some(attendance) = attendance_repo
        .find_optional_by_id(write_pool, rec.attendance_id)
        .await?
    {
        if attendance.clock_out_time.is_some() {
            recalculate_total_hours(write_pool, attendance, now_utc).await?;
        }
    }

    Ok(BreakRecordResponse::from(rec))
}

fn ensure_system_admin(user: &User) -> Result<(), AppError> {
    if user.is_system_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden("Forbidden".into()))
    }
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

#[derive(Debug)]
struct ParsedAdminAttendanceUpsert {
    user_id: UserId,
    date: NaiveDate,
    clock_in_time: NaiveDateTime,
    clock_out_time: Option<NaiveDateTime>,
    breaks: Vec<ParsedAdminBreakItem>,
}

#[derive(Debug)]
struct ParsedAdminBreakItem {
    break_start_time: NaiveDateTime,
    break_end_time: Option<NaiveDateTime>,
}

fn parse_admin_attendance_upsert(
    body: AdminAttendanceUpsertInput,
) -> Result<ParsedAdminAttendanceUpsert, AppError> {
    let date = NaiveDate::parse_from_str(&body.date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid date".into()))?;
    let clock_in_time = parse_admin_datetime(&body.clock_in_time, "clock_in_time")?;
    let clock_out_time = body
        .clock_out_time
        .as_deref()
        .map(|raw| parse_admin_datetime(raw, "clock_out_time"))
        .transpose()?;
    let user_id = UserId::from_str(&body.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id format".into()))?;

    let breaks = body
        .breaks
        .unwrap_or_default()
        .into_iter()
        .map(parse_admin_break_item)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ParsedAdminAttendanceUpsert {
        user_id,
        date,
        clock_in_time,
        clock_out_time,
        breaks,
    })
}

fn parse_admin_break_item(item: AdminBreakItemInput) -> Result<ParsedAdminBreakItem, AppError> {
    let break_start_time = parse_admin_datetime(&item.break_start_time, "break_start_time")?;
    let break_end_time = item
        .break_end_time
        .as_deref()
        .map(|raw| parse_admin_datetime(raw, "break_end_time"))
        .transpose()?;

    Ok(ParsedAdminBreakItem {
        break_start_time,
        break_end_time,
    })
}

fn parse_admin_datetime(raw: &str, field_name: &str) -> Result<NaiveDateTime, AppError> {
    NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S"))
        .map_err(|_| AppError::BadRequest(format!("Invalid {field_name}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::UserRole;

    fn sample_user(is_system_admin: bool) -> User {
        let now = Utc::now();
        User {
            id: UserId::new(),
            username: "admin".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Admin".to_string(),
            email: "admin@example.com".to_string(),
            role: UserRole::Admin,
            is_system_admin,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            failed_login_attempts: 0,
            locked_until: None,
            lock_reason: None,
            lockout_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn parse_admin_attendance_upsert_accepts_iso_datetimes() {
        let parsed = parse_admin_attendance_upsert(AdminAttendanceUpsertInput {
            user_id: UserId::new().to_string(),
            date: "2026-03-09".to_string(),
            clock_in_time: "2026-03-09T09:00:00".to_string(),
            clock_out_time: Some("2026-03-09T18:00:00".to_string()),
            breaks: Some(vec![AdminBreakItemInput {
                break_start_time: "2026-03-09T12:00:00".to_string(),
                break_end_time: Some("2026-03-09T13:00:00".to_string()),
            }]),
        })
        .expect("input should parse");

        assert_eq!(
            parsed.date,
            NaiveDate::from_ymd_opt(2026, 3, 9).expect("date")
        );
        assert_eq!(parsed.breaks.len(), 1);
    }

    #[test]
    fn parse_admin_attendance_upsert_rejects_invalid_user_id() {
        let err = parse_admin_attendance_upsert(AdminAttendanceUpsertInput {
            user_id: "bad-id".to_string(),
            date: "2026-03-09".to_string(),
            clock_in_time: "2026-03-09T09:00:00".to_string(),
            clock_out_time: None,
            breaks: None,
        })
        .expect_err("invalid user id should fail");

        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn parse_admin_datetime_rejects_bad_value() {
        let err = parse_admin_datetime("bad-time", "clock_in_time").expect_err("bad time");
        match err {
            AppError::BadRequest(message) => assert_eq!(message, "Invalid clock_in_time"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn ensure_system_admin_rejects_non_system_admin() {
        let err = ensure_system_admin(&sample_user(false)).expect_err("forbidden");
        match err {
            AppError::Forbidden(message) => assert_eq!(message, "Forbidden"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn build_attendance_response_preserves_break_records() {
        let now = Utc::now();
        let date = NaiveDate::from_ymd_opt(2026, 3, 9).expect("date");
        let attendance = Attendance::new(UserId::new(), date, now);
        let break_record = BreakRecordResponse {
            id: BreakRecordId::new(),
            attendance_id: attendance.id,
            break_start_time: date.and_hms_opt(12, 0, 0).expect("break start"),
            break_end_time: None,
            duration_minutes: None,
        };

        let response = build_attendance_response(attendance, vec![break_record]);
        assert_eq!(response.break_records.len(), 1);
    }
}
