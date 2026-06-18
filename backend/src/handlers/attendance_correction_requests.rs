use axum::{
    extract::{Extension, Path, State},
    Json,
};
use timekeeper_app::attendance::{
    AttendanceCorrectionBreak as AppCorrectionBreak,
    AttendanceCorrectionRecord as AppCorrectionRecord,
    AttendanceCorrectionRequestStatus as AppCorrectionStatus,
    AttendanceCorrectionSnapshot as AppCorrectionSnapshot,
    CancelAttendanceCorrectionCommand as AppCancelCorrectionCommand,
    CancelAttendanceCorrectionRequest as CancelCorrectionUseCase,
    CreateAttendanceCorrectionCommand as AppCreateCorrectionCommand,
    CreateAttendanceCorrectionError as AppCreateCorrectionError,
    CreateAttendanceCorrectionRequest as CreateCorrectionUseCase,
    UpdateAttendanceCorrectionCommand as AppUpdateCorrectionCommand,
    UpdateAttendanceCorrectionRepository as AppUpdateCorrectionRepository,
    UpdateAttendanceCorrectionRequest as UpdateCorrectionUseCase,
};
use timekeeper_infra_postgres::attendance_correction::AttendanceCorrectionRepository;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    attendance_correction_request::{
        AttendanceCorrectionResponse, AttendanceCorrectionSnapshot, CorrectionBreakItem,
        CreateAttendanceCorrectionRequest, UpdateAttendanceCorrectionRequest,
    },
    user::User,
};
use crate::state::AppState;

