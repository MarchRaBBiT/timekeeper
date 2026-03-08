use chrono::Utc;
use serde::Serialize;
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
}
