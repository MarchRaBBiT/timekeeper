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

use crate::services::notification_queue::{
    enqueue_application_notification_job, ApplicationNotificationJob, NotificationJob,
};
use crate::{
    error::AppError,
    models::{
        attendance_correction_request::AttendanceCorrectionResponse,
        leave_request::{
            CreateLeaveRequest, LeaveAcquisitionUnit, LeaveRequest, LeaveRequestResponse, LeaveType,
        },
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
    validate_leave_type_policy(&state, &payload).await?;

    let requested_minutes = match (payload.start_time, payload.end_time) {
        (Some(start), Some(end)) => Some(
            i32::try_from((end - start).num_minutes())
                .map_err(|_| AppError::BadRequest("leave duration is too large".into()))?,
        ),
        _ => None,
    };
    let leave_request = LeaveRequest::new_with_duration(
        user_id,
        leave_type,
        payload.start_date,
        payload.end_date,
        acquisition_unit_to_model(payload.acquisition_unit),
        payload.start_time,
        payload.end_time,
        requested_minutes,
        payload.reason,
    );

    let repo = RequestRepository::new();
    repo.ensure_balance_tracked_leave_request_balance(&state.write_pool, &leave_request)
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
    enqueue_request_submitted_notifications(
        &state,
        &response.user_id.to_string(),
        &response.id.to_string(),
        "leave",
    )
    .await;
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
    enqueue_request_submitted_notifications(
        &state,
        &response.user_id.to_string(),
        &response.id.to_string(),
        "overtime",
    )
    .await;
    Ok(Json(response))
}

async fn enqueue_request_submitted_notifications(
    state: &AppState,
    applicant_id: &str,
    request_id: &str,
    request_kind: &str,
) {
    let Some(redis_pool) = state.redis() else {
        return;
    };
    let recipients = match sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT dm.user_id
         FROM users applicant
         JOIN department_managers dm ON dm.department_id = applicant.department_id
         WHERE applicant.id = $1",
    )
    .bind(applicant_id)
    .fetch_all(state.read_pool())
    .await
    {
        Ok(items) => items,
        Err(error) => {
            tracing::warn!(error = %error, "failed to resolve request notification recipients");
            return;
        }
    };
    for recipient in recipients {
        let job = ApplicationNotificationJob::new(
            recipient,
            Some(applicant_id.to_string()),
            applicant_id.to_string(),
            request_id.to_string(),
            request_kind.to_string(),
            "ja".to_string(),
        );
        if let Err(error) = enqueue_application_notification_job(
            redis_pool,
            &NotificationJob::RequestSubmitted(job),
        )
        .await
        {
            tracing::warn!(error = %error, "failed to enqueue request submitted notification");
        }
    }
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
    match payload.acquisition_unit {
        timekeeper_contract::requests::LeaveAcquisitionUnit::Hour => {
            if payload.start_date != payload.end_date {
                return Err(AppError::BadRequest(
                    "hour leave must be requested for a single day".into(),
                ));
            }
            match (payload.start_time, payload.end_time) {
                (Some(start), Some(end)) if start < end => {}
                _ => {
                    return Err(AppError::BadRequest(
                        "hour leave requires start_time < end_time".into(),
                    ))
                }
            }
        }
        _ if payload.start_time.is_some() || payload.end_time.is_some() => {
            return Err(AppError::BadRequest(
                "start_time and end_time are only valid for hour leave".into(),
            ))
        }
        _ => {}
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
        custom
            if custom
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_lowercase())
                && custom
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_') =>
        {
            Ok(LeaveType::Custom(custom.to_string()))
        }
        _ => Err(AppError::BadRequest("Invalid leave_type".into())),
    }
}

fn acquisition_unit_to_model(
    value: timekeeper_contract::requests::LeaveAcquisitionUnit,
) -> LeaveAcquisitionUnit {
    match value {
        timekeeper_contract::requests::LeaveAcquisitionUnit::Day => LeaveAcquisitionUnit::Day,
        timekeeper_contract::requests::LeaveAcquisitionUnit::HalfAm => LeaveAcquisitionUnit::HalfAm,
        timekeeper_contract::requests::LeaveAcquisitionUnit::HalfPm => LeaveAcquisitionUnit::HalfPm,
        timekeeper_contract::requests::LeaveAcquisitionUnit::Hour => LeaveAcquisitionUnit::Hour,
    }
}

async fn validate_leave_type_policy(
    state: &AppState,
    payload: &CreateLeaveRequest,
) -> Result<(), AppError> {
    let unit = match payload.acquisition_unit {
        timekeeper_contract::requests::LeaveAcquisitionUnit::Day => "day",
        timekeeper_contract::requests::LeaveAcquisitionUnit::HalfAm => "half_am",
        timekeeper_contract::requests::LeaveAcquisitionUnit::HalfPm => "half_pm",
        timekeeper_contract::requests::LeaveAcquisitionUnit::Hour => "hour",
    };
    validate_leave_type_code_policy(state, &payload.leave_type, unit).await
}

async fn validate_leave_type_code_policy(
    state: &AppState,
    leave_type_code: &str,
    acquisition_unit: &str,
) -> Result<(), AppError> {
    let allowed = sqlx::query_scalar::<_, bool>(
        "SELECT is_active AND $2 = ANY(allowed_units)
         FROM leave_types WHERE code = $1",
    )
    .bind(leave_type_code)
    .bind(acquisition_unit)
    .fetch_optional(state.read_pool())
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid leave_type".into()))?;
    if !allowed {
        return Err(AppError::BadRequest(
            "leave type is inactive or does not allow the requested unit".into(),
        ));
    }
    Ok(())
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
        if matches!(
            updated.acquisition_unit,
            LeaveAcquisitionUnit::HalfAm
                | LeaveAcquisitionUnit::HalfPm
                | LeaveAcquisitionUnit::Hour
        ) && new_start != new_end
        {
            return Err(AppError::BadRequest(
                "partial leave must be requested for a single day".into(),
            ));
        }
        let new_reason = upd.reason.or(updated.reason.clone());
        let now = Utc::now();
        updated.leave_type = new_type;
        updated.start_date = new_start;
        updated.end_date = new_end;
        updated.reason = new_reason;
        updated.updated_at = now;
        let acquisition_unit = match updated.acquisition_unit {
            LeaveAcquisitionUnit::Day => "day",
            LeaveAcquisitionUnit::HalfAm => "half_am",
            LeaveAcquisitionUnit::HalfPm => "half_pm",
            LeaveAcquisitionUnit::Hour => "hour",
        };
        validate_leave_type_code_policy(&state, updated.leave_type.db_value(), acquisition_unit)
            .await?;
        // M-4: 更新後の値が残高連動種別なら、作成時
        // (`create_leave_request`) と同じ稼働日ベースの残高検証を更新前に
        // 再実行する。pending 申請は承認まで ledger を消費しないため、この
        // 検証は作成時と全く同じロジックで良い（重複実装を避けるため
        // `RequestRepository::ensure_balance_tracked_leave_request_balance` を共用する）。
        // これにより「更新は成功するのに承認で初めて拒否される」非対称を防ぐ。
        let request_repo = RequestRepository::new();
        request_repo
            .ensure_balance_tracked_leave_request_balance(&state.write_pool, &updated)
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
            acquisition_unit: timekeeper_contract::requests::LeaveAcquisitionUnit::Day,
            start_time: None,
            end_time: None,
            reason: None,
        };

        assert!(super::validate_create_leave_request(&payload).is_err());
        assert!(super::parse_leave_type("Invalid").is_err());
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
