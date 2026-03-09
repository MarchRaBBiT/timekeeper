use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        attendance::Attendance,
        attendance_correction_request::{
            AttendanceCorrectionResponse, AttendanceCorrectionSnapshot, CorrectionBreakItem,
            CreateAttendanceCorrectionRequest, UpdateAttendanceCorrectionRequest,
        },
    },
    repositories::{
        attendance::{AttendanceRepository, AttendanceRepositoryTrait},
        attendance_correction_request::AttendanceCorrectionRequestRepository,
        break_record::BreakRecordRepository,
    },
    types::UserId,
};

pub async fn create_attendance_correction_request(
    read_pool: &sqlx::PgPool,
    write_pool: &sqlx::PgPool,
    user_id: UserId,
    payload: CreateAttendanceCorrectionRequest,
) -> Result<AttendanceCorrectionResponse, AppError> {
    validate_reason(&payload.reason)?;

    let attendance_repo = AttendanceRepository::new();
    let break_repo = BreakRecordRepository::new();

    let attendance = attendance_repo
        .find_by_user_and_date(read_pool, user_id, payload.date)
        .await?
        .ok_or_else(|| {
            AppError::NotFound("No attendance record found for specified date".into())
        })?;

    let original_snapshot = build_snapshot(&attendance, &break_repo, read_pool).await?;
    let proposed_snapshot = build_proposed_snapshot(
        &original_snapshot,
        payload.clock_in_time,
        payload.clock_out_time,
        payload.breaks,
    )?;

    if original_snapshot == proposed_snapshot {
        return Err(AppError::BadRequest(
            "At least one field must be changed".into(),
        ));
    }

    validate_snapshot(&proposed_snapshot)?;

    let repo = AttendanceCorrectionRequestRepository::new();
    let request = repo
        .create(
            write_pool,
            &Uuid::new_v4().to_string(),
            user_id,
            attendance.id,
            payload.date,
            &payload.reason,
            &original_snapshot,
            &proposed_snapshot,
        )
        .await?;

    request.to_response().map_err(AppError::InternalServerError)
}

pub async fn list_user_attendance_correction_requests(
    read_pool: &sqlx::PgPool,
    user_id: UserId,
) -> Result<Vec<AttendanceCorrectionResponse>, AppError> {
    let repo = AttendanceCorrectionRequestRepository::new();
    let list = repo.list_by_user(read_pool, user_id).await?;

    list.into_iter()
        .map(|item| item.to_response().map_err(AppError::InternalServerError))
        .collect()
}

pub async fn get_user_attendance_correction_request(
    read_pool: &sqlx::PgPool,
    user_id: UserId,
    id: &str,
) -> Result<AttendanceCorrectionResponse, AppError> {
    let repo = AttendanceCorrectionRequestRepository::new();
    let request = repo.find_by_id_for_user(read_pool, id, user_id).await?;

    request.to_response().map_err(AppError::InternalServerError)
}

pub async fn update_user_attendance_correction_request(
    read_pool: &sqlx::PgPool,
    write_pool: &sqlx::PgPool,
    user_id: UserId,
    id: &str,
    payload: UpdateAttendanceCorrectionRequest,
) -> Result<AttendanceCorrectionResponse, AppError> {
    validate_reason(&payload.reason)?;

    let repo = AttendanceCorrectionRequestRepository::new();
    let current = repo.find_by_id_for_user(read_pool, id, user_id).await?;

    if current.status.db_value() != "pending" {
        return Err(AppError::Conflict(
            "Only pending requests can be updated".into(),
        ));
    }

    let original_snapshot = current
        .parse_original_snapshot()
        .map_err(|error| AppError::InternalServerError(error.into()))?;
    let proposed_snapshot = build_proposed_snapshot(
        &original_snapshot,
        payload.clock_in_time,
        payload.clock_out_time,
        payload.breaks,
    )?;

    if original_snapshot == proposed_snapshot {
        return Err(AppError::BadRequest(
            "At least one field must be changed".into(),
        ));
    }

    validate_snapshot(&proposed_snapshot)?;

    let updated = repo
        .update_pending_for_user(write_pool, id, user_id, &payload.reason, &proposed_snapshot)
        .await?;

    updated.to_response().map_err(AppError::InternalServerError)
}

pub async fn cancel_user_attendance_correction_request(
    write_pool: &sqlx::PgPool,
    user_id: UserId,
    id: &str,
) -> Result<serde_json::Value, AppError> {
    let repo = AttendanceCorrectionRequestRepository::new();
    repo.cancel_pending_for_user(write_pool, id, user_id)
        .await?;
    Ok(serde_json::json!({ "id": id, "status": "cancelled" }))
}

pub fn build_proposed_snapshot(
    original: &AttendanceCorrectionSnapshot,
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
    breaks: Option<Vec<CorrectionBreakItem>>,
) -> Result<AttendanceCorrectionSnapshot, AppError> {
    Ok(AttendanceCorrectionSnapshot {
        clock_in_time: clock_in_time.or(original.clock_in_time),
        clock_out_time: clock_out_time.or(original.clock_out_time),
        breaks: breaks.unwrap_or_else(|| original.breaks.clone()),
    })
}

pub fn validate_snapshot(snapshot: &AttendanceCorrectionSnapshot) -> Result<(), AppError> {
    let Some(clock_in) = snapshot.clock_in_time else {
        return Err(AppError::BadRequest("clock_in_time is required".into()));
    };

    if let Some(clock_out) = snapshot.clock_out_time {
        if clock_in > clock_out {
            return Err(AppError::BadRequest(
                "clock_out_time must be later than clock_in_time".into(),
            ));
        }
    }

    for break_item in &snapshot.breaks {
        if let Some(end) = break_item.break_end_time {
            if break_item.break_start_time > end {
                return Err(AppError::BadRequest(
                    "break_end_time must be later than break_start_time".into(),
                ));
            }
            if break_item.break_start_time < clock_in {
                return Err(AppError::BadRequest(
                    "break_start_time must be later than clock_in_time".into(),
                ));
            }
            if let Some(clock_out) = snapshot.clock_out_time {
                if end > clock_out {
                    return Err(AppError::BadRequest(
                        "break_end_time must be earlier than clock_out_time".into(),
                    ));
                }
            }
        }
    }

    Ok(())
}

pub fn validate_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().is_empty() {
        return Err(AppError::BadRequest("reason is required".into()));
    }
    if reason.chars().count() > 500 {
        return Err(AppError::BadRequest(
            "reason must be between 1 and 500 characters".into(),
        ));
    }
    Ok(())
}

pub async fn build_snapshot(
    attendance: &Attendance,
    break_repo: &BreakRecordRepository,
    db: &sqlx::PgPool,
) -> Result<AttendanceCorrectionSnapshot, AppError> {
    let breaks = break_repo
        .find_by_attendance(db, attendance.id)
        .await?
        .into_iter()
        .map(|item| CorrectionBreakItem {
            break_start_time: item.break_start_time,
            break_end_time: item.break_end_time,
        })
        .collect();

    Ok(AttendanceCorrectionSnapshot {
        clock_in_time: attendance.clock_in_time,
        clock_out_time: attendance.clock_out_time,
        breaks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_reason_rejects_empty_and_long_values() {
        assert!(matches!(
            validate_reason("   "),
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            validate_reason(&"a".repeat(501)),
            Err(AppError::BadRequest(_))
        ));
    }
}
