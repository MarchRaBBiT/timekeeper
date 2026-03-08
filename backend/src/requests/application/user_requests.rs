use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

use crate::{
    db::connection::DbPool,
    error::AppError,
    models::{
        attendance_correction_request::AttendanceCorrectionResponse,
        leave_request::LeaveRequestResponse, overtime_request::OvertimeRequestResponse,
    },
    repositories::{
        attendance_correction_request::AttendanceCorrectionRequestRepository,
        leave_request::{LeaveRequestRepository, LeaveRequestRepositoryTrait},
        overtime_request::{OvertimeRequestRepository, OvertimeRequestRepositoryTrait},
        request::RequestRepository,
    },
    types::{LeaveRequestId, OvertimeRequestId, UserId},
};

#[derive(Debug, Deserialize)]
struct UpdateLeavePayload {
    leave_type: Option<crate::models::leave_request::LeaveType>,
    start_date: Option<chrono::NaiveDate>,
    end_date: Option<chrono::NaiveDate>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateOvertimePayload {
    date: Option<chrono::NaiveDate>,
    planned_hours: Option<f64>,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MyRequestsView {
    pub leave_requests: Vec<LeaveRequestResponse>,
    pub overtime_requests: Vec<OvertimeRequestResponse>,
    pub attendance_corrections: Vec<AttendanceCorrectionResponse>,
}

pub async fn get_my_requests_view(
    read_pool: &DbPool,
    user_id: UserId,
) -> Result<MyRequestsView, AppError> {
    let repo = RequestRepository::new();
    let requests = repo.get_user_requests(read_pool, user_id).await?;

    let correction_repo = AttendanceCorrectionRequestRepository::new();
    let corrections = correction_repo.list_by_user(read_pool, user_id).await?;
    let attendance_corrections = corrections
        .iter()
        .map(|item| {
            item.to_response()
                .map_err(|error| AppError::InternalServerError(error.into()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MyRequestsView {
        leave_requests: requests
            .leave_requests
            .into_iter()
            .map(LeaveRequestResponse::from)
            .collect(),
        overtime_requests: requests
            .overtime_requests
            .into_iter()
            .map(OvertimeRequestResponse::from)
            .collect(),
        attendance_corrections,
    })
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CancelRequestResult {
    pub id: String,
    pub status: &'static str,
}

pub async fn cancel_my_request(
    write_pool: &DbPool,
    user_id: UserId,
    request_id: &str,
) -> Result<CancelRequestResult, AppError> {
    let now = Utc::now();

    let leave_request_id = LeaveRequestId::from_str(request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;
    let leave_repo = LeaveRequestRepository::new();
    let result = leave_repo
        .cancel(write_pool, leave_request_id, user_id, now)
        .await?;
    if result > 0 {
        return Ok(CancelRequestResult {
            id: request_id.to_string(),
            status: "cancelled",
        });
    }

    let overtime_request_id = OvertimeRequestId::from_str(request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;
    let overtime_repo = OvertimeRequestRepository::new();
    let result = overtime_repo
        .cancel(write_pool, overtime_request_id, user_id, Utc::now())
        .await?;
    if result > 0 {
        return Ok(CancelRequestResult {
            id: request_id.to_string(),
            status: "cancelled",
        });
    }

    let correction_repo = AttendanceCorrectionRequestRepository::new();
    match correction_repo
        .cancel_pending_for_user(write_pool, request_id, user_id)
        .await
    {
        Ok(_) => Ok(CancelRequestResult {
            id: request_id.to_string(),
            status: "cancelled",
        }),
        Err(AppError::Conflict(_)) | Err(AppError::NotFound(_)) => Err(AppError::NotFound(
            "Request not found or not cancellable".into(),
        )),
        Err(err) => Err(err),
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct UpdateRequestResult {
    pub message: &'static str,
}

pub async fn update_my_request(
    write_pool: &DbPool,
    user_id: UserId,
    request_id: &str,
    payload: Value,
) -> Result<UpdateRequestResult, AppError> {
    let leave_request_id = LeaveRequestId::from_str(request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;

    let leave_repo = LeaveRequestRepository::new();
    if let Some(req) = leave_repo
        .find_by_id_for_user(write_pool, leave_request_id, user_id)
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
        updated.leave_type = new_type;
        updated.start_date = new_start;
        updated.end_date = new_end;
        updated.reason = new_reason;
        updated.updated_at = Utc::now();
        leave_repo.update(write_pool, &updated).await?;
        return Ok(UpdateRequestResult {
            message: "Leave request updated",
        });
    }

    let overtime_request_id = OvertimeRequestId::from_str(request_id)
        .map_err(|_| AppError::BadRequest("Invalid request ID format".into()))?;
    let overtime_repo = OvertimeRequestRepository::new();
    if let Some(req) = overtime_repo
        .find_by_id_for_user(write_pool, overtime_request_id, user_id)
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
        let mut updated = req;
        updated.date = new_date;
        updated.planned_hours = new_hours;
        updated.reason = new_reason;
        updated.updated_at = Utc::now();
        overtime_repo.update(write_pool, &updated).await?;
        return Ok(UpdateRequestResult {
            message: "Overtime request updated",
        });
    }

    let correction_repo = AttendanceCorrectionRequestRepository::new();
    match correction_repo
        .find_by_id_for_user(write_pool, request_id, user_id)
        .await
    {
        Ok(current) => {
            if current.status.db_value() != "pending" {
                return Err(AppError::BadRequest(
                    "Only pending requests can be updated".into(),
                ));
            }

            let reason = payload
                .get("reason")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| AppError::BadRequest("reason is required".into()))?;

            let proposed_values_json = payload
                .get("proposed_values")
                .cloned()
                .ok_or_else(|| AppError::BadRequest("proposed_values is required".into()))?;
            let proposed = serde_json::from_value(proposed_values_json)
                .map_err(|_| AppError::BadRequest("Invalid proposed_values".into()))?;

            correction_repo
                .update_pending_for_user(write_pool, request_id, user_id, &reason, &proposed)
                .await?;
            Ok(UpdateRequestResult {
                message: "Attendance correction request updated",
            })
        }
        Err(AppError::NotFound(_)) => Err(AppError::NotFound("Request not found".into())),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_requests_view_stores_grouped_responses() {
        let view = MyRequestsView {
            leave_requests: Vec::new(),
            overtime_requests: Vec::new(),
            attendance_corrections: Vec::new(),
        };

        assert!(view.leave_requests.is_empty());
        assert!(view.overtime_requests.is_empty());
        assert!(view.attendance_corrections.is_empty());
    }

    #[test]
    fn cancel_request_result_uses_cancelled_status() {
        let result = CancelRequestResult {
            id: "request-1".to_string(),
            status: "cancelled",
        };

        assert_eq!(result.id, "request-1");
        assert_eq!(result.status, "cancelled");
    }

    #[test]
    fn update_request_result_keeps_message() {
        let result = UpdateRequestResult {
            message: "Leave request updated",
        };

        assert_eq!(result.message, "Leave request updated");
    }
}
