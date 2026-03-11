use crate::api::client::ApiClient;
use crate::api::types::{ApiError, AuditLog, AuditLogListResponse, PiiProtectedResponse};

#[derive(Debug, Clone, Default)]
pub struct AuditLogQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub actor_id: Option<String>,
    pub event_type: Option<String>,
    pub result: Option<String>,
}

fn audit_log_params(query: AuditLogQuery) -> Vec<(String, String)> {
    let mut params = Vec::new();
    if let Some(page) = query.page {
        params.push(("page".to_string(), page.to_string()));
    }
    if let Some(per_page) = query.per_page {
        params.push(("per_page".to_string(), per_page.to_string()));
    }
    if let Some(value) = query.from {
        if !value.is_empty() {
            params.push(("from".into(), value));
        }
    }
    if let Some(value) = query.to {
        if !value.is_empty() {
            params.push(("to".into(), value));
        }
    }
    if let Some(value) = query.actor_id {
        if !value.is_empty() {
            params.push(("actor_id".into(), value));
        }
    }
    if let Some(value) = query.event_type {
        if !value.is_empty() {
            params.push(("event_type".into(), value));
        }
    }
    if let Some(value) = query.result {
        if !value.is_empty() {
            params.push(("result".into(), value));
        }
    }
    params
}

impl ApiClient {
    pub async fn list_audit_logs_with_policy(
        &self,
        query: AuditLogQuery,
    ) -> Result<PiiProtectedResponse<AuditLogListResponse>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let params = audit_log_params(query);

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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }

    pub async fn export_audit_logs_with_policy(
        &self,
        query: AuditLogQuery,
    ) -> Result<PiiProtectedResponse<Vec<AuditLog>>, ApiError> {
        let base_url = self.resolved_base_url().await;
        let params = audit_log_params(query);

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
                .map_err(ApiClient::map_error_payload_parse_failure)?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_log_params_include_page_defaults() {
        let params = audit_log_params(AuditLogQuery {
            page: Some(1),
            per_page: Some(20),
            ..Default::default()
        });
        assert!(params.contains(&("page".to_string(), "1".to_string())));
        assert!(params.contains(&("per_page".to_string(), "20".to_string())));
    }

    #[test]
    fn audit_log_params_skip_empty_strings() {
        let params = audit_log_params(AuditLogQuery {
            to: Some("2025-01-01".into()),
            from: Some("".into()),
            actor_id: Some("".into()),
            result: Some("success".into()),
            ..Default::default()
        });
        assert!(!params.iter().any(|(k, _)| k == "from"));
        assert!(params.iter().any(|(k, v)| k == "to" && v == "2025-01-01"));
        assert!(!params.iter().any(|(k, _)| k == "actor_id"));
        assert!(params.iter().any(|(k, v)| k == "result" && v == "success"));
    }
}
