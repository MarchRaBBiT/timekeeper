use serde_json::json;

use super::{
    client::ApiClient,
    types::{
        ApiError, CreateDataSubjectRequest, DataSubjectRequestResponse, SubjectRequestListResponse,
    },
};

impl ApiClient {
    pub async fn create_subject_request(
        &self,
        payload: CreateDataSubjectRequest,
    ) -> Result<DataSubjectRequestResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/subject-requests", base_url))
                    .json(&payload))
            })
            .await?;
        map_typed_response(response).await
    }

    pub async fn list_my_subject_requests(
        &self,
    ) -> Result<Vec<DataSubjectRequestResponse>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .get(format!("{}/subject-requests/me", base_url)))
            })
            .await?;
        map_typed_response(response).await
    }

    pub async fn cancel_subject_request(&self, id: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .delete(format!("{}/subject-requests/{}", base_url, id)))
            })
            .await?;
        map_empty_response(response).await
    }

    pub async fn admin_list_subject_requests(
        &self,
        status: Option<String>,
        request_type: Option<String>,
        user_id: Option<String>,
        from: Option<String>,
        to: Option<String>,
        page: i64,
        per_page: i64,
    ) -> Result<SubjectRequestListResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let mut params = vec![
            ("page".to_string(), page.to_string()),
            ("per_page".to_string(), per_page.to_string()),
        ];
        if let Some(value) = status {
            if !value.is_empty() {
                params.push(("status".to_string(), value));
            }
        }
        if let Some(value) = request_type {
            if !value.is_empty() {
                params.push(("type".to_string(), value));
            }
        }
        if let Some(value) = user_id {
            if !value.is_empty() {
                params.push(("user_id".to_string(), value));
            }
        }
        if let Some(value) = from {
            if !value.is_empty() {
                params.push(("from".to_string(), value));
            }
        }
        if let Some(value) = to {
            if !value.is_empty() {
                params.push(("to".to_string(), value));
            }
        }

        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .get(format!("{}/admin/subject-requests", base_url))
                    .query(&params))
            })
            .await?;
        map_typed_response(response).await
    }

    pub async fn admin_approve_subject_request(
        &self,
        id: &str,
        comment: &str,
    ) -> Result<(), ApiError> {
        self.admin_mutate_subject_request(id, "approve", comment)
            .await
    }

    pub async fn admin_reject_subject_request(
        &self,
        id: &str,
        comment: &str,
    ) -> Result<(), ApiError> {
        self.admin_mutate_subject_request(id, "reject", comment)
            .await
    }

    async fn admin_mutate_subject_request(
        &self,
        id: &str,
        action: &str,
        comment: &str,
    ) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .put(format!(
                        "{}/admin/subject-requests/{}/{}",
                        base_url, id, action
                    ))
                    .json(&json!({ "comment": comment })))
            })
            .await?;
        map_empty_response(response).await
    }
}

async fn map_typed_response<T>(response: reqwest::Response) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();
    ApiClient::handle_unauthorized_status(status);
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

async fn map_empty_response(response: reqwest::Response) -> Result<(), ApiError> {
    let status = response.status();
    ApiClient::handle_unauthorized_status(status);
    if status.is_success() {
        Ok(())
    } else {
        let error: ApiError = response
            .json()
            .await
            .map_err(ApiClient::map_error_payload_parse_failure)?;
        Err(error)
    }
}
