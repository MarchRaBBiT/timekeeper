use chrono::NaiveDate;
use serde_json::{json, Value};

use super::{
    client::ApiClient,
    types::{
        AdminAttendanceUpsert, ApiError, AttendanceResponse, AttendanceStatusResponse,
        AttendanceSummary, BreakRecordResponse,
    },
};

fn attendance_range_params(
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Vec<(&'static str, String)> {
    let mut params = Vec::new();
    if let Some(value) = from {
        params.push(("from", value.format("%Y-%m-%d").to_string()));
    }
    if let Some(value) = to {
        params.push(("to", value.format("%Y-%m-%d").to_string()));
    }
    params
}

fn attendance_status_params(date: Option<NaiveDate>) -> Vec<(&'static str, String)> {
    date.map(|value| vec![("date", value.format("%Y-%m-%d").to_string())])
        .unwrap_or_default()
}

fn attendance_summary_params(year: Option<i32>, month: Option<u32>) -> Vec<(&'static str, String)> {
    let mut params = Vec::new();
    if let Some(year) = year {
        params.push(("year", year.to_string()));
    }
    if let Some(month) = month {
        params.push(("month", month.to_string()));
    }
    params
}

impl ApiClient {
    pub async fn clock_in(&self) -> Result<AttendanceResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/attendance/clock-in", base_url))
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

    pub async fn clock_out(&self) -> Result<AttendanceResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/attendance/clock-out", base_url))
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

    pub async fn break_start(&self, attendance_id: &str) -> Result<BreakRecordResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/attendance/break-start", base_url))
                    .json(&json!({ "attendance_id": attendance_id })))
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

    pub async fn break_end(&self, break_record_id: &str) -> Result<BreakRecordResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .post(format!("{}/attendance/break-end", base_url))
                    .json(&json!({ "break_record_id": break_record_id })))
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

    pub async fn get_my_attendance_range(
        &self,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<AttendanceResponse>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let params = attendance_range_params(from, to);
        let response = self
            .send_with_refresh(|| {
                let mut request = self
                    .http_client()
                    .get(format!("{}/attendance/me", base_url));
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
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }

    pub async fn get_attendance_status(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<AttendanceStatusResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let params = attendance_status_params(date);
        let response = self
            .send_with_refresh(|| {
                let mut request = self
                    .http_client()
                    .get(format!("{}/attendance/status", base_url));
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
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }

    pub async fn get_breaks_by_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<Vec<BreakRecordResponse>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let url = format!("{}/attendance/{}/breaks", base_url, attendance_id);
        let response = self
            .send_with_refresh(|| Ok(self.http_client().get(&url)))
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

    pub async fn admin_upsert_attendance(
        &self,
        payload: AdminAttendanceUpsert,
    ) -> Result<AttendanceResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .put(format!("{}/admin/attendance", base_url))
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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }

    pub async fn admin_force_end_break(
        &self,
        break_id: &str,
    ) -> Result<BreakRecordResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .put(format!("{}/admin/breaks/{}/force-end", base_url, break_id)))
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

    pub async fn get_my_summary(
        &self,
        year: Option<i32>,
        month: Option<u32>,
    ) -> Result<AttendanceSummary, ApiError> {
        let base_url = self.resolved_base_url().await;
        let params = attendance_summary_params(year, month);
        let response = self
            .send_with_refresh(|| {
                let mut request = self
                    .http_client()
                    .get(format!("{}/attendance/me/summary", base_url));
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
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }

    pub async fn export_my_attendance_filtered(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Value, ApiError> {
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
                let mut request = self
                    .http_client()
                    .get(format!("{}/attendance/export", base_url));
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
                .map_err(|e| ApiError::unknown(format!("Failed to parse response: {}", e)))
        } else {
            let error: ApiError = response
                .json()
                .await
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn builds_attendance_range_params() {
        let from = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let to = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        let params = attendance_range_params(Some(from), Some(to));
        assert_eq!(params.len(), 2);
        assert!(params.contains(&("from", "2025-01-02".to_string())));
        assert!(params.contains(&("to", "2025-01-31".to_string())));
    }

    #[test]
    fn builds_attendance_status_params() {
        let date = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let params = attendance_status_params(Some(date));
        assert_eq!(params, vec![("date", "2025-02-01".to_string())]);
        assert!(attendance_status_params(None).is_empty());
    }

    #[test]
    fn builds_attendance_summary_params() {
        let params = attendance_summary_params(Some(2024), Some(3));
        assert!(params.contains(&("year", "2024".to_string())));
        assert!(params.contains(&("month", "3".to_string())));
        assert_eq!(attendance_summary_params(None, None).len(), 0);
    }
}
