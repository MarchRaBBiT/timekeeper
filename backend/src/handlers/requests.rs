use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde_json::{json, Value};
use timekeeper_app::attendance::{
    AttendanceCorrectionRequestStatus, CancelAttendanceCorrectionRepository,
    CreateAttendanceCorrectionError, UpdateAttendanceCorrectionRepository,
    UpdatedAttendanceCorrectionRequest,
};
use timekeeper_infra_postgres::attendance_correction::AttendanceCorrectionRepository;

use crate::{
    error::AppError,
    models::{
        attendance_correction_request::AttendanceCorrectionResponse,
        leave_request::{CreateLeaveRequest, LeaveRequest, LeaveRequestResponse, LeaveType},
        overtime_request::{CreateOvertimeRequest, OvertimeRequest, OvertimeRequestResponse},
    },
    repositories::{
        leave_request::{LeaveRequestRepository, LeaveRequestRepositoryTrait},
        overtime_request::{OvertimeRequestRepository, OvertimeRequestRepositoryTrait},
        request::{RequestCreate, RequestRecord, RequestRepository},
    },
    state::AppState,
    types::{LeaveRequestId, OvertimeRequestId},
};

use chrono::Utc;
use serde::Deserialize;
use std::str::FromStr;

use super::attendance_correction_requests::{
    app_record_to_backend_response, backend_snapshot_to_app, create_correction_error_to_app_error,
};

const MAX_REQUEST_REASON_LENGTH: usize = 500;

pub async fn create_leave_request(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
    Json(payload): Json<CreateLeaveRequest>,
) -> Result<Json<LeaveRequestResponse>, AppError> {
    let user_id = user.id;

    validate_create_leave_request(&payload)?;
    let leave_type = parse_leave_type(&payload.leave_type)?;

    let leave_request = LeaveRequest::new(
        user_id,
        leave_type,
        payload.start_date,
        payload.end_date,
        payload.reason,
    );

    let repo = RequestRepository::new();
    repo.ensure_annual_leave_request_balance(&state.write_pool, &leave_request)
        .await?;
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

    validate_create_overtime_request(&payload)?;

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
    let user_id = user.id;

    let repo = RequestRepository::new();
    let requests = repo.get_user_requests(state.read_pool(), user_id).await?;
    let correction_repo = AttendanceCorrectionRepository::new(state.read_pool().clone());
    let corrections = correction_repo
        .list_by_user(&user_id.to_string())
        .await
        .map_err(create_correction_error_to_app_error)?;
    let correction_responses: Vec<AttendanceCorrectionResponse> = corrections
        .into_iter()
        .map(app_record_to_backend_response)
        .collect::<Result<Vec<_>, _>>()?;

    let response = json!({
        "leave_requests": requests.leave_requests.into_iter().map(LeaveRequestResponse::from).collect::<Vec<_>>(),
        "overtime_requests": requests.overtime_requests.into_iter().map(OvertimeRequestResponse::from).collect::<Vec<_>>(),
        "attendance_corrections": correction_responses
    });

    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct UpdateLeavePayload {
    pub leave_type: Option<LeaveType>,
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

fn validate_create_leave_request(payload: &CreateLeaveRequest) -> Result<(), AppError> {
    if payload.start_date > payload.end_date {
        return Err(AppError::BadRequest(
            "start_date must be <= end_date".into(),
        ));
    }
    validate_optional_reason(&payload.reason)?;
    Ok(())
}

fn validate_create_overtime_request(payload: &CreateOvertimeRequest) -> Result<(), AppError> {
    if !(0.5..=24.0).contains(&payload.planned_hours) {
        return Err(AppError::BadRequest(
            "planned_hours must be between 0.5 and 24".into(),
        ));
    }
    validate_optional_reason(&payload.reason)?;
    Ok(())
}

fn validate_optional_reason(reason: &Option<String>) -> Result<(), AppError> {
    if reason
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAX_REQUEST_REASON_LENGTH)
    {
        return Err(AppError::BadRequest(format!(
            "reason must be at most {} characters",
            MAX_REQUEST_REASON_LENGTH
        )));
    }
    Ok(())
}

fn parse_leave_type(value: &str) -> Result<LeaveType, AppError> {
    match value {
        "annual" => Ok(LeaveType::Annual),
        "sick" => Ok(LeaveType::Sick),
        "personal" => Ok(LeaveType::Personal),
        "other" => Ok(LeaveType::Other),
        _ => Err(AppError::BadRequest("Invalid leave_type".into())),
    }
}

pub async fn update_request(
    State(state): State<AppState>,
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
        .find_by_id_for_user(&state.write_pool, leave_request_id, user_id)
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
        // M-4: 更新後の値が annual のままなら、作成時
        // (`create_leave_request`) と同じ稼働日ベースの残高検証を更新前に
        // 再実行する。pending 申請は承認まで ledger を消費しないため、この
        // 検証は作成時と全く同じロジックで良い（重複実装を避けるため
        // `RequestRepository::ensure_annual_leave_request_balance` を共用する）。
        // これにより「更新は成功するのに承認で初めて拒否される」非対称を防ぐ。
        let request_repo = RequestRepository::new();
        request_repo
            .ensure_annual_leave_request_balance(&state.write_pool, &updated)
            .await?;
        leave_repo.update(&state.write_pool, &updated).await?;
        return Ok(Json(json!({"message":"Leave request updated"})));
    }

    // Try overtime update
    let overtime_request_id = OvertimeRequestId::from_str(&request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;

    let overtime_repo = OvertimeRequestRepository::new();
    if let Some(req) = overtime_repo
        .find_by_id_for_user(&state.write_pool, overtime_request_id, user_id)
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
        overtime_repo.update(&state.write_pool, &updated).await?;
        return Ok(Json(json!({"message":"Overtime request updated"})));
    }

    // Try attendance correction request update
    let correction_repo = AttendanceCorrectionRepository::new(state.write_pool.clone());
    match correction_repo
        .find_attendance_correction_request_for_user(&request_id, &user_id.to_string())
        .await
    {
        Ok(current) => {
            if current.status != AttendanceCorrectionRequestStatus::Pending {
                return Err(AppError::BadRequest(
                    "Only pending requests can be updated".into(),
                ));
            }

            let reason = payload
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::BadRequest("reason is required".into()))?;

            let proposed_values_json = payload
                .get("proposed_values")
                .cloned()
                .ok_or_else(|| AppError::BadRequest("proposed_values is required".into()))?;
            let proposed: crate::models::attendance_correction_request::AttendanceCorrectionSnapshot =
                serde_json::from_value(proposed_values_json)
                .map_err(|_| AppError::BadRequest("Invalid proposed_values".into()))?;

            correction_repo
                .update_pending_attendance_correction_request(UpdatedAttendanceCorrectionRequest {
                    id: request_id.clone(),
                    user_id: user_id.to_string(),
                    reason,
                    proposed_values: backend_snapshot_to_app(proposed),
                })
                .await
                .map_err(create_correction_error_to_app_error)?;
            return Ok(Json(
                json!({"message":"Attendance correction request updated"}),
            ));
        }
        Err(CreateAttendanceCorrectionError::RequestNotFound) => {}
        Err(err) => return Err(create_correction_error_to_app_error(err)),
    }

    Err(AppError::NotFound("Request not found".into()))
}

