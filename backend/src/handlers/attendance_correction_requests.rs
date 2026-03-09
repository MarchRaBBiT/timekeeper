use axum::{
    extract::{Extension, Path, State},
    Json,
};

use crate::{
    attendance::application::correction_requests::{
        cancel_user_attendance_correction_request,
        create_attendance_correction_request as create_attendance_correction_request_use_case,
        get_user_attendance_correction_request, list_user_attendance_correction_requests,
        update_user_attendance_correction_request,
    },
    error::AppError,
    models::{
        attendance_correction_request::{
            AttendanceCorrectionResponse, CreateAttendanceCorrectionRequest,
            UpdateAttendanceCorrectionRequest,
        },
        user::User,
    },
    state::AppState,
};

pub use crate::attendance::application::correction_requests::{
    build_proposed_snapshot, build_snapshot, validate_reason, validate_snapshot,
};

pub async fn create_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateAttendanceCorrectionRequest>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    Ok(Json(
        create_attendance_correction_request_use_case(
            state.read_pool(),
            &state.write_pool,
            user.id,
            payload,
        )
        .await?,
    ))
}

pub async fn list_my_attendance_correction_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<AttendanceCorrectionResponse>>, AppError> {
    Ok(Json(
        list_user_attendance_correction_requests(state.read_pool(), user.id).await?,
    ))
}

pub async fn get_my_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    Ok(Json(
        get_user_attendance_correction_request(state.read_pool(), user.id, &id).await?,
    ))
}

pub async fn update_my_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAttendanceCorrectionRequest>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    Ok(Json(
        update_user_attendance_correction_request(
            state.read_pool(),
            &state.write_pool,
            user.id,
            &id,
            payload,
        )
        .await?,
    ))
}

pub async fn cancel_my_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(
        cancel_user_attendance_correction_request(&state.write_pool, user.id, &id).await?,
    ))
}
