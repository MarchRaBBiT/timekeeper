#![allow(dead_code)]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use leptos::*;
use reqwest_wasm::{Client, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;
use web_sys::Storage;

use crate::{api::types::*, config, utils::storage as storage_utils};

pub struct ApiClient {
    client: Client,
    base_url: Option<String>,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: None,
        }
    }

    pub fn new_with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: Some(base_url.into()),
        }
    }

    async fn resolved_base_url(&self) -> String {
        if let Some(base) = &self.base_url {
            base.clone()
        } else {
            config::await_api_base_url().await
        }
    }

    fn get_auth_headers(&self) -> Result<reqwest_wasm::header::HeaderMap, String> {
        let mut headers = reqwest_wasm::header::HeaderMap::new();

        let storage = storage_utils::local_storage()?;
        let token = storage
            .get_item("access_token")
            .map_err(|_| "Failed to get token")?
            .ok_or("No token")?;

        headers.insert(
            reqwest_wasm::header::AUTHORIZATION,
            format!("Bearer {}", token)
                .parse()
                .map_err(|_| "Invalid token format")?,
        );

        Ok(headers)
    }

    fn handle_unauthorized_status(status: StatusCode) {
        if status == StatusCode::UNAUTHORIZED {
            Self::clear_auth_session();
            Self::redirect_to_login_if_needed();
        }
    }

    fn clear_auth_session() {
        if let Ok(storage) = storage_utils::local_storage() {
            let _ = storage.remove_item("access_token");
            let _ = storage.remove_item("access_token_jti");
            let _ = storage.remove_item("refresh_token");
            let _ = storage.remove_item("current_user");
        }
    }

    fn redirect_to_login_if_needed() {
        if let Some(window) = web_sys::window() {
            let location = window.location();
            if let Ok(pathname) = location.pathname() {
                if pathname == "/login" {
                    return;
                }
            }
            let _ = location.set_href("/login");
        }
    }

    pub async fn login(&self, mut request: LoginRequest) -> Result<LoginResponse, String> {
        let storage = storage_utils::local_storage()?;
        if request.device_label.is_none() {
            request.device_label = Some(ensure_device_label(&storage)?);
        }
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
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
            if let Value::Object(ref mut map) = payload {
                map.insert("previous_jti".into(), json!(jti));
            }
        }
        let response = self
            .client
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
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;

        // Read refresh token if present
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
            .client
            .post(&format!("{}/auth/logout", base_url))
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/auth/mfa", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/auth/mfa/register", base_url))
            .headers(headers)
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/auth/mfa/activate", base_url))
            .headers(headers)
            .json(&MfaCodeRequest {
                code: code.to_string(),
            })
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn clock_in(&self) -> Result<AttendanceResponse, String> {
        let headers = self.get_auth_headers()?;

        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/attendance/clock-in", base_url))
            .headers(headers)
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn clock_out(&self) -> Result<AttendanceResponse, String> {
        let headers = self.get_auth_headers()?;

        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/attendance/clock-out", base_url))
            .headers(headers)
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn get_my_attendance(
        &self,
        year: Option<i32>,
        month: Option<u32>,
    ) -> Result<Vec<AttendanceResponse>, String> {
        let headers = self.get_auth_headers()?;

        let base_url = self.resolved_base_url().await;
        let mut url = format!("{}/attendance/me", base_url);
        let mut query_params = Vec::new();

        if let Some(year) = year {
            query_params.push(format!("year={}", year));
        }
        if let Some(month) = month {
            query_params.push(format!("month={}", month));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn break_start(&self, attendance_id: &str) -> Result<BreakRecordResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/attendance/break-start", base_url))
            .headers(headers)
            .json(&json!({"attendance_id": attendance_id}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn break_end(&self, break_record_id: &str) -> Result<BreakRecordResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/attendance/break-end", base_url))
            .headers(headers)
            .json(&json!({"break_record_id": break_record_id}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn get_my_attendance_range(
        &self,
        from: Option<chrono::NaiveDate>,
        to: Option<chrono::NaiveDate>,
    ) -> Result<Vec<AttendanceResponse>, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let mut url = format!("{}/attendance/me", base_url);
        let mut query_params = Vec::new();
        if let Some(f) = from {
            query_params.push(format!("from={}", f));
        }
        if let Some(t) = to {
            query_params.push(format!("to={}", t));
        }
        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn get_attendance_status(
        &self,
        date: Option<chrono::NaiveDate>,
    ) -> Result<AttendanceStatusResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let mut url = format!("{}/attendance/status", base_url);
        if let Some(d) = date {
            url.push_str(&format!("?date={}", d));
        }
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn get_breaks_by_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<Vec<BreakRecordResponse>, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let url = format!("{}/attendance/{}/breaks", base_url, attendance_id);
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn admin_upsert_attendance(
        &self,
        payload: AdminAttendanceUpsert,
    ) -> Result<AttendanceResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .put(&format!("{}/admin/attendance", base_url))
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn admin_force_end_break(
        &self,
        break_id: &str,
    ) -> Result<BreakRecordResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .put(&format!("{}/admin/breaks/{}/force-end", base_url, break_id))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn admin_list_requests(
        &self,
        status: Option<&str>,
        user_id: Option<&str>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
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
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn admin_get_request_detail(&self, id: &str) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/admin/requests/{}", base_url, id))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn admin_approve_request(
        &self,
        id: &str,
        comment: &str,
    ) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .put(&format!("{}/admin/requests/{}/approve", base_url, id))
            .headers(headers)
            .json(&json!({"comment": comment}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn admin_reject_request(
        &self,
        id: &str,
        comment: &str,
    ) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .put(&format!("{}/admin/requests/{}/reject", base_url, id))
            .headers(headers)
            .json(&json!({"comment": comment}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn update_request(
        &self,
        id: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .put(&format!("{}/requests/{}", base_url, id))
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn cancel_request(&self, id: &str) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .delete(&format!("{}/requests/{}", base_url, id))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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

    pub async fn get_my_summary(
        &self,
        year: Option<i32>,
        month: Option<u32>,
    ) -> Result<AttendanceSummary, String> {
        let headers = self.get_auth_headers()?;

        let base_url = self.resolved_base_url().await;
        let mut url = format!("{}/attendance/me/summary", base_url);
        let mut query_params = Vec::new();

        if let Some(year) = year {
            query_params.push(format!("year={}", year));
        }
        if let Some(month) = month {
            query_params.push(format!("month={}", month));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn create_leave_request(
        &self,
        request: CreateLeaveRequest,
    ) -> Result<LeaveRequestResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/requests/leave", base_url))
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn create_overtime_request(
        &self,
        request: CreateOvertimeRequest,
    ) -> Result<OvertimeRequestResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/requests/overtime", base_url))
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn get_my_requests(&self) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/requests/me", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn get_users(&self) -> Result<Vec<UserResponse>, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/admin/users", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn create_user(&self, request: CreateUser) -> Result<UserResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/admin/users", base_url))
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn admin_reset_mfa(&self, user_id: &str) -> Result<(), String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let resp = self
            .client
            .post(&format!("{}/admin/mfa/reset", base_url))
            .headers(headers)
            .json(&json!({ "user_id": user_id }))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        let status = resp.status();
        Self::handle_unauthorized_status(status);
        if status.is_success() {
            Ok(())
        } else {
            let error: ApiError = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse error: {}", e))?;
            Err(error.error)
        }
    }

    pub async fn get_public_holidays(&self) -> Result<Vec<HolidayResponse>, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/holidays", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn admin_list_holidays(&self) -> Result<Vec<HolidayResponse>, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/admin/holidays", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn admin_create_holiday(
        &self,
        payload: &CreateHolidayRequest,
    ) -> Result<HolidayResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/admin/holidays", base_url))
            .headers(headers)
            .json(payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn admin_delete_holiday(&self, id: &str) -> Result<(), String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .delete(&format!("{}/admin/holidays/{}", base_url, id))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn check_holiday(
        &self,
        date: chrono::NaiveDate,
    ) -> Result<HolidayCheckResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/holidays/check", base_url))
            .query(&[("date", date.format("%Y-%m-%d").to_string())])
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn get_monthly_holidays(
        &self,
        year: i32,
        month: u32,
    ) -> Result<Vec<HolidayCalendarEntry>, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/holidays/month", base_url))
            .query(&[("year", year.to_string()), ("month", month.to_string())])
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn admin_list_weekly_holidays(&self) -> Result<Vec<WeeklyHolidayResponse>, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/admin/holidays/weekly", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn admin_create_weekly_holiday(
        &self,
        payload: &CreateWeeklyHolidayRequest,
    ) -> Result<WeeklyHolidayResponse, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .post(&format!("{}/admin/holidays/weekly", base_url))
            .headers(headers)
            .json(payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn admin_fetch_google_holidays(
        &self,
        year: Option<i32>,
    ) -> Result<Vec<CreateHolidayRequest>, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let mut url = format!("{}/admin/holidays/google", base_url);
        if let Some(year) = year {
            url.push_str(&format!("?year={}", year));
        }
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn export_data(&self) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let response = self
            .client
            .get(&format!("{}/admin/export", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn export_data_filtered(
        &self,
        username: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
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

        let mut request = self
            .client
            .get(&format!("{}/admin/export", base_url))
            .headers(headers);
        if !params.is_empty() {
            request = request.query(&params);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

    pub async fn export_my_attendance_filtered(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let headers = self.get_auth_headers()?;
        let base_url = self.resolved_base_url().await;
        let mut params: Vec<(&str, String)> = Vec::new();
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

        let mut request = self
            .client
            .get(&format!("{}/attendance/export", base_url))
            .headers(headers);
        if !params.is_empty() {
            request = request.query(&params);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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
}

fn ensure_device_label(storage: &Storage) -> Result<String, String> {
    if let Ok(Some(label)) = storage.get_item("device_label") {
        if !label.trim().is_empty() {
            return Ok(label);
        }
    }
    let label = format!("device-{}", Uuid::new_v4());
    storage
        .set_item("device_label", &label)
        .map_err(|_| "Failed to persist device label")?;
    Ok(label)
}

fn decode_jti(token: &str) -> Option<String> {
    let mut parts = token.split('.');
    parts.next()?;
    let payload = parts.next()?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let value: Value = serde_json::from_slice(&decoded).ok()?;
    value
        .get("jti")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn persist_session(storage: &Storage, response: &LoginResponse) -> Result<(), String> {
    storage
        .set_item("access_token", &response.access_token)
        .map_err(|_| "Failed to store token")?;
    storage
        .set_item("refresh_token", &response.refresh_token)
        .map_err(|_| "Failed to store refresh token")?;
    let user_json =
        serde_json::to_string(&response.user).map_err(|_| "Failed to serialize user profile")?;
    storage
        .set_item("current_user", &user_json)
        .map_err(|_| "Failed to store user profile")?;
    if let Some(jti) = decode_jti(&response.access_token) {
        let _ = storage.set_item("access_token_jti", &jti);
    } else {
        let _ = storage.remove_item("access_token_jti");
    }
    Ok(())
}

fn current_access_jti(storage: &Storage) -> Option<String> {
    if let Ok(Some(jti)) = storage.get_item("access_token_jti") {
        return Some(jti);
    }
    if let Ok(Some(token)) = storage.get_item("access_token") {
        if let Some(jti) = decode_jti(&token) {
            let _ = storage.set_item("access_token_jti", &jti);
            return Some(jti);
        }
    }
    None
}
