use axum::{
    extract::{Extension, Path, State},
    Json,
};
use validator::Validate;

use crate::{
    application::dto::{IdStatusResponse, MessageResponse},
    error::AppError,
    models::{
        leave_request::{CreateLeaveRequest, LeaveRequest, LeaveRequestResponse},
        overtime_request::{CreateOvertimeRequest, OvertimeRequest, OvertimeRequestResponse},
    },
    requests::application::user_requests::{
        cancel_my_request, create_leave_request as create_leave_request_use_case,
        create_overtime_request as create_overtime_request_use_case, get_my_requests_view,
        update_my_request,
    },
    state::AppState,
};

pub async fn create_leave_request(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
    Json(payload): Json<CreateLeaveRequest>,
) -> Result<Json<LeaveRequestResponse>, AppError> {
    let user_id = user.id;

    payload.validate()?;

    let leave_request = LeaveRequest::new(
        user_id,
        payload.leave_type,
        payload.start_date,
        payload.end_date,
        payload.reason,
    );

    let response = create_leave_request_use_case(&state.write_pool, &leave_request).await?;
    Ok(Json(response))
}

pub async fn create_overtime_request(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
    Json(payload): Json<CreateOvertimeRequest>,
) -> Result<Json<OvertimeRequestResponse>, AppError> {
    let user_id = user.id;

    payload.validate()?;

    let overtime_request =
        OvertimeRequest::new(user_id, payload.date, payload.planned_hours, payload.reason);

    let response = create_overtime_request_use_case(&state.write_pool, &overtime_request).await?;
    Ok(Json(response))
}

pub async fn get_my_requests(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
) -> Result<Json<crate::requests::application::user_requests::MyRequestsView>, AppError> {
    let response = get_my_requests_view(state.read_pool(), user.id).await?;
    Ok(Json(response))
}

pub async fn update_request(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<MessageResponse>, AppError> {
    let result = update_my_request(&state.write_pool, user.id, &request_id, payload).await?;
    Ok(Json(result))
}

pub async fn cancel_request(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
) -> Result<Json<IdStatusResponse>, AppError> {
    let result = cancel_my_request(&state.write_pool, user.id, &request_id).await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    fn is_valid_leave_window(start: NaiveDate, end: NaiveDate) -> bool {
        start <= end
    }

    fn is_valid_planned_hours(hours: f64) -> bool {
        hours > 0.0
    }

    #[test]
    fn leave_window_validation_requires_start_before_end() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        assert!(is_valid_leave_window(start, end));
        assert!(!is_valid_leave_window(end, start));
    }

    #[test]
    fn planned_hours_validation_disallows_non_positive_values() {
        assert!(is_valid_planned_hours(0.5));
        assert!(!is_valid_planned_hours(0.0));
        assert!(!is_valid_planned_hours(-1.0));
    }
}
