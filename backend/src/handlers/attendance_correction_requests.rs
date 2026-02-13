use axum::{
    extract::{Extension, Path, State},
    Json,
};
use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    attendance::Attendance,
    attendance_correction_request::{
        AttendanceCorrectionResponse, AttendanceCorrectionSnapshot, CorrectionBreakItem,
        CreateAttendanceCorrectionRequest, UpdateAttendanceCorrectionRequest,
    },
    user::User,
};
use crate::repositories::{
    attendance::{AttendanceRepository, AttendanceRepositoryTrait},
    attendance_correction_request::AttendanceCorrectionRequestRepository,
    break_record::BreakRecordRepository,
};
use crate::state::AppState;

pub async fn create_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateAttendanceCorrectionRequest>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    validate_reason(&payload.reason)?;

    let attendance_repo = AttendanceRepository::new();
    let break_repo = BreakRecordRepository::new();

    let attendance = attendance_repo
        .find_by_user_and_date(state.read_pool(), user.id, payload.date)
        .await?
        .ok_or_else(|| {
            AppError::NotFound("No attendance record found for specified date".into())
        })?;

    let original_snapshot = build_snapshot(&attendance, &break_repo, state.read_pool()).await?;
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
            &state.write_pool,
            &Uuid::new_v4().to_string(),
            user.id,
            attendance.id,
            payload.date,
            &payload.reason,
            &original_snapshot,
            &proposed_snapshot,
        )
        .await?;

    Ok(Json(
        request
            .to_response()
            .map_err(|e| AppError::InternalServerError(e.into()))?,
    ))
}

pub async fn list_my_attendance_correction_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<AttendanceCorrectionResponse>>, AppError> {
    let repo = AttendanceCorrectionRequestRepository::new();
    let list = repo.list_by_user(state.read_pool(), user.id).await?;

    let mut responses = Vec::with_capacity(list.len());
    for item in list {
        responses.push(
            item.to_response()
                .map_err(|e| AppError::InternalServerError(e.into()))?,
        );
    }

    Ok(Json(responses))
}

pub async fn get_my_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    let repo = AttendanceCorrectionRequestRepository::new();
    let request = repo
        .find_by_id_for_user(state.read_pool(), &id, user.id)
        .await?;

    Ok(Json(
        request
            .to_response()
            .map_err(|e| AppError::InternalServerError(e.into()))?,
    ))
}

pub async fn update_my_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAttendanceCorrectionRequest>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    validate_reason(&payload.reason)?;

    let repo = AttendanceCorrectionRequestRepository::new();
    let current = repo
        .find_by_id_for_user(state.read_pool(), &id, user.id)
        .await?;

    if current.status.db_value() != "pending" {
        return Err(AppError::Conflict(
            "Only pending requests can be updated".into(),
        ));
    }

    let original_snapshot = current
        .parse_original_snapshot()
        .map_err(|e| AppError::InternalServerError(e.into()))?;
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
        .update_pending_for_user(
            &state.write_pool,
            &id,
            user.id,
            &payload.reason,
            &proposed_snapshot,
        )
        .await?;

    Ok(Json(
        updated
            .to_response()
            .map_err(|e| AppError::InternalServerError(e.into()))?,
    ))
}

pub async fn cancel_my_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = AttendanceCorrectionRequestRepository::new();
    repo.cancel_pending_for_user(&state.write_pool, &id, user.id)
        .await?;
    Ok(Json(serde_json::json!({ "id": id, "status": "cancelled" })))
}

pub fn build_proposed_snapshot(
    original: &AttendanceCorrectionSnapshot,
    clock_in_time: Option<NaiveDateTime>,
    clock_out_time: Option<NaiveDateTime>,
    breaks: Option<Vec<CorrectionBreakItem>>,
) -> Result<AttendanceCorrectionSnapshot, AppError> {
    let proposed = AttendanceCorrectionSnapshot {
        clock_in_time: clock_in_time.or(original.clock_in_time),
        clock_out_time: clock_out_time.or(original.clock_out_time),
        breaks: breaks.unwrap_or_else(|| original.breaks.clone()),
    };
    Ok(proposed)
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

    for br in &snapshot.breaks {
        if let Some(end) = br.break_end_time {
            if br.break_start_time > end {
                return Err(AppError::BadRequest(
                    "break_end_time must be later than break_start_time".into(),
                ));
            }
            if br.break_start_time < clock_in {
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

fn validate_reason(reason: &str) -> Result<(), AppError> {
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
