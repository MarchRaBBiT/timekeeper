#![allow(dead_code)]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::NaiveDate;
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

    pub(super) fn get_auth_headers(&self) -> Result<reqwest_wasm::header::HeaderMap, String> {
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

    pub(super) fn handle_unauthorized_status(status: StatusCode) {
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

    pub async fn admin_list_holidays(
        &self,
        page: i64,
        per_page: i64,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<AdminHolidayListResponse, String> {
        let headers = self.get_auth_headers()?;
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
            .client
            .get(&format!("{}/admin/holidays", base_url))
            .headers(headers)
            .query(&params)
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
}

pub(super) fn ensure_device_label(storage: &Storage) -> Result<String, String> {
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

pub(super) fn decode_jti(token: &str) -> Option<String> {
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

pub(super) fn persist_session(storage: &Storage, response: &LoginResponse) -> Result<(), String> {
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

pub(super) fn current_access_jti(storage: &Storage) -> Option<String> {
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

    #[wasm_bindgen_test]
    fn decode_jti_extracts_payload_value() {
        let token = "aaa.eyJqdGkiOiJhYmMifQ.sig";
        assert_eq!(decode_jti(token).as_deref(), Some("abc"));
    }

    #[wasm_bindgen_test]
    fn current_access_jti_decodes_and_caches() {
        cleanup(&["access_token", "access_token_jti"]);
        let store = storage();
        let token = "bbb.eyJqdGkiOiJ0ZXN0LWp0aSJ9.sig";
        store.set_item("access_token", token).unwrap();
        let resolved = current_access_jti(&store).expect("jti present");
        assert_eq!(resolved, "test-jti");
        assert_eq!(
            store.get_item("access_token_jti").unwrap().as_deref(),
            Some("test-jti")
        );
        cleanup(&["access_token", "access_token_jti"]);
    }
}
