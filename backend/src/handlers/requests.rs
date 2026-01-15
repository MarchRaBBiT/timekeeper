use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use validator::Validate;

use crate::{
    config::Config,
    error::AppError,
    models::{
        leave_request::{CreateLeaveRequest, LeaveRequest, LeaveRequestResponse},
        overtime_request::{CreateOvertimeRequest, OvertimeRequest, OvertimeRequestResponse},
    },
    repositories::{repository::Repository, LeaveRequestRepository, OvertimeRequestRepository},
    types::{LeaveRequestId, OvertimeRequestId},
};

use chrono::Utc;
use serde::Deserialize;
use std::str::FromStr;

pub async fn create_leave_request(
    State((pool, _config)): State<(PgPool, Config)>,
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

    let repo = LeaveRequestRepository::new();
    let saved = repo.create(&pool, &leave_request).await?;
    let response = LeaveRequestResponse::from(saved);
    Ok(Json(response))
}

pub async fn create_overtime_request(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
    Json(payload): Json<CreateOvertimeRequest>,
) -> Result<Json<OvertimeRequestResponse>, AppError> {
    let user_id = user.id;

    payload.validate()?;

    let overtime_request =
        OvertimeRequest::new(user_id, payload.date, payload.planned_hours, payload.reason);

    let repo = OvertimeRequestRepository::new();
    let saved = repo.create(&pool, &overtime_request).await?;
    let response = OvertimeRequestResponse::from(saved);
    Ok(Json(response))
}

pub async fn get_my_requests(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = user.id;

    let leave_repo = LeaveRequestRepository::new();
    let leave_requests = leave_repo.find_by_user(&pool, user_id).await?;

    let overtime_repo = OvertimeRequestRepository::new();
    let overtime_requests = overtime_repo.find_by_user(&pool, user_id).await?;

    let response = json!({
        "leave_requests": leave_requests.into_iter().map(LeaveRequestResponse::from).collect::<Vec<_>>(),
        "overtime_requests": overtime_requests.into_iter().map(OvertimeRequestResponse::from).collect::<Vec<_>>()
    });

    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct UpdateLeavePayload {
    pub leave_type: Option<crate::models::leave_request::LeaveType>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateOvertimePayload {
    pub date: Option<chrono::NaiveDate>,
    pub planned_hours: Option<f64>,
    pub reason: Option<String>,
}

pub async fn update_request(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    let user_id = user.id;

    // Try leave update first
    let leave_request_id = LeaveRequestId::from_str(&request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;

    let leave_repo = LeaveRequestRepository::new();
    if let Some(req) = leave_repo
        .find_by_id_for_user(&pool, leave_request_id, user_id)
        .await?
    {
        if !req.is_pending() {
            return Err(AppError::BadRequest(
                "Only pending requests can be updated".into(),
            ));
        }
        let upd: UpdateLeavePayload = serde_json::from_value(payload.clone())
            .map_err(|_| AppError::BadRequest("Invalid payload".into()))?;
        let mut updated = req;
        let new_type = upd.leave_type.unwrap_or_else(|| updated.leave_type.clone());
        let new_start = upd.start_date.unwrap_or(updated.start_date);
        let new_end = upd.end_date.unwrap_or(updated.end_date);
        if new_start > new_end {
            return Err(AppError::BadRequest(
                "start_date must be <= end_date".into(),
            ));
        }
        let new_reason = upd.reason.or(updated.reason.clone());
        let now = Utc::now();
        updated.leave_type = new_type;
        updated.start_date = new_start;
        updated.end_date = new_end;
        updated.reason = new_reason;
        updated.updated_at = now;
        leave_repo.update(&pool, &updated).await?;
        return Ok(Json(json!({"message":"Leave request updated"})));
    }

    // Try overtime update
    let overtime_request_id = OvertimeRequestId::from_str(&request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;

    let overtime_repo = OvertimeRequestRepository::new();
    if let Some(req) = overtime_repo
        .find_by_id_for_user(&pool, overtime_request_id, user_id)
        .await?
    {
        if !req.is_pending() {
            return Err(AppError::BadRequest(
                "Only pending requests can be updated".into(),
            ));
        }
        let upd: UpdateOvertimePayload = serde_json::from_value(payload.clone())
            .map_err(|_| AppError::BadRequest("Invalid payload".into()))?;
        let new_date = upd.date.unwrap_or(req.date);
        let new_hours = upd.planned_hours.unwrap_or(req.planned_hours);
        if new_hours <= 0.0 {
            return Err(AppError::BadRequest("planned_hours must be > 0".into()));
        }
        let new_reason = upd.reason.or(req.reason.clone());
        let now = Utc::now();
        let mut updated = req;
        updated.date = new_date;
        updated.planned_hours = new_hours;
        updated.reason = new_reason;
        updated.updated_at = now;
        overtime_repo.update(&pool, &updated).await?;
        return Ok(Json(json!({"message":"Overtime request updated"})));
    }

    Err(AppError::NotFound("Request not found".into()))
}

pub async fn cancel_request(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let user_id = user.id;
    let now = Utc::now();

    // Try leave cancellation first
    let leave_request_id = LeaveRequestId::from_str(&request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;
    let leave_repo = LeaveRequestRepository::new();
    let result = leave_repo
        .cancel(&pool, leave_request_id, user_id, now)
        .await?;
    if result > 0 {
        return Ok(Json(json!({"id": request_id, "status":"cancelled"})));
    }

    // Try overtime cancellation
    let overtime_request_id = OvertimeRequestId::from_str(&request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;
    let overtime_repo = OvertimeRequestRepository::new();
    let result = overtime_repo
        .cancel(&pool, overtime_request_id, user_id, Utc::now())
        .await?;
    if result > 0 {
        return Ok(Json(json!({"id": request_id, "status":"cancelled"})));
    }

    Err(AppError::NotFound(
        "Request not found or not cancellable".into(),
    ))
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
