use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::error::AppError;
use crate::handlers::attendance_correction_requests::build_snapshot;
use crate::models::attendance_correction_request::{AttendanceCorrectionResponse, DecisionPayload};
use crate::models::user::User;
use crate::repositories::{
    attendance::{AttendanceRepository, AttendanceRepositoryTrait},
    attendance_correction_request::AttendanceCorrectionRequestRepository,
    break_record::BreakRecordRepository,
};
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize)]
pub struct AdminAttendanceCorrectionListQuery {
    pub status: Option<String>,
    pub user_id: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_attendance_correction_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<AdminAttendanceCorrectionListQuery>,
) -> Result<Json<Vec<AttendanceCorrectionResponse>>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let user_filter = match query.user_id {
        Some(raw) => Some(
            raw.parse()
                .map_err(|_| AppError::BadRequest("invalid user_id".into()))?,
        ),
        None => None,
    };

    let repo = AttendanceCorrectionRequestRepository::new();
    let list = repo
        .list_paginated(
            state.read_pool(),
            query.status.as_deref(),
            user_filter,
            page,
            per_page,
        )
        .await?;

    let mut responses = Vec::with_capacity(list.len());
    for item in list {
        responses.push(
            item.to_response()
                .map_err(|e| AppError::InternalServerError(e.into()))?,
        );
    }

    Ok(Json(responses))
}

pub async fn get_attendance_correction_request_detail(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<AttendanceCorrectionResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let repo = AttendanceCorrectionRequestRepository::new();
    let request = repo.find_by_id(state.read_pool(), &id).await?;

    Ok(Json(
        request
            .to_response()
            .map_err(|e| AppError::InternalServerError(e.into()))?,
    ))
}

pub async fn approve_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<DecisionPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    validate_comment(&payload.comment)?;

    let repo = AttendanceCorrectionRequestRepository::new();
    let request = repo.find_by_id(&state.write_pool, &id).await?;

    if request.status.db_value() != "pending" {
        return Err(AppError::Conflict(
            "Request not found or already processed".into(),
        ));
    }

    let attendance_repo = AttendanceRepository::new();
    let break_repo = BreakRecordRepository::new();
    let attendance = attendance_repo
        .find_by_id(state.read_pool(), request.attendance_id)
        .await?;

    let latest_snapshot = build_snapshot(&attendance, &break_repo, state.read_pool()).await?;
    let original_snapshot = request
        .parse_original_snapshot()
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if latest_snapshot != original_snapshot {
        return Err(AppError::Conflict(
            "Attendance record changed after request submission. Please resubmit.".into(),
        ));
    }

    let proposed = request
        .parse_proposed_values()
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    repo.upsert_effective_values(
        &state.write_pool,
        request.attendance_id,
        &request.id,
        &proposed,
        user.id,
    )
    .await?;

    repo.approve(&state.write_pool, &id, user.id, &payload.comment)
        .await?;

    Ok(Json(serde_json::json!({ "message": "Request approved" })))
}

pub async fn reject_attendance_correction_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<DecisionPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    validate_comment(&payload.comment)?;

    let repo = AttendanceCorrectionRequestRepository::new();
    repo.reject(&state.write_pool, &id, user.id, &payload.comment)
        .await?;
    Ok(Json(serde_json::json!({ "message": "Request rejected" })))
}

fn validate_comment(comment: &str) -> Result<(), AppError> {
    if comment.trim().is_empty() {
        return Err(AppError::BadRequest("comment is required".into()));
    }
    if comment.chars().count() > 500 {
        return Err(AppError::BadRequest(
            "comment must be between 1 and 500 characters".into(),
        ));
    }
    Ok(())
}
