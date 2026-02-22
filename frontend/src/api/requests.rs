use serde_json::{json, Value};

use super::{
    client::ApiClient,
    types::{
        ApiError, CreateAttendanceCorrectionRequest, CreateLeaveRequest, CreateOvertimeRequest,
        LeaveRequestResponse, OvertimeRequestResponse, UpdateAttendanceCorrectionRequest,
    },
};

fn admin_request_params(
    status: Option<&str>,
    user_id: Option<&str>,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Vec<(&'static str, String)> {
    let mut params = Vec::new();
    if let Some(status) = status {
        params.push(("status", status.to_string()));
    }
    if let Some(user_id) = user_id {
        params.push(("user_id", user_id.to_string()));
    }
    if let Some(page) = page {
        params.push(("page", page.to_string()));
    }
    if let Some(per_page) = per_page {
        params.push(("per_page", per_page.to_string()));
    }
    params
}

impl ApiClient {
    pub async fn admin_list_requests(
        &self,
        status: Option<&str>,
        user_id: Option<&str>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let params = admin_request_params(status, user_id, page, per_page);
        let response = self
            .send_with_refresh(|| {
                let mut request = self
                    .http_client()
                    .get(format!("{}/admin/requests", base_url));
                if !params.is_empty() {
                    request = request.query(&params);
                }
                Ok(request)
            })
            .await?;
        self.map_json_response(response).await
    }

    pub async fn admin_approve_request(&self, id: &str, comment: &str) -> Result<Value, ApiError> {
        self.admin_mutate_request(id, "approve", comment).await
    }

    pub async fn admin_reject_request(&self, id: &str, comment: &str) -> Result<Value, ApiError> {
        self.admin_mutate_request(id, "reject", comment).await
    }

    async fn admin_mutate_request(
        &self,
        id: &str,
        action: &str,
        comment: &str,
    ) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .put(format!("{}/admin/requests/{}/{}", base_url, id, action))
                    .json(&json!({ "comment": comment })))
            })
            .await?;
        self.map_json_response(response).await
    }

    pub async fn update_request(&self, id: &str, payload: Value) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .put(format!("{}/requests/{}", base_url, id))
                    .json(&payload))
            })
            .await?;
        self.map_json_response(response).await
    }

    pub async fn cancel_request(&self, id: &str) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .delete(format!("{}/requests/{}", base_url, id)))
            })
            .await?;
        self.map_json_response(response).await
    }

    pub async fn create_leave_request(
        &self,
        request: CreateLeaveRequest,
    ) -> Result<LeaveRequestResponse, ApiError> {
        self.create_request("leave", request).await
    }

    pub async fn create_overtime_request(
        &self,
        request: CreateOvertimeRequest,
    ) -> Result<OvertimeRequestResponse, ApiError> {
        self.create_request("overtime", request).await
    }

    pub async fn create_attendance_correction_request(
        &self,
        request: CreateAttendanceCorrectionRequest,
    ) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/attendance-corrections", base_url))
                    .json(&request))
            })
            .await?;
        self.map_json_response(response).await
    }

    pub async fn update_attendance_correction_request(
        &self,
        id: &str,
        request: UpdateAttendanceCorrectionRequest,
    ) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .put(format!("{}/attendance-corrections/{}", base_url, id))
                    .json(&request))
            })
            .await?;
        self.map_json_response(response).await
    }

    pub async fn cancel_attendance_correction_request(&self, id: &str) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .delete(format!("{}/attendance-corrections/{}", base_url, id)))
            })
            .await?;
        self.map_json_response(response).await
    }

    async fn create_request<T, R>(&self, kind: &str, payload: T) -> Result<R, ApiError>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/requests/{}", base_url, kind))
                    .json(&payload))
            })
            .await?;

        let status = response.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }

    pub async fn get_my_requests(&self) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| Ok(self.http_client().get(format!("{}/requests/me", base_url))))
            .await?;
        self.map_json_response(response).await
    }

    async fn map_json_response(&self, response: reqwest::Response) -> Result<Value, ApiError> {
        let status = response.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_request_params_skip_missing_values() {
        let params = admin_request_params(None, None, None, None);
        assert!(params.is_empty());
    }

    #[test]
    fn admin_request_params_includes_filters() {
        let params = admin_request_params(Some("approved"), Some("user-1"), Some(2), Some(50));
        assert!(params.contains(&("status", "approved".to_string())));
        assert!(params.contains(&("user_id", "user-1".to_string())));
        assert!(params.contains(&("page", "2".to_string())));
        assert!(params.contains(&("per_page", "50".to_string())));
    }
}
