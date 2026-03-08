use serde::Serialize;

use crate::{
    db::connection::DbPool,
    error::AppError,
    models::{
        attendance_correction_request::AttendanceCorrectionResponse,
        leave_request::LeaveRequestResponse, overtime_request::OvertimeRequestResponse,
    },
    repositories::{
        attendance_correction_request::AttendanceCorrectionRequestRepository,
        request::RequestRepository,
    },
    types::UserId,
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
}
