use crate::api::client::ApiClient;
use crate::api::types::{ApiError, AuditLog, AuditLogListResponse, PiiProtectedResponse};

fn audit_log_params(
    page: Option<i64>,
    per_page: Option<i64>,
    from: Option<String>,
    to: Option<String>,
    actor_id: Option<String>,
    event_type: Option<String>,
    result: Option<String>,
) -> Vec<(String, String)> {
    let mut params = Vec::new();
    if let Some(page) = page {
        params.push(("page".to_string(), page.to_string()));
    }
    if let Some(per_page) = per_page {
        params.push(("per_page".to_string(), per_page.to_string()));
    }
    if let Some(value) = from {
        if !value.is_empty() {
            params.push(("from".into(), value));
        }
    }
    if let Some(value) = to {
        if !value.is_empty() {
            params.push(("to".into(), value));
        }
    }
    if let Some(value) = actor_id {
        if !value.is_empty() {
            params.push(("actor_id".into(), value));
        }
    }
    if let Some(value) = event_type {
        if !value.is_empty() {
            params.push(("event_type".into(), value));
        }
    }
    if let Some(value) = result {
        if !value.is_empty() {
            params.push(("result".into(), value));
        }
    }
    params
}

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
        self.list_audit_logs_with_policy(page, per_page, from, to, actor_id, event_type, result)
            .await
            .map(|response| response.data)
    }

    pub async fn list_audit_logs_with_policy(
        &self,
        page: i64,
        per_page: i64,
        from: Option<String>,
        to: Option<String>,
        actor_id: Option<String>,
        event_type: Option<String>,
        result: Option<String>,
    ) -> Result<PiiProtectedResponse<AuditLogListResponse>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let params = audit_log_params(
            Some(page),
            Some(per_page),
            from,
            to,
            actor_id,
            event_type,
            result,
        );

        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .get(format!("{}/admin/audit-logs", base_url))
                    .query(&params))
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

    pub async fn export_audit_logs(
        &self,
        from: Option<String>,
        to: Option<String>,
        actor_id: Option<String>,
        event_type: Option<String>,
        result: Option<String>,
    ) -> Result<Vec<AuditLog>, ApiError> {
        self.export_audit_logs_with_policy(from, to, actor_id, event_type, result)
            .await
            .map(|response| response.data)
    }

    pub async fn export_audit_logs_with_policy(
        &self,
        from: Option<String>,
        to: Option<String>,
        actor_id: Option<String>,
        event_type: Option<String>,
        result: Option<String>,
    ) -> Result<PiiProtectedResponse<Vec<AuditLog>>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let params = audit_log_params(None, None, from, to, actor_id, event_type, result);

        let response = self
            .send_with_refresh(|| {
                Ok(self
                    .http_client()
                    .get(format!("{}/admin/audit-logs/export", base_url))
                    .query(&params))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_log_params_include_page_defaults() {
        let params = audit_log_params(Some(1), Some(20), None, None, None, None, None);
        assert!(params.contains(&("page".to_string(), "1".to_string())));
        assert!(params.contains(&("per_page".to_string(), "20".to_string())));
    }

    #[test]
    fn audit_log_params_skip_empty_strings() {
        let params = audit_log_params(
            None,
            None,
            Some("".into()),
            Some("2025-01-01".into()),
            Some("".into()),
            None,
            Some("success".into()),
        );
        assert!(!params.iter().any(|(k, _)| k == "from"));
        assert!(params.iter().any(|(k, v)| k == "to" && v == "2025-01-01"));
        assert!(!params.iter().any(|(k, _)| k == "actor_id"));
        assert!(params.iter().any(|(k, v)| k == "result" && v == "success"));
    }
}
