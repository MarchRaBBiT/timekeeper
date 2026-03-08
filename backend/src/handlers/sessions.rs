use axum::{
    extract::{Extension, Path, State},
    http::{header::USER_AGENT, HeaderMap},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    error::AppError,
    identity::application::sessions::{list_user_sessions, revoke_user_session, SessionView},
    middleware::request_id::RequestId,
    models::user::User,
    services::audit_log::{AuditLogEntry, AuditLogServiceTrait},
    state::AppState,
    utils::jwt::Claims,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionResponse {
    pub id: String,
    pub device_label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

impl From<SessionView> for SessionResponse {
    fn from(view: SessionView) -> Self {
        Self {
            id: view.id,
            device_label: view.device_label,
            created_at: view.created_at,
            last_seen_at: view.last_seen_at,
            expires_at: view.expires_at,
            is_current: view.is_current,
        }
    }
}

pub async fn list_sessions(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<SessionResponse>>, AppError> {
    let responses = list_user_sessions(state.read_pool(), user.id, &claims.jti)
        .await?
        .into_iter()
        .map(SessionResponse::from)
        .collect();
    Ok(Json(responses))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(claims): Extension<Claims>,
    Extension(request_id): Extension<RequestId>,
    Extension(audit_log_service): Extension<Arc<dyn AuditLogServiceTrait>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let revoked = revoke_user_session(
        &state.write_pool,
        state.token_cache.as_ref(),
        user.id,
        &claims.jti,
        &session_id,
    )
    .await?;

    record_session_audit_event(
        Some(audit_log_service),
        &user.id,
        &headers,
        &request_id,
        "session_destroy",
        Some(revoked.session_id.clone()),
        Some(json!({ "reason": "user_revoke" })),
    );

    Ok(Json(json!({
        "message": "Session revoked",
        "session_id": revoked.session_id
    })))
}

fn record_session_audit_event(
    audit_log_service: Option<Arc<dyn AuditLogServiceTrait>>,
    actor_id: &crate::types::UserId,
    headers: &HeaderMap,
    request_id: &RequestId,
    event_type: &'static str,
    session_id: Option<String>,
    metadata: Option<Value>,
) {
    let Some(audit_log_service) = audit_log_service else {
        return;
    };
    let entry = AuditLogEntry {
        occurred_at: Utc::now(),
        actor_id: Some(*actor_id),
        actor_type: "user".to_string(),
        event_type: event_type.to_string(),
        target_type: Some("session".to_string()),
        target_id: session_id,
        result: "success".to_string(),
        error_code: None,
        metadata,
        ip: extract_ip(headers),
        user_agent: extract_user_agent(headers),
        request_id: Some(request_id.0.clone()),
    };

    tokio::spawn(async move {
        if let Err(err) = audit_log_service.record_event(entry).await {
            tracing::warn!(
                error = ?err,
                event_type = %event_type,
                "Failed to record session audit log"
            );
        }
    });
}

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        return value
            .split(',')
            .next()
            .map(|ip| ip.trim().to_string())
            .filter(|ip| !ip.is_empty());
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|agent| agent.trim().to_string())
        .filter(|agent| !agent.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn extract_ip_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "203.0.113.195, 70.41.3.18, 150.172.238.178"
                .parse()
                .unwrap(),
        );

        let result = extract_ip(&headers);

        assert_eq!(result, Some("203.0.113.195".to_string()));
    }

    #[test]
    fn extract_ip_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "192.168.1.100".parse().unwrap());

        let result = extract_ip(&headers);

        assert_eq!(result, Some("192.168.1.100".to_string()));
    }

    #[test]
    fn extract_ip_prefers_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.195".parse().unwrap());
        headers.insert("x-real-ip", "192.168.1.100".parse().unwrap());

        let result = extract_ip(&headers);

        assert_eq!(result, Some("203.0.113.195".to_string()));
    }

    #[test]
    fn extract_ip_returns_none_when_missing() {
        let headers = HeaderMap::new();

        let result = extract_ip(&headers);

        assert_eq!(result, None);
    }

    #[test]
    fn extract_user_agent_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64)".parse().unwrap(),
        );

        let result = extract_user_agent(&headers);

        assert_eq!(
            result,
            Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string())
        );
    }

    #[test]
    fn extract_user_agent_returns_none_when_missing() {
        let headers = HeaderMap::new();

        let result = extract_user_agent(&headers);

        assert_eq!(result, None);
    }

    #[test]
    fn extract_user_agent_trims_whitespace() {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "  TestAgent  ".parse().unwrap());

        let result = extract_user_agent(&headers);

        assert_eq!(result, Some("TestAgent".to_string()));
    }
}
