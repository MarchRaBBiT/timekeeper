use crate::api::{
    ApiClient, CreateLeaveRequest, CreateOvertimeRequest, LeaveRequestResponse,
    OvertimeRequestResponse,
};
use serde_json::Value;
use std::rc::Rc;

use super::types::MyRequestsResponse;

#[derive(Clone)]
pub struct RequestsRepository {
    client: Rc<ApiClient>,
}

impl Default for RequestsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestsRepository {
    pub fn new() -> Self {
        Self {
            client: Rc::new(ApiClient::new()),
        }
    }

    pub fn with_client(client: ApiClient) -> Self {
        Self {
            client: Rc::new(client),
        }
    }

    pub async fn submit_leave(
        &self,
        payload: CreateLeaveRequest,
    ) -> Result<LeaveRequestResponse, String> {
        self.client.create_leave_request(payload).await
    }

    pub async fn submit_overtime(
        &self,
        payload: CreateOvertimeRequest,
    ) -> Result<OvertimeRequestResponse, String> {
        self.client.create_overtime_request(payload).await
    }

    pub async fn update_request(&self, id: &str, payload: Value) -> Result<(), String> {
        self.client.update_request(id, payload).await.map(|_| ())
    }

    pub async fn cancel_request(&self, id: &str) -> Result<(), String> {
        self.client.cancel_request(id).await.map(|_| ())
    }

    pub async fn list_my_requests(&self) -> Result<MyRequestsResponse, String> {
        let value: Value = self.client.get_my_requests().await?;
        serde_json::from_value(value)
            .map_err(|err| format!("Failed to parse requests response: {}", err))
    }
}
