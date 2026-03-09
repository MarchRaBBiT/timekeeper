use axum::{
    body::Body,
    extract::{Extension, Path, Query, State},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderValue,
    },
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::{
    admin::application::audit_logs as application,
    error::AppError,
    models::{audit_log::AuditLog, user::User},
    state::AppState,
    utils::time,
};

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct AuditLogListQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub actor_id: Option<String>,
    pub actor_type: Option<String>,
    pub event_type: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct AuditLogExportQuery {
    pub from: String,
    pub to: String,
    pub actor_id: Option<String>,
    pub actor_type: Option<String>,
    pub event_type: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuditLogResponse {
    pub id: String,
    pub occurred_at: chrono::DateTime<Utc>,
    pub actor_id: Option<String>,
    pub actor_type: String,
    pub event_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: String,
    pub error_code: Option<String>,
    pub metadata: Option<Value>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuditLogListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<AuditLogResponse>,
}

impl From<application::AuditLogResponse> for AuditLogResponse {
    fn from(value: application::AuditLogResponse) -> Self {
        Self {
            id: value.id,
            occurred_at: value.occurred_at,
            actor_id: value.actor_id,
            actor_type: value.actor_type,
            event_type: value.event_type,
            target_type: value.target_type,
            target_id: value.target_id,
            result: value.result,
            error_code: value.error_code,
            metadata: value.metadata,
            ip: value.ip,
            user_agent: value.user_agent,
            request_id: value.request_id,
        }
    }
}

impl From<application::AuditLogListResponse> for AuditLogListResponse {
    fn from(value: application::AuditLogListResponse) -> Self {
        Self {
            page: value.page,
            per_page: value.per_page,
            total: value.total,
            items: value
                .items
                .into_iter()
                .map(AuditLogResponse::from)
                .collect(),
        }
    }
}

impl From<AuditLog> for AuditLogResponse {
    fn from(log: AuditLog) -> Self {
        application::AuditLogResponse::from(log).into()
    }
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<AuditLogListQuery>,
) -> Result<Response, AppError> {
    let result = application::list_audit_logs(
        state.read_pool(),
        &user,
        application::AuditLogListQuery {
            from: q.from,
            to: q.to,
            actor_id: q.actor_id,
            actor_type: q.actor_type,
            event_type: q.event_type,
            target_type: q.target_type,
            target_id: q.target_id,
            result: q.result,
            page: q.page,
            per_page: q.per_page,
        },
    )
    .await?;

    let mut response = Json(AuditLogListResponse::from(result.response)).into_response();
    response.headers_mut().insert(
        "X-PII-Masked",
        HeaderValue::from_static(if result.pii_masked { "true" } else { "false" }),
    );
    Ok(response)
}

pub async fn get_audit_log_detail(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let result = application::get_audit_log_detail(state.read_pool(), &user, &id).await?;

    let mut response = Json(AuditLogResponse::from(result.response)).into_response();
    response.headers_mut().insert(
        "X-PII-Masked",
        HeaderValue::from_static(if result.pii_masked { "true" } else { "false" }),
    );
    Ok(response)
}

pub async fn export_audit_logs(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<AuditLogExportQuery>,
) -> Result<Response, AppError> {
    let result = application::export_audit_logs(
        state.read_pool(),
        &user,
        application::AuditLogExportQuery {
            from: q.from,
            to: q.to,
            actor_id: q.actor_id,
            actor_type: q.actor_type,
            event_type: q.event_type,
            target_type: q.target_type,
            target_id: q.target_id,
            result: q.result,
        },
    )
    .await?;

    let filename = format!(
        "audit_logs_{}.json",
        time::now_in_timezone(&state.config.time_zone).format("%Y%m%d_%H%M%S")
    );
    let mut response = Response::new(Body::from(result.body));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(result.content_type));
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    response.headers_mut().insert(
        "X-PII-Masked",
        HeaderValue::from_static(if result.pii_masked { "true" } else { "false" }),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_log_response_maps_from_application_response() {
        let response = AuditLogResponse::from(application::AuditLogResponse {
            id: "log-1".to_string(),
            occurred_at: Utc::now(),
            actor_id: Some("user-1".to_string()),
            actor_type: "user".to_string(),
            event_type: "login".to_string(),
            target_type: None,
            target_id: None,
            result: "success".to_string(),
            error_code: None,
            metadata: None,
            ip: Some("203.0.113.10".to_string()),
            user_agent: Some("agent".to_string()),
            request_id: Some("req-1".to_string()),
        });

        assert_eq!(response.id, "log-1");
        assert_eq!(response.actor_id.as_deref(), Some("user-1"));
        assert_eq!(response.request_id.as_deref(), Some("req-1"));
    }

    #[test]
    fn audit_log_list_response_maps_items() {
        let response = AuditLogListResponse::from(application::AuditLogListResponse {
            page: 1,
            per_page: 25,
            total: 1,
            items: vec![application::AuditLogResponse {
                id: "log-1".to_string(),
                occurred_at: Utc::now(),
                actor_id: None,
                actor_type: "user".to_string(),
                event_type: "login".to_string(),
                target_type: None,
                target_id: None,
                result: "success".to_string(),
                error_code: None,
                metadata: None,
                ip: None,
                user_agent: None,
                request_id: None,
            }],
        });

        assert_eq!(response.page, 1);
        assert_eq!(response.total, 1);
        assert_eq!(response.items.len(), 1);
    }
}
