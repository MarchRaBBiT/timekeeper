#![allow(dead_code)]
use leptos::*;
use reqwest_wasm::Client;
use serde_json::json;

use crate::{api::types::*, config};

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

    fn base_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| config::resolve_api_base_url())
    }

    fn get_auth_headers(&self) -> Result<reqwest_wasm::header::HeaderMap, String> {
        let mut headers = reqwest_wasm::header::HeaderMap::new();

        // Get token from localStorage
        let window = web_sys::window().ok_or("No window object")?;
        let storage = window
            .local_storage()
            .map_err(|_| "No localStorage")?
            .ok_or("No localStorage")?;
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

    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse, String> {
        let response = self
            .client
            .post(&format!("{}/auth/login", self.base_url()))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
            let login_response: LoginResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Store tokens in localStorage
            let window = web_sys::window().ok_or("No window object")?;
            let storage = window
                .local_storage()
                .map_err(|_| "No localStorage")?
                .ok_or("No localStorage")?;
            storage
                .set_item("access_token", &login_response.access_token)
                .map_err(|_| "Failed to store token")?;
            storage
                .set_item("refresh_token", &login_response.refresh_token)
                .map_err(|_| "Failed to store refresh token")?;
            let user_json = serde_json::to_string(&login_response.user)
                .map_err(|_| "Failed to serialize user profile")?;
            storage
                .set_item("current_user", &user_json)
                .map_err(|_| "Failed to store user profile")?;

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
        let window = web_sys::window().ok_or("No window object")?;
        let storage = window
            .local_storage()
            .map_err(|_| "No localStorage")?
            .ok_or("No localStorage")?;
        let refresh_token = storage
            .get_item("refresh_token")
            .map_err(|_| "Failed to get refresh token")?
            .ok_or("No refresh token")?;

        let response = self
            .client
            .post(&format!("{}/auth/refresh", self.base_url()))
            .json(&json!({ "refresh_token": refresh_token }))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
            let login_response: LoginResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Update tokens in localStorage
            storage
                .set_item("access_token", &login_response.access_token)
                .map_err(|_| "Failed to store token")?;
            storage
                .set_item("refresh_token", &login_response.refresh_token)
                .map_err(|_| "Failed to store refresh token")?;
            let user_json = serde_json::to_string(&login_response.user)
                .map_err(|_| "Failed to serialize user profile")?;
            storage
                .set_item("current_user", &user_json)
                .map_err(|_| "Failed to store user profile")?;

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

        // Read refresh token id if present
        let window = web_sys::window()
            .ok_or("No window object")
            .map_err(|e| e.to_string())?;
        let storage = window
            .local_storage()
            .map_err(|_| "No localStorage")?
            .ok_or("No localStorage")?;
        let refresh = storage.get_item("refresh_token").ok().flatten();

        let body = if all {
            serde_json::json!({ "all": true })
        } else if let Some(rt) = refresh {
            serde_json::json!({ "refresh_token": rt })
        } else {
            serde_json::json!({})
        };

        let resp = self
            .client
            .post(&format!("{}/auth/logout", self.base_url()))
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if resp.status().is_success() {
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
        let response = self
            .client
            .get(&format!("{}/auth/mfa", self.base_url()))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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
        let response = self
            .client
            .post(&format!("{}/auth/mfa/register", self.base_url()))
            .headers(headers)
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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
        let response = self
            .client
            .post(&format!("{}/auth/mfa/activate", self.base_url()))
            .headers(headers)
            .json(&MfaCodeRequest {
                code: code.to_string(),
            })
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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

        let response = self
            .client
            .post(&format!("{}/attendance/clock-in", self.base_url()))
            .headers(headers)
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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

        let response = self
            .client
            .post(&format!("{}/attendance/clock-out", self.base_url()))
            .headers(headers)
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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

        let mut url = format!("{}/attendance/me", self.base_url());
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

        if response.status().is_success() {
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
        let response = self
            .client
            .post(&format!("{}/attendance/break-start", self.base_url()))
            .headers(headers)
            .json(&json!({"attendance_id": attendance_id}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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
        let response = self
            .client
            .post(&format!("{}/attendance/break-end", self.base_url()))
            .headers(headers)
            .json(&json!({"break_record_id": break_record_id}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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
        let mut url = format!("{}/attendance/me", self.base_url());
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
        if response.status().is_success() {
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
        let mut url = format!("{}/attendance/status", self.base_url());
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
        if response.status().is_success() {
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
        let url = format!("{}/attendance/{}/breaks", self.base_url(), attendance_id);
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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
        let response = self
            .client
            .put(&format!("{}/admin/attendance", self.base_url()))
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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
        let response = self
            .client
            .put(&format!(
                "{}/admin/breaks/{}/force-end",
                self.base_url(),
                break_id
            ))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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
        let mut url = format!("{}/admin/requests", self.base_url());
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
        if response.status().is_success() {
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
        let response = self
            .client
            .get(&format!("{}/admin/requests/{}", self.base_url(), id))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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
        let response = self
            .client
            .put(&format!(
                "{}/admin/requests/{}/approve",
                self.base_url(),
                id
            ))
            .headers(headers)
            .json(&json!({"comment": comment}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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
        let response = self
            .client
            .put(&format!("{}/admin/requests/{}/reject", self.base_url(), id))
            .headers(headers)
            .json(&json!({"comment": comment}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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
        let response = self
            .client
            .put(&format!("{}/requests/{}", self.base_url(), id))
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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
        let response = self
            .client
            .delete(&format!("{}/requests/{}", self.base_url(), id))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
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

        let mut url = format!("{}/attendance/me/summary", self.base_url());
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

        if response.status().is_success() {
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

        let response = self
            .client
            .post(&format!("{}/requests/leave", self.base_url()))
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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

        let response = self
            .client
            .post(&format!("{}/requests/overtime", self.base_url()))
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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

        let response = self
            .client
            .get(&format!("{}/requests/me", self.base_url()))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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

        let response = self
            .client
            .get(&format!("{}/admin/users", self.base_url()))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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

        let response = self
            .client
            .post(&format!("{}/admin/users", self.base_url()))
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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

        let response = self
            .client
            .get(&format!("{}/admin/export", self.base_url()))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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
            .get(&format!("{}/admin/export", self.base_url()))
            .headers(headers);
        if !params.is_empty() {
            request = request.query(&params);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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
            .get(&format!("{}/attendance/export", self.base_url()))
            .headers(headers);
        if !params.is_empty() {
            request = request.query(&params);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
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
