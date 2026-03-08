use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde_json::{json, Value};
use validator::Validate;

use crate::{
    error::AppError,
    models::{
        leave_request::{CreateLeaveRequest, LeaveRequest, LeaveRequestResponse},
        overtime_request::{CreateOvertimeRequest, OvertimeRequest, OvertimeRequestResponse},
    },
    repositories::request::{RequestCreate, RequestRecord, RequestRepository},
    requests::application::user_requests::{
        cancel_my_request, get_my_requests_view, update_my_request,
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

    let repo = RequestRepository::new();
    let saved = repo
        .create_request_with_history(&state.write_pool, RequestCreate::Leave(&leave_request))
        .await?;
    let response = match saved {
        RequestRecord::Leave(item) => LeaveRequestResponse::from(item),
        RequestRecord::Overtime(_) => {
            return Err(AppError::InternalServerError(anyhow::anyhow!(
                "Repository returned OvertimeRequest when LeaveRequest was expected"
            )))
        }
    };
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

    let repo = RequestRepository::new();
    let saved = repo
        .create_request_with_history(
            &state.write_pool,
            RequestCreate::Overtime(&overtime_request),
        )
        .await?;
    let response = match saved {
        RequestRecord::Overtime(item) => OvertimeRequestResponse::from(item),
        RequestRecord::Leave(_) => {
            return Err(AppError::InternalServerError(anyhow::anyhow!(
                "Repository returned LeaveRequest when OvertimeRequest was expected"
            )))
        }
    };
    Ok(Json(response))
}

pub async fn get_my_requests(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
) -> Result<Json<serde_json::Value>, AppError> {
    let response = get_my_requests_view(state.read_pool(), user.id).await?;
    Ok(Json(json!(response)))
}

pub async fn update_request(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    let result = update_my_request(&state.write_pool, user.id, &request_id, payload).await?;
    Ok(Json(json!(result)))
}

pub async fn cancel_request(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let result = cancel_my_request(&state.write_pool, user.id, &request_id).await?;
    Ok(Json(json!(result)))
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
