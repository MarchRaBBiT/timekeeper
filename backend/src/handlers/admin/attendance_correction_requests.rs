use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::Deserialize;
use timekeeper_app::attendance::{
    AdminAttendanceCorrectionReadError as AppReadCorrectionError,
    ApproveAttendanceCorrectionCommand as AppApproveCorrectionCommand,
    ApproveAttendanceCorrectionRequest as ApproveCorrectionUseCase,
    AttendanceCorrectionDecisionError as AppDecisionError,
    GetAdminAttendanceCorrectionRequest as GetAdminCorrectionUseCase,
    GetAdminAttendanceCorrectionRequestQuery as AppGetAdminCorrectionQuery,
    ListAdminAttendanceCorrectionRequests as ListAdminCorrectionsUseCase,
    ListAdminAttendanceCorrectionRequestsQuery as AppListAdminCorrectionsQuery,
    RejectAttendanceCorrectionCommand as AppRejectCorrectionCommand,
    RejectAttendanceCorrectionRequest as RejectCorrectionUseCase,
};
use timekeeper_infra_postgres::attendance_correction::AttendanceCorrectionRepository;

use crate::error::AppError;
use crate::models::attendance_correction_request::{AttendanceCorrectionResponse, DecisionPayload};
use crate::models::user::User;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize)]
pub struct AdminAttendanceCorrectionListQuery {
    pub status: Option<String>,
    pub user_id: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

fn read_error_to_app_error(error: AppReadCorrectionError) -> AppError {
    match error {
        AppReadCorrectionError::RequestNotFound => {
            AppError::NotFound("Attendance correction request not found".into())
        }
        AppReadCorrectionError::InvalidUserId => AppError::BadRequest("invalid user_id".into()),
        AppReadCorrectionError::Forbidden => AppError::Forbidden("Forbidden".into()),
        AppReadCorrectionError::ManagerNotAuthorized => {
            AppError::Forbidden("Manager does not have permission to approve this request".into())
        }
        AppReadCorrectionError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn decision_error_to_app_error(error: AppDecisionError) -> AppError {
    match error {
        AppDecisionError::RequestNotFound => {
            AppError::NotFound("Attendance correction request not found".into())
        }
        AppDecisionError::Forbidden => AppError::Forbidden("Forbidden".into()),
        AppDecisionError::ManagerNotAuthorized => {
            AppError::Forbidden("Manager does not have permission to approve this request".into())
        }
        AppDecisionError::SelfDecision => {
            AppError::Forbidden("Admins cannot approve or reject their own requests".into())
        }
        AppDecisionError::CommentRequired => AppError::BadRequest("comment is required".into()),
        AppDecisionError::CommentTooLong => {
            AppError::BadRequest("comment must be between 1 and 500 characters".into())
        }
        AppDecisionError::AlreadyProcessed => {
            AppError::Conflict("Request not found or already processed".into())
        }
        AppDecisionError::AttendanceChanged => AppError::Conflict(
            "Attendance record changed after request submission. Please resubmit.".into(),
        ),
        AppDecisionError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

pub async fn list_attendance_correction_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<AdminAttendanceCorrectionListQuery>,
) -> Result<Json<Vec<AttendanceCorrectionResponse>>, AppError> {
    let use_case = ListAdminCorrectionsUseCase::new(AttendanceCorrectionRepository::new(
        state.read_pool().clone(),
    ));
    let list = use_case
        .execute(AppListAdminCorrectionsQuery {
            requester_id: user.id.to_string(),
            requester_is_manager: user.is_manager(),
            requester_is_system_admin: user.is_system_admin(),
            status: query.status,
            user_id: query.user_id,
            page: query.page,
            per_page: query.per_page,
        })
        .await
        .map_err(read_error_to_app_error)?;
    let responses = list
        .into_iter()
        .map(crate::handlers::attendance_correction_requests::app_record_to_backend_response)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(responses))
}

pub async fn get_attendance_correction_request_detail(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    let use_case = GetAdminCorrectionUseCase::new(AttendanceCorrectionRepository::new(
        state.read_pool().clone(),
    ));
    let request = use_case
        .execute(AppGetAdminCorrectionQuery {
            requester_id: user.id.to_string(),
            requester_is_manager: user.is_manager(),
            requester_is_system_admin: user.is_system_admin(),
            request_id: id,
        })
        .await
        .map_err(read_error_to_app_error)?;

    Ok(Json(
        crate::handlers::attendance_correction_requests::app_record_to_backend_response(request)?,
    ))
}

pub async fn approve_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<DecisionPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let use_case = ApproveCorrectionUseCase::new(AttendanceCorrectionRepository::new(
        state.write_pool.clone(),
    ));
    use_case
        .execute(AppApproveCorrectionCommand {
            request_id: id,
            approver_id: user.id.to_string(),
            approver_is_manager: user.is_manager(),
            approver_is_system_admin: user.is_system_admin(),
            comment: payload.comment,
        })
        .await
        .map_err(decision_error_to_app_error)?;

    Ok(Json(serde_json::json!({ "message": "Request approved" })))
}

pub async fn reject_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<DecisionPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let use_case = RejectCorrectionUseCase::new(AttendanceCorrectionRepository::new(
        state.write_pool.clone(),
    ));
    use_case
        .execute(AppRejectCorrectionCommand {
            request_id: id,
            approver_id: user.id.to_string(),
            approver_is_manager: user.is_manager(),
            approver_is_system_admin: user.is_system_admin(),
            comment: payload.comment,
        })
        .await
        .map_err(decision_error_to_app_error)?;
    Ok(Json(serde_json::json!({ "message": "Request rejected" })))
}
