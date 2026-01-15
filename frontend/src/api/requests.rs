use serde_json::{json, Value};

use super::{
    client::ApiClient,
    types::{
        ApiError, CreateLeaveRequest, CreateOvertimeRequest, LeaveRequestResponse,
        OvertimeRequestResponse,
    },
};

impl ApiClient {
    pub async fn admin_list_requests(
        &self,
        status: Option<&str>,
        user_id: Option<&str>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let mut url = format!("{}/admin/requests", base_url);
        let mut qp = vec![];
        if let Some(s) = status {
            qp.push(format!("status={}", s));
        }
        if let Some(u) = user_id {
            qp.push(format!("user_id={}", u));
        }
        if let Some(p) = page {
            qp.push(format!("page={}", p));
        }
        if let Some(pp) = per_page {
            qp.push(format!("per_page={}", pp));
        }
        if !qp.is_empty() {
            url.push('?');
            url.push_str(&qp.join("&"));
        }
        let response = self
            .send_with_refresh(|| Ok(self.http_client().get(&url)))
            .await?;
        self.map_json_response(response).await
    }

    #[allow(dead_code)]
    pub async fn admin_get_request_detail(&self, id: &str) -> Result<Value, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .get(format!("{}/admin/requests/{}", base_url, id)))
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
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
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
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }
}
