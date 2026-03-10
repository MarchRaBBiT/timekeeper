use serde_json::json;

use super::{
    client::{encode_path_segment, ensure_device_label, ApiClient},
    types::{
        AdminSessionResponse, ApiError, LoginRequest, LoginResponse, MfaSetupResponse,
        MfaStatusResponse, SessionResponse,
    },
};

impl ApiClient {
    pub async fn login(&self, mut request: LoginRequest) -> Result<LoginResponse, ApiError> {
        if request.device_label.is_none() {
            request.device_label = Some(ensure_device_label()?);
        }
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_request(ApiClient::with_credentials(
                self.http_client()
                    .post(format!("{}/auth/login", base_url))
                    .json(&request),
            ))
            .await?;

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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }

    pub async fn refresh_token(&self) -> Result<LoginResponse, ApiError> {
        #[cfg(all(test, not(target_arch = "wasm32")))]
        if let Some(result) = self.next_refresh_override() {
            return result;
        }

        let device_label = ensure_device_label()?;

        let base_url = self.resolved_base_url().await;
        let payload = json!({
            "device_label": device_label
        });

        let response = self
            .send_request(ApiClient::with_credentials(
                self.http_client()
                    .post(format!("{}/auth/refresh", base_url))
                    .json(&payload),
            ))
            .await?;

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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionResponse>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .get(format!("{}/auth/sessions", base_url)))
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

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let encoded_session_id = encode_path_segment(session_id);
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .delete(format!("{}/auth/sessions/{}", base_url, encoded_session_id)))
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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }

    pub async fn admin_list_user_sessions(
        &self,
        user_id: &str,
    ) -> Result<Vec<AdminSessionResponse>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let encoded_user_id = encode_path_segment(user_id);
        let response = self
            .send_with_refresh(|| {
                Ok(self.http_client().get(format!(
                    "{}/admin/users/{}/sessions",
                    base_url, encoded_user_id
                )))
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

    pub async fn admin_revoke_session(&self, session_id: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let encoded_session_id = encode_path_segment(session_id);
        let response = self
            .send_with_refresh(|| {
                Ok(self.http_client().delete(format!(
                    "{}/admin/sessions/{}",
                    base_url, encoded_session_id
                )))
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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn queue_refresh_override(client: &ApiClient, result: Result<LoginResponse, ApiError>) {
    client.queue_refresh_override(result);
}