pub(crate) fn create_correction_error_to_app_error(error: AppCreateCorrectionError) -> AppError {
    match error {
        AppCreateCorrectionError::RequestNotFound => {
            AppError::NotFound("Attendance correction request not found".into())
        }
        AppCreateCorrectionError::ReasonRequired => {
            AppError::BadRequest("reason is required".into())
        }
        AppCreateCorrectionError::ReasonTooLong => {
            AppError::BadRequest("reason must be between 1 and 500 characters".into())
        }
        AppCreateCorrectionError::AttendanceNotFound => {
            AppError::NotFound("No attendance record found for specified date".into())
        }
        AppCreateCorrectionError::NotPendingUpdate => {
            AppError::Conflict("Only pending requests can be updated".into())
        }
        AppCreateCorrectionError::NotPendingCancel => {
            AppError::Conflict("Only pending requests can be cancelled".into())
        }
        AppCreateCorrectionError::NoChanges => {
            AppError::BadRequest("At least one field must be changed".into())
        }
        AppCreateCorrectionError::ClockInRequired => {
            AppError::BadRequest("clock_in_time is required".into())
        }
        AppCreateCorrectionError::ClockOutBeforeClockIn => {
            AppError::BadRequest("clock_out_time must be later than clock_in_time".into())
        }
        AppCreateCorrectionError::BreakEndBeforeStart => {
            AppError::BadRequest("break_end_time must be later than break_start_time".into())
        }
        AppCreateCorrectionError::BreakStartBeforeClockIn => {
            AppError::BadRequest("break_start_time must be later than clock_in_time".into())
        }
        AppCreateCorrectionError::BreakEndAfterClockOut => {
            AppError::BadRequest("break_end_time must be earlier than clock_out_time".into())
        }
        AppCreateCorrectionError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn app_status_to_backend(
    status: AppCorrectionStatus,
) -> crate::models::attendance_correction_request::AttendanceCorrectionStatus {
    match status {
        AppCorrectionStatus::Pending => {
            crate::models::attendance_correction_request::AttendanceCorrectionStatus::Pending
        }
        AppCorrectionStatus::Approved => {
            crate::models::attendance_correction_request::AttendanceCorrectionStatus::Approved
        }
        AppCorrectionStatus::Rejected => {
            crate::models::attendance_correction_request::AttendanceCorrectionStatus::Rejected
        }
        AppCorrectionStatus::Cancelled => {
            crate::models::attendance_correction_request::AttendanceCorrectionStatus::Cancelled
        }
        AppCorrectionStatus::Conflict => {
            crate::models::attendance_correction_request::AttendanceCorrectionStatus::Conflict
        }
    }
}

pub(crate) fn backend_snapshot_to_app(
    snapshot: AttendanceCorrectionSnapshot,
) -> AppCorrectionSnapshot {
    AppCorrectionSnapshot {
        clock_in_time: snapshot.clock_in_time,
        clock_out_time: snapshot.clock_out_time,
        breaks: snapshot
            .breaks
            .into_iter()
            .map(|item| AppCorrectionBreak {
                break_start_time: item.break_start_time,
                break_end_time: item.break_end_time,
            })
            .collect(),
    }
}

pub(crate) fn app_snapshot_to_backend(
    snapshot: AppCorrectionSnapshot,
) -> AttendanceCorrectionSnapshot {
    AttendanceCorrectionSnapshot {
        clock_in_time: snapshot.clock_in_time,
        clock_out_time: snapshot.clock_out_time,
        breaks: snapshot
            .breaks
            .into_iter()
            .map(|item| CorrectionBreakItem {
                break_start_time: item.break_start_time,
                break_end_time: item.break_end_time,
            })
            .collect(),
    }
}

pub(crate) fn app_record_to_backend_response(
    record: AppCorrectionRecord,
) -> Result<AttendanceCorrectionResponse, AppError> {
    Ok(AttendanceCorrectionResponse {
        id: record.id,
        user_id: record.user_id,
        attendance_id: record.attendance_id,
        date: record.date,
        status: app_status_to_backend(record.status),
        reason: record.reason,
        original_snapshot: app_snapshot_to_backend(record.original_snapshot),
        proposed_values: app_snapshot_to_backend(record.proposed_values),
        decision_comment: record.decision_comment,
        approved_by: record.approved_by,
        approved_at: record.approved_at,
        rejected_by: record.rejected_by,
        rejected_at: record.rejected_at,
        cancelled_at: record.cancelled_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

pub async fn create_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateAttendanceCorrectionRequest>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    let use_case = CreateCorrectionUseCase::new(AttendanceCorrectionRepository::new(
        state.write_pool.clone(),
    ));
    let request = use_case
        .execute(AppCreateCorrectionCommand {
            request_id: Uuid::new_v4().to_string(),
            user_id: user.id.to_string(),
            date: payload.date,
            clock_in_time: payload.clock_in_time,
            clock_out_time: payload.clock_out_time,
            breaks: payload.breaks.map(|breaks| {
                breaks
                    .into_iter()
                    .map(|item| AppCorrectionBreak {
                        break_start_time: item.break_start_time,
                        break_end_time: item.break_end_time,
                    })
                    .collect()
            }),
            reason: payload.reason,
        })
        .await
        .map_err(create_correction_error_to_app_error)?;

    Ok(Json(app_record_to_backend_response(request)?))
}

pub async fn list_my_attendance_correction_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<AttendanceCorrectionResponse>>, AppError> {
    let repo = AttendanceCorrectionRepository::new(state.read_pool().clone());
    let list = repo
        .list_by_user(&user.id.to_string())
        .await
        .map_err(create_correction_error_to_app_error)?;
    let responses = list
        .into_iter()
        .map(app_record_to_backend_response)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(responses))
}

pub async fn get_my_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    let repo = AttendanceCorrectionRepository::new(state.read_pool().clone());
    let request = repo
        .find_attendance_correction_request_for_user(&id, &user.id.to_string())
        .await
        .map_err(create_correction_error_to_app_error)?;

    Ok(Json(app_record_to_backend_response(request)?))
}

pub async fn update_my_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAttendanceCorrectionRequest>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    let use_case = UpdateCorrectionUseCase::new(AttendanceCorrectionRepository::new(
        state.write_pool.clone(),
    ));
    let updated = use_case
        .execute(AppUpdateCorrectionCommand {
            request_id: id,
            user_id: user.id.to_string(),
            clock_in_time: payload.clock_in_time,
            clock_out_time: payload.clock_out_time,
            breaks: payload.breaks.map(|breaks| {
                breaks
                    .into_iter()
                    .map(|item| AppCorrectionBreak {
                        break_start_time: item.break_start_time,
                        break_end_time: item.break_end_time,
                    })
                    .collect()
            }),
            reason: payload.reason,
        })
        .await
        .map_err(create_correction_error_to_app_error)?;

    Ok(Json(app_record_to_backend_response(updated)?))
}

pub async fn cancel_my_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let use_case = CancelCorrectionUseCase::new(AttendanceCorrectionRepository::new(
        state.write_pool.clone(),
    ));
    use_case
        .execute(AppCancelCorrectionCommand {
            request_id: id.clone(),
            user_id: user.id.to_string(),
        })
        .await
        .map_err(create_correction_error_to_app_error)?;
    Ok(Json(serde_json::json!({ "id": id, "status": "cancelled" })))
}
