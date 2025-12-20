use serde_json::json;

use super::{
    client::{current_access_jti, ensure_device_label, persist_session, ApiClient},
    types::{ApiError, LoginRequest, LoginResponse, MfaSetupResponse, MfaStatusResponse},
};
use crate::utils::storage as storage_utils;

impl ApiClient {
    pub async fn login(&self, mut request: LoginRequest) -> Result<LoginResponse, String> {
        let storage = storage_utils::local_storage()?;
        if request.device_label.is_none() {
            request.device_label = Some(ensure_device_label(&storage)?);
        }
        let base_url = self.resolved_base_url().await;
        let response = self
            .http_client()
            .post(&format!("{}/auth/login", base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
            let login_response: LoginResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            persist_session(&storage, &login_response)?;
            Ok(login_response)
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse error: {}", e))?;
            Err(error.error)
        }
    }

    pub async fn refresh_token(&self) -> Result<LoginResponse, String> {
        let storage = storage_utils::local_storage()?;
        let refresh_token = storage
            .get_item("refresh_token")
            .map_err(|_| "Failed to get refresh token")?
            .ok_or("No refresh token")?;
        let device_label = ensure_device_label(&storage)?;
        let previous_jti = current_access_jti(&storage);

        let base_url = self.resolved_base_url().await;
        let mut payload = json!({
            "refresh_token": refresh_token,
            "device_label": device_label
        });
        if let Some(jti) = previous_jti {
            if let serde_json::Value::Object(ref mut map) = payload {
                map.insert("previous_jti".into(), json!(jti));
            }
        }

        let response = self
            .http_client()
            .post(&format!("{}/auth/refresh", base_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            let login_response: LoginResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            persist_session(&storage, &login_response)?;
            Ok(login_response)
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse error: {}", e))?;
            Err(error.error)
        }
    }

    pub async fn logout(&self, all: bool) -> Result<(), String> {
        let base_url = self.resolved_base_url().await;

        let refresh = storage_utils::local_storage()
            .ok()
            .and_then(|s| s.get_item("refresh_token").ok().flatten());

        let body = if all {
            serde_json::json!({ "all": true })
        } else if let Some(rt) = refresh {
            serde_json::json!({ "refresh_token": rt })
        } else {
            serde_json::json!({})
        };

        let resp = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .post(&format!("{}/auth/logout", base_url))
                    .headers(headers)
                    .json(&body))
            })
            .await?;
        let status = resp.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let err: Result<ApiError, _> = resp.json().await;
            Err(err
                .map(|e| e.error)
                .unwrap_or_else(|_| "Logout failed".into()))
        }
    }

    pub async fn get_mfa_status(&self) -> Result<MfaStatusResponse, String> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .get(&format!("{}/auth/mfa", base_url))
                    .headers(headers))
            })
            .await?;

        let status = response.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse error: {}", e))?;
            Err(error.error)
        }
    }

    pub async fn register_mfa(&self) -> Result<MfaSetupResponse, String> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .post(&format!("{}/auth/mfa/register", base_url))
                    .headers(headers)
                    .json(&json!({})))
            })
            .await?;

        let status = response.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse error: {}", e))?;
            Err(error.error)
        }
    }

    pub async fn activate_mfa(&self, code: &str) -> Result<(), String> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .post(&format!("{}/auth/mfa/activate", base_url))
                    .headers(headers)
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
                .map_err(|e| format!("Failed to parse error: {}", e))?;
            Err(error.error)
        }
    }
}
