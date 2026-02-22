use crate::api::types::*;
use crate::config;
use chrono::NaiveDate;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use reqwest::{Client, StatusCode};
use serde_json::json;
use uuid::Uuid;
use web_sys::Storage;

#[cfg(all(test, not(target_arch = "wasm32")))]
use crate::utils::storage as storage_utils;
#[cfg(all(test, target_arch = "wasm32"))]
use crate::utils::storage as storage_utils;
#[cfg(all(test, not(target_arch = "wasm32")))]
use std::collections::HashMap;
#[cfg(all(test, not(target_arch = "wasm32")))]
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: Option<String>,
    #[cfg(all(test, not(target_arch = "wasm32")))]
    responder: Option<Arc<dyn TestResponder>>,
    #[cfg(all(test, not(target_arch = "wasm32")))]
    refresh_overrides: Arc<Mutex<Vec<Result<LoginResponse, ApiError>>>>,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[derive(Clone)]
pub(crate) struct MockResponse {
    status: StatusCode,
    body: serde_json::Value,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
impl MockResponse {
    pub(crate) fn json(status: u16, body: serde_json::Value) -> Self {
        Self {
            status: StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            body,
        }
    }

    fn into_response(self) -> reqwest::Response {
        let body = serde_json::to_vec(&self.body).unwrap_or_default();
        let response = http::Response::builder()
            .status(self.status)
            .header("content-type", "application/json")
            .body(reqwest::Body::from(body))
            .unwrap();
        response.into()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) trait TestResponder: Send + Sync {
    fn respond(&self, request: &reqwest::Request) -> Result<MockResponse, ApiError>;
}

#[cfg(all(test, not(target_arch = "wasm32")))]
static MOCK_REGISTRY: OnceLock<Mutex<HashMap<String, Arc<dyn TestResponder>>>> = OnceLock::new();

const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

pub(super) fn encode_path_segment(value: &str) -> String {
    utf8_percent_encode(value, PATH_SEGMENT_ENCODE_SET).to_string()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn register_mock(base_url: String, responder: Arc<dyn TestResponder>) {
    let registry = MOCK_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = registry.lock() {
        guard.insert(base_url, responder);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
fn lookup_mock(base_url: &str) -> Option<Arc<dyn TestResponder>> {
    MOCK_REGISTRY
        .get()
        .and_then(|registry| registry.lock().ok())
        .and_then(|guard| guard.get(base_url).cloned())
}

impl ApiClient {
    fn parse_pii_masked_header(headers: &reqwest::header::HeaderMap) -> bool {
        headers
            .get("X-PII-Masked")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: None,
            #[cfg(all(test, not(target_arch = "wasm32")))]
            responder: None,
            #[cfg(all(test, not(target_arch = "wasm32")))]
            refresh_overrides: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn new_with_base_url(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: Some(base_url.to_string()),
            #[cfg(all(test, not(target_arch = "wasm32")))]
            responder: lookup_mock(base_url),
            #[cfg(all(test, not(target_arch = "wasm32")))]
            refresh_overrides: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn queue_refresh_override(&self, result: Result<LoginResponse, ApiError>) {
        if let Ok(mut stack) = self.refresh_overrides.lock() {
            stack.push(result);
        }
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn next_refresh_override(&self) -> Option<Result<LoginResponse, ApiError>> {
        self.refresh_overrides
            .lock()
            .ok()
            .and_then(|mut stack| stack.pop())
    }

    pub(super) async fn resolved_base_url(&self) -> String {
        if let Some(base) = &self.base_url {
            base.clone()
        } else {
            config::await_api_base_url().await
        }
    }

    pub(super) fn http_client(&self) -> &Client {
        &self.client
    }

    pub(super) async fn send_request(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, ApiError> {
        #[cfg(all(test, not(target_arch = "wasm32")))]
        if let Some(responder) = &self.responder {
            let request = builder
                .build()
                .map_err(|e| ApiError::unknown(format!("Failed to build request: {}", e)))?;
            let mock = responder.respond(&request)?;
            return Ok(mock.into_response());
        }

        builder
            .send()
            .await
            .map_err(|e| ApiError::request_failed(format!("Request failed: {}", e)))
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn with_credentials(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.fetch_credentials_include()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn with_credentials(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
    }

    pub(super) fn handle_unauthorized_status(status: StatusCode) {
        if status == StatusCode::UNAUTHORIZED {
            Self::redirect_to_login_if_needed();
        }
    }

    pub(super) async fn send_with_refresh<F>(
        &self,
        build_request: F,
    ) -> Result<reqwest::Response, ApiError>
    where
        F: Fn() -> Result<reqwest::RequestBuilder, ApiError>,
    {
        let response = self
            .send_request(Self::with_credentials(build_request()?))
            .await?;

        if response.status() != StatusCode::UNAUTHORIZED {
            return Ok(response);
        }

        if self.refresh_token().await.is_ok() {
            let retry_response = self
                .send_request(Self::with_credentials(build_request()?))
                .await?;
            return Ok(retry_response);
        }

        Ok(response)
    }

    fn redirect_to_login_if_needed() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return;
        }

        #[cfg(target_arch = "wasm32")]
        if let Some(window) = web_sys::window() {
            let location = window.location();
            if let Ok(pathname) = location.pathname() {
                // Public paths where 401 is expected (not logged in) and should not trigger redirect
                let public_paths = [
                    "/",
                    "/login",
                    "/forgot-password",
                    "/reset-password",
                    "/mfa/register",
                ];
                if public_paths.contains(&pathname.as_str()) {
                    return;
                }
            }
            let _ = location.set_href("/login");
        }
    }

    pub async fn get_me(&self) -> Result<UserResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| Ok(self.client.get(format!("{}/auth/me", base_url))))
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

    pub async fn get_users(&self) -> Result<Vec<UserResponse>, ApiError> {
        self.get_users_with_policy()
            .await
            .map(|response| response.data)
    }

    pub async fn get_users_with_policy(
        &self,
    ) -> Result<PiiProtectedResponse<Vec<UserResponse>>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| Ok(self.client.get(format!("{}/admin/users", base_url))))
            .await?;

        let status = response.status();
        let pii_masked = Self::parse_pii_masked_header(response.headers());
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            let data = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))?;
            Ok(PiiProtectedResponse { data, pii_masked })
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    pub async fn create_user(&self, request: CreateUser) -> Result<UserResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .post(format!("{}/admin/users", base_url))
                    .json(&request))
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

    pub async fn admin_reset_mfa(&self, user_id: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let resp = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .post(format!("{}/admin/mfa/reset", base_url))
                    .json(&json!({ "user_id": user_id })))
            })
            .await?;
        let status = resp.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let error: ApiError = resp
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    /// Delete a user (soft delete by default, hard delete if `hard` is true).
    pub async fn admin_delete_user(&self, user_id: &str, hard: bool) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let encoded_user_id = encode_path_segment(user_id);
        let resp = self
            .send_with_refresh(|| {
                let mut request = self
                    .client
                    .delete(format!("{}/admin/users/{}", base_url, encoded_user_id));
                if hard {
                    request = request.query(&[("hard", "true")]);
                }
                Ok(request)
            })
            .await?;
        let status = resp.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let error: ApiError = resp
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    pub async fn admin_unlock_user(&self, user_id: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let encoded_user_id = encode_path_segment(user_id);
        let resp = self
            .send_with_refresh(|| {
                Ok(self.client.post(format!(
                    "{}/admin/users/{}/unlock",
                    base_url, encoded_user_id
                )))
            })
            .await?;
        let status = resp.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let error: ApiError = resp
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    /// Get all archived users.
    pub async fn admin_get_archived_users(&self) -> Result<Vec<ArchivedUserResponse>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let resp = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .get(format!("{}/admin/archived-users", base_url)))
            })
            .await?;
        let status = resp.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            resp.json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))
        } else {
            let error: ApiError = resp
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    /// Restore an archived user.
    pub async fn admin_restore_archived_user(&self, user_id: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let encoded_user_id = encode_path_segment(user_id);
        let resp = self
            .send_with_refresh(|| {
                Ok(self.client.post(format!(
                    "{}/admin/archived-users/{}/restore",
                    base_url, encoded_user_id
                )))
            })
            .await?;
        let status = resp.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let error: ApiError = resp
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    /// Permanently delete an archived user.
    pub async fn admin_delete_archived_user(&self, user_id: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let encoded_user_id = encode_path_segment(user_id);
        let resp = self
            .send_with_refresh(|| {
                Ok(self.client.delete(format!(
                    "{}/admin/archived-users/{}",
                    base_url, encoded_user_id
                )))
            })
            .await?;
        let status = resp.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let error: ApiError = resp
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    pub async fn admin_list_holidays(
        &self,
        page: i64,
        per_page: i64,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<AdminHolidayListResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let mut params = vec![
            ("type".to_string(), "public".to_string()),
            ("page".to_string(), page.to_string()),
            ("per_page".to_string(), per_page.to_string()),
        ];
        if let Some(from) = from {
            params.push(("from".into(), from.format("%Y-%m-%d").to_string()));
        }
        if let Some(to) = to {
            params.push(("to".into(), to.format("%Y-%m-%d").to_string()));
        }
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .get(format!("{}/admin/holidays", base_url))
                    .query(&params))
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

    pub async fn admin_create_holiday(
        &self,
        payload: &CreateHolidayRequest,
    ) -> Result<HolidayResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .post(format!("{}/admin/holidays", base_url))
                    .json(payload))
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

    pub async fn admin_delete_holiday(&self, id: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let encoded_id = encode_path_segment(id);
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .delete(format!("{}/admin/holidays/{}", base_url, encoded_id)))
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

    pub async fn check_holiday(
        &self,
        date: chrono::NaiveDate,
    ) -> Result<HolidayCheckResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .get(format!("{}/holidays/check", base_url))
                    .query(&[("date", date.format("%Y-%m-%d").to_string())]))
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

    pub async fn get_monthly_holidays(
        &self,
        year: i32,
        month: u32,
    ) -> Result<Vec<HolidayCalendarEntry>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .get(format!("{}/holidays/month", base_url))
                    .query(&[("year", year.to_string()), ("month", month.to_string())]))
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

    pub async fn admin_list_weekly_holidays(&self) -> Result<Vec<WeeklyHolidayResponse>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .get(format!("{}/admin/holidays/weekly", base_url)))
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

    pub async fn admin_create_weekly_holiday(
        &self,
        payload: &CreateWeeklyHolidayRequest,
    ) -> Result<WeeklyHolidayResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .post(format!("{}/admin/holidays/weekly", base_url))
                    .json(payload))
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

    pub async fn admin_delete_weekly_holiday(&self, id: &str) -> Result<(), ApiError> {
        let base_url = self.resolved_base_url().await;
        let encoded_id = encode_path_segment(id);
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .client
                    .delete(format!("{}/admin/holidays/weekly/{}", base_url, encoded_id)))
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

    pub async fn admin_fetch_google_holidays(
        &self,
        year: Option<i32>,
    ) -> Result<Vec<CreateHolidayRequest>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let mut request = self
                    .client
                    .get(format!("{}/admin/holidays/google", base_url));
                if let Some(year) = year {
                    let params = [("year", year.to_string())];
                    request = request.query(&params);
                }
                Ok(request)
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

    pub async fn export_data_filtered(
        &self,
        username: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<serde_json::Value, ApiError> {
        self.export_data_filtered_with_policy(username, from, to)
            .await
            .map(|response| response.data)
    }

    pub async fn export_data_filtered_with_policy(
        &self,
        username: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<PiiProtectedResponse<serde_json::Value>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(u) = username {
            if !u.is_empty() {
                params.push(("username", u.to_string()));
            }
        }
        if let Some(f) = from {
            if !f.is_empty() {
                params.push(("from", f.to_string()));
            }
        }
        if let Some(t) = to {
            if !t.is_empty() {
                params.push(("to", t.to_string()));
            }
        }

        let response = self
            .send_with_refresh(|| {
                let mut request = self.client.get(format!("{}/admin/export", base_url));
                if !params.is_empty() {
                    request = request.query(&params);
                }
                Ok(request)
            })
            .await?;

        let status = response.status();
        let pii_masked = Self::parse_pii_masked_header(response.headers());
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            let data = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))?;
            Ok(PiiProtectedResponse { data, pii_masked })
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(|e| ApiError::unknown(format!("Failed to parse error: {}", e)))?;
            Err(error)
        }
    }

    pub async fn request_password_reset(&self, email: String) -> Result<MessageResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let request = RequestPasswordResetRequest { email };
        let response = self
            .send_request(
                self.client
                    .post(format!("{}/auth/request-password-reset", base_url))
                    .json(&request),
            )
            .await?;

        let status = response.status();
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

    pub async fn reset_password(
        &self,
        token: String,
        new_password: String,
    ) -> Result<MessageResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let request = ResetPasswordRequest {
            token,
            new_password,
        };
        let response = self
            .send_request(
                self.client
                    .post(format!("{}/auth/reset-password", base_url))
                    .json(&request),
            )
            .await?;

        let status = response.status();
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

pub(super) fn ensure_device_label(storage: &Storage) -> Result<String, ApiError> {
    if let Ok(Some(label)) = storage.get_item("device_label") {
        if !label.trim().is_empty() {
            return Ok(label);
        }
    }
    let label = format!("device-{}", Uuid::new_v4());
    storage
        .set_item("device_label", &label)
        .map_err(|_| ApiError::unknown("Failed to persist device label"))?;
    Ok(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn storage() -> Storage {
        storage_utils::local_storage().expect("local_storage available")
    }

    fn cleanup(keys: &[&str]) {
        let store = storage();
        for key in keys {
            let _ = store.remove_item(key);
        }
    }

    #[wasm_bindgen_test]
    fn ensure_device_label_persists_value() {
        cleanup(&["device_label"]);
        let store = storage();
        let first = ensure_device_label(&store).expect("label generated");
        assert!(first.starts_with("device-"));
        assert_eq!(store.get_item("device_label").unwrap().unwrap(), first);
        let second = ensure_device_label(&store).expect("label reused");
        assert_eq!(first, second);
        cleanup(&["device_label"]);
    }
}
