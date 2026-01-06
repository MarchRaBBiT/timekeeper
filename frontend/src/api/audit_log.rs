use crate::api::client::ApiClient;
use crate::api::types::{ApiError, AuditLog, AuditLogListResponse};

impl ApiClient {
    pub async fn list_audit_logs(
        &self,
        page: i64,
        per_page: i64,
        from: Option<String>,
        to: Option<String>,
        actor_id: Option<String>,
        event_type: Option<String>,
        result: Option<String>,
    ) -> Result<AuditLogListResponse, ApiError> {
        let base_url = self.resolved_base_url().await;
        let mut params = vec![
            ("page".to_string(), page.to_string()),
            ("per_page".to_string(), per_page.to_string()),
        ];
        if let Some(v) = from {
            if !v.is_empty() {
                params.push(("from".into(), v));
            }
        }
        if let Some(v) = to {
            if !v.is_empty() {
                params.push(("to".into(), v));
            }
        }
        if let Some(v) = actor_id {
            if !v.is_empty() {
                params.push(("actor_id".into(), v));
            }
        }
        if let Some(v) = event_type {
            if !v.is_empty() {
                params.push(("event_type".into(), v));
            }
        }
        if let Some(v) = result {
            if !v.is_empty() {
                params.push(("result".into(), v));
            }
        }

        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .get(format!("{}/admin/audit-logs", base_url))
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

    pub async fn export_audit_logs(
        &self,
        from: Option<String>,
        to: Option<String>,
        actor_id: Option<String>,
        event_type: Option<String>,
        result: Option<String>,
    ) -> Result<Vec<AuditLog>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let mut params = Vec::new();
        if let Some(v) = from {
            if !v.is_empty() {
                params.push(("from".to_string(), v));
            }
        }
        if let Some(v) = to {
            if !v.is_empty() {
                params.push(("to".to_string(), v));
            }
        }
        if let Some(v) = actor_id {
            if !v.is_empty() {
                params.push(("actor_id".to_string(), v));
            }
        }
        if let Some(v) = event_type {
            if !v.is_empty() {
                params.push(("event_type".to_string(), v));
            }
        }
        if let Some(v) = result {
            if !v.is_empty() {
                params.push(("result".to_string(), v));
            }
        }

        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .get(format!("{}/admin/audit-logs/export", base_url))
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
}
