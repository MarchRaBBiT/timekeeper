use crate::api::{
    ApiClient, ApiError, CreateAttendanceCorrectionRequest, CreateLeaveRequest,
    CreateOvertimeRequest, LeaveRequestResponse, OvertimeRequestResponse,
    UpdateAttendanceCorrectionRequest,
};
use serde_json::Value;
use std::rc::Rc;

use super::types::MyRequestsResponse;

#[derive(Clone)]
pub struct RequestsRepository {
    client: Rc<ApiClient>,
}

impl RequestsRepository {
    pub fn new(api: ApiClient) -> Self {
        Self {
            client: Rc::new(api),
        }
    }

    pub async fn submit_leave(
        &self,
        payload: CreateLeaveRequest,
    ) -> Result<LeaveRequestResponse, ApiError> {
        self.client.create_leave_request(payload).await
    }

    pub async fn submit_overtime(
        &self,
        payload: CreateOvertimeRequest,
    ) -> Result<OvertimeRequestResponse, ApiError> {
        self.client.create_overtime_request(payload).await
    }

    pub async fn submit_attendance_correction(
        &self,
        payload: CreateAttendanceCorrectionRequest,
    ) -> Result<(), ApiError> {
        self.client
            .create_attendance_correction_request(payload)
            .await
            .map(|_| ())
    }

    pub async fn update_request(&self, id: &str, payload: Value) -> Result<(), ApiError> {
        self.client.update_request(id, payload).await.map(|_| ())
    }

    pub async fn cancel_request(&self, id: &str) -> Result<(), ApiError> {
        self.client.cancel_request(id).await.map(|_| ())
    }

    pub async fn update_attendance_correction(
        &self,
        id: &str,
        payload: UpdateAttendanceCorrectionRequest,
    ) -> Result<(), ApiError> {
        self.client
            .update_attendance_correction_request(id, payload)
            .await
            .map(|_| ())
    }

    pub async fn cancel_attendance_correction(&self, id: &str) -> Result<(), ApiError> {
        self.client
            .cancel_attendance_correction_request(id)
            .await
            .map(|_| ())
    }

    pub async fn list_my_requests(&self) -> Result<MyRequestsResponse, ApiError> {
        let value: Value = self.client.get_my_requests().await?;
        serde_json::from_value(value)
            .map_err(|err| ApiError::unknown(format!("Failed to parse requests response: {}", err)))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;

    fn repo(server: &MockServer) -> RequestsRepository {
        RequestsRepository::new(ApiClient::new_with_base_url(&server.url("/api")))
    }

    #[tokio::test]
    async fn requests_repository_calls_api() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(POST).path("/api/requests/leave");
            then.status(200).json_body(serde_json::json!({
                "id": "leave-1",
                "user_id": "u1",
                "leave_type": "annual",
                "start_date": "2025-01-10",
                "end_date": "2025-01-12",
                "reason": null,
                "status": "pending",
                "approved_by": null,
                "approved_at": null,
                "rejected_by": null,
                "rejected_at": null,
                "cancelled_at": null,
                "decision_comment": null,
                "created_at": "2025-01-01T00:00:00Z"
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/requests/overtime");
            then.status(200).json_body(serde_json::json!({
                "id": "ot-1",
                "user_id": "u1",
                "date": "2025-01-11",
                "planned_hours": 2.5,
                "reason": null,
                "status": "pending",
                "approved_by": null,
                "approved_at": null,
                "rejected_by": null,
                "rejected_at": null,
                "cancelled_at": null,
                "decision_comment": null,
                "created_at": "2025-01-01T00:00:00Z"
            }));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/requests/req-1");
            then.status(200)
                .json_body(serde_json::json!({ "status": "updated" }));
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/requests/req-1");
            then.status(200)
                .json_body(serde_json::json!({ "status": "cancelled" }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/requests/me");
            then.status(200).json_body(serde_json::json!({
                "items": [],
                "leave_requests": [],
                "overtime_requests": []
            }));
        });

        let repo = repo(&server);
        repo.submit_leave(CreateLeaveRequest {
            leave_type: "annual".into(),
            start_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 12).unwrap(),
            reason: None,
        })
        .await
        .unwrap();
        repo.submit_overtime(CreateOvertimeRequest {
            date: chrono::NaiveDate::from_ymd_opt(2025, 1, 11).unwrap(),
            planned_hours: 2.5,
            reason: None,
        })
        .await
        .unwrap();
        repo.update_request("req-1", serde_json::json!({ "status": "updated" }))
            .await
            .unwrap();
        repo.cancel_request("req-1").await.unwrap();
        repo.list_my_requests().await.unwrap();
    }
}
