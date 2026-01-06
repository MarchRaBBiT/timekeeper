use serde_json::json;

use super::{
    client::{ensure_device_label, ApiClient},
    types::{ApiError, LoginRequest, LoginResponse, MfaSetupResponse, MfaStatusResponse},
};
use crate::utils::storage as storage_utils;

impl ApiClient {
    pub async fn login(&self, mut request: LoginRequest) -> Result<LoginResponse, ApiError> {
        let storage = storage_utils::local_storage().map_err(ApiError::unknown)?;
        if request.device_label.is_none() {
            request.device_label = Some(ensure_device_label(&storage)?);
        }
        let base_url = self.resolved_base_url().await;
        let response = ApiClient::with_credentials(
            self.http_client()
                .post(format!("{}/auth/login", base_url))
                .json(&request),
        )
        .send()
        .await
        .map_err(|e| ApiError::request_failed(format!("Request failed: {}", e)))?;

        if response.status().is_success() {
            let login_response: LoginResponse = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))?;
            Ok(login_response)
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    pub async fn refresh_token(&self) -> Result<LoginResponse, ApiError> {
        let storage = storage_utils::local_storage().map_err(ApiError::unknown)?;
        let device_label = ensure_device_label(&storage)?;

        let base_url = self.resolved_base_url().await;
        let payload = json!({
            "device_label": device_label
        });

        let response = ApiClient::with_credentials(
            self.http_client()
                .post(format!("{}/auth/refresh", base_url))
                .json(&payload),
        )
        .send()
        .await
        .map_err(|e| ApiError::request_failed(format!("Request failed: {}", e)))?;

        let status = response.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            let login_response: LoginResponse = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))?;
            Ok(login_response)
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    pub async fn logout(&self, all: bool) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;

        let body = if all {
            serde_json::json!({ "all": true })
        } else {
            serde_json::json!({})
        };

        let resp = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/auth/logout", base_url))
                    .json(&body))
            })
            .await?;
        let status = resp.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let err: Result<ApiError, _> = resp.json().await;
            Err(err.unwrap_or_else(|_| ApiError::unknown("Logout failed")))
        }
    }

    pub async fn get_mfa_status(&self) -> Result<MfaStatusResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| Ok(self.http_client().get(format!("{}/auth/mfa", base_url))))
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

    pub async fn register_mfa(&self) -> Result<MfaSetupResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/auth/mfa/register", base_url))
                    .json(&json!({})))
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

    pub async fn activate_mfa(&self, code: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/auth/mfa/activate", base_url))
                    .json(&json!({ "code": code })))
            })
            .await?;

        let status = response.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    pub async fn change_password(&self, current: String, new: String) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let payload = crate::api::types::ChangePasswordRequest {
            current_password: current,
            new_password: new,
        };

        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .put(format!("{}/auth/change-password", base_url))
                    .json(&payload))
            })
            .await?;

        let status = response.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }
}
