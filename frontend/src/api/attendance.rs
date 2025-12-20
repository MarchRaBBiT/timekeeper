use chrono::NaiveDate;
use serde_json::{json, Value};

use super::{
    client::ApiClient,
    types::{
        AdminAttendanceUpsert, ApiError, AttendanceResponse, AttendanceStatusResponse,
        AttendanceSummary, BreakRecordResponse,
    },
};

impl ApiClient {
    pub async fn clock_in(&self) -> Result<AttendanceResponse, String> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .post(format!("{}/attendance/clock-in", base_url))
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

    pub async fn clock_out(&self) -> Result<AttendanceResponse, String> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .post(format!("{}/attendance/clock-out", base_url))
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

    pub async fn get_my_attendance(
        &self,
        year: Option<i32>,
        month: Option<u32>,
    ) -> Result<Vec<AttendanceResponse>, String> {
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
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self.http_client().get(&url).headers(headers))
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

    pub async fn break_start(&self, attendance_id: &str) -> Result<BreakRecordResponse, String> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .post(format!("{}/attendance/break-start", base_url))
                    .headers(headers)
                    .json(&json!({ "attendance_id": attendance_id })))
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

    pub async fn break_end(&self, break_record_id: &str) -> Result<BreakRecordResponse, String> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .post(format!("{}/attendance/break-end", base_url))
                    .headers(headers)
                    .json(&json!({ "break_record_id": break_record_id })))
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

    pub async fn get_my_attendance_range(
        &self,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<AttendanceResponse>, String> {
        let base_url = self.resolved_base_url().await;
        let mut url = format!("{}/attendance/me", base_url);
        let mut query_params = Vec::new();

        if let Some(from) = from {
            query_params.push(format!("from={}", from));
        }
        if let Some(to) = to {
            query_params.push(format!("to={}", to));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self.http_client().get(&url).headers(headers))
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

    pub async fn get_attendance_status(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<AttendanceStatusResponse, String> {
        let base_url = self.resolved_base_url().await;
        let mut url = format!("{}/attendance/status", base_url);
        if let Some(d) = date {
            url.push_str(&format!("?date={}", d));
        }
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self.http_client().get(&url).headers(headers))
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

    pub async fn get_breaks_by_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<Vec<BreakRecordResponse>, String> {
        let base_url = self.resolved_base_url().await;
        let url = format!("{}/attendance/{}/breaks", base_url, attendance_id);
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self.http_client().get(&url).headers(headers))
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

    pub async fn admin_upsert_attendance(
        &self,
        payload: AdminAttendanceUpsert,
    ) -> Result<AttendanceResponse, String> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .put(format!("{}/admin/attendance", base_url))
                    .headers(headers)
                    .json(&payload))
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

    pub async fn admin_force_end_break(
        &self,
        break_id: &str,
    ) -> Result<BreakRecordResponse, String> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self
                    .http_client()
                    .put(format!("{}/admin/breaks/{}/force-end", base_url, break_id))
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

    pub async fn get_my_summary(
        &self,
        year: Option<i32>,
        month: Option<u32>,
    ) -> Result<AttendanceSummary, String> {
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
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                Ok(self.http_client().get(&url).headers(headers))
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

    pub async fn export_my_attendance_filtered(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Value, String> {
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

        let response = self
            .send_with_refresh(|| {
                let headers = self.get_auth_headers()?;
                let mut request = self
                    .http_client()
                    .get(format!("{}/attendance/export", base_url))
                    .headers(headers);
                if !params.is_empty() {
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