pub async fn cancel_request(
    State(state): State<AppState>,
    Extension(user): Extension<crate::models::user::User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let user_id = user.id;
    let now = Utc::now();

    // Try leave cancellation first
    let leave_request_id = LeaveRequestId::from_str(&request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;
    let request_repo = RequestRepository::new();
    let result = request_repo
        .cancel_leave_request_with_ledger(&state.write_pool, leave_request_id, user_id, now)
        .await?;
    if result > 0 {
        return Ok(Json(json!({"id": request_id, "status":"cancelled"})));
    }

    // Try overtime cancellation
    let overtime_request_id = OvertimeRequestId::from_str(&request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;
    let overtime_repo = OvertimeRequestRepository::new();
    let result = overtime_repo
        .cancel(&state.write_pool, overtime_request_id, user_id, Utc::now())
        .await?;
    if result > 0 {
        return Ok(Json(json!({"id": request_id, "status":"cancelled"})));
    }

    // Try attendance correction cancellation
    let correction_repo = AttendanceCorrectionRepository::new(state.write_pool.clone());
    match correction_repo
        .cancel_pending_attendance_correction_request(&request_id, &user_id.to_string())
        .await
    {
        Ok(_) => return Ok(Json(json!({"id": request_id, "status":"cancelled"}))),
        Err(CreateAttendanceCorrectionError::NotPendingCancel)
        | Err(CreateAttendanceCorrectionError::RequestNotFound) => {}
        Err(err) => return Err(create_correction_error_to_app_error(err)),
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

    #[test]
    fn create_leave_validation_rejects_invalid_type_and_window() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let payload = super::CreateLeaveRequest {
            leave_type: "annual".into(),
            start_date: start,
            end_date: end,
            reason: None,
        };

        assert!(super::validate_create_leave_request(&payload).is_err());
        assert!(super::parse_leave_type("invalid").is_err());
        assert!(matches!(
            super::parse_leave_type("sick").unwrap(),
            super::LeaveType::Sick
        ));
    }

    #[test]
    fn create_overtime_validation_rejects_out_of_range_hours() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let valid = super::CreateOvertimeRequest {
            date,
            planned_hours: 0.5,
            reason: None,
        };
        let too_low = super::CreateOvertimeRequest {
            planned_hours: 0.25,
            ..valid.clone()
        };
        let too_high = super::CreateOvertimeRequest {
            planned_hours: 24.5,
            ..valid.clone()
        };

        assert!(super::validate_create_overtime_request(&valid).is_ok());
        assert!(super::validate_create_overtime_request(&too_low).is_err());
        assert!(super::validate_create_overtime_request(&too_high).is_err());
    }
}
