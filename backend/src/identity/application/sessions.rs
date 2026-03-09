use axum::http::{header::USER_AGENT, HeaderMap};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::{
    application::{
        clock::{Clock, SYSTEM_CLOCK},
        dto::SessionActionResponse,
    },
    db::connection::DbPool,
    error::AppError,
    middleware::request_id::RequestId,
    models::active_session::ActiveSession,
    services::{
        audit_log::{AuditLogEntry, AuditLogServiceTrait},
        token_cache::TokenCacheServiceTrait,
    },
    types::UserId,
};
use std::sync::Arc;

#[derive(Debug, Serialize, ToSchema, PartialEq, Eq)]
pub struct SessionView {
    pub id: String,
    pub device_label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

impl SessionView {
    fn from_session(session: ActiveSession, current_jti: &str) -> Self {
        let is_current = session
            .access_jti
            .as_deref()
            .map(|jti| jti == current_jti)
            .unwrap_or(false);

        Self {
            id: session.id,
            device_label: session.device_label,
            created_at: session.created_at,
            last_seen_at: session.last_seen_at,
            expires_at: session.expires_at,
            is_current,
        }
    }
}

pub async fn list_user_sessions(
    read_pool: &DbPool,
    user_id: UserId,
    current_jti: &str,
) -> Result<Vec<SessionView>, AppError> {
    let sessions =
        crate::repositories::active_session::list_active_sessions_for_user(read_pool, user_id)
            .await
            .map_err(|error| AppError::InternalServerError(error.into()))?;

    Ok(sessions
        .into_iter()
        .map(|session| SessionView::from_session(session, current_jti))
        .collect())
}

pub async fn revoke_user_session(
    write_pool: &DbPool,
    token_cache: Option<&Arc<dyn TokenCacheServiceTrait>>,
    actor_user_id: UserId,
    current_jti: &str,
    session_id: &str,
) -> Result<SessionActionResponse, AppError> {
    if session_id.trim().is_empty() {
        return Err(AppError::BadRequest("Session ID is required".into()));
    }

    let session =
        crate::repositories::active_session::find_active_session_by_id(write_pool, session_id)
            .await
            .map_err(|error| AppError::InternalServerError(error.into()))?
            .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

    if session.user_id != actor_user_id {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    if session
        .access_jti
        .as_deref()
        .map(|jti| jti == current_jti)
        .unwrap_or(false)
    {
        return Err(AppError::BadRequest(
            "Cannot revoke current session; use logout instead".into(),
        ));
    }

    revoke_session_tokens(write_pool, token_cache, &session).await?;

    Ok(SessionActionResponse::new("Session revoked", session.id))
}

pub fn record_session_audit_event(
    audit_log_service: Option<Arc<dyn AuditLogServiceTrait>>,
    actor_id: &UserId,
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
        occurred_at: SYSTEM_CLOCK.now_utc(&chrono_tz::UTC),
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

pub fn extract_ip(headers: &HeaderMap) -> Option<String> {
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

pub fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|agent| agent.trim().to_string())
        .filter(|agent| !agent.is_empty())
}

async fn revoke_session_tokens(
    pool: &DbPool,
    token_cache: Option<&Arc<dyn TokenCacheServiceTrait>>,
    session: &ActiveSession,
) -> Result<(), AppError> {
    if let Some(access_jti) = session.access_jti.as_deref() {
        crate::repositories::auth::delete_active_access_token_by_jti(pool, access_jti)
            .await
            .map_err(|error| AppError::InternalServerError(error.into()))?;
        if let Some(cache) = token_cache {
            let _ = cache.invalidate_token(access_jti).await;
        }
    }

    crate::repositories::auth::delete_refresh_token_by_id(pool, &session.refresh_token_id)
        .await
        .map_err(|error| AppError::InternalServerError(error.into()))?;

    crate::repositories::active_session::delete_active_session_by_id(pool, &session.id)
        .await
        .map_err(|error| AppError::InternalServerError(error.into()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UserId;
    use axum::http::HeaderMap;

    #[test]
    fn from_session_marks_current_session() {
        let user_id = UserId::new();
        let session = ActiveSession {
            id: "session-1".to_string(),
            user_id,
            refresh_token_id: "refresh-1".to_string(),
            access_jti: Some("current-jti".to_string()),
            device_label: Some("Firefox".to_string()),
            created_at: Utc::now(),
            last_seen_at: None,
            expires_at: Utc::now(),
        };

        let view = SessionView::from_session(session, "current-jti");

        assert!(view.is_current);
        assert_eq!(view.device_label.as_deref(), Some("Firefox"));
    }

    #[test]
    fn from_session_marks_other_session_as_not_current() {
        let session = ActiveSession {
            id: "session-2".to_string(),
            user_id: UserId::new(),
            refresh_token_id: "refresh-2".to_string(),
            access_jti: Some("other-jti".to_string()),
            device_label: None,
            created_at: Utc::now(),
            last_seen_at: None,
            expires_at: Utc::now(),
        };

        let view = SessionView::from_session(session, "current-jti");

        assert!(!view.is_current);
    }

    #[test]
    fn revoked_session_keeps_session_id() {
        let revoked = SessionActionResponse::new("Session revoked", "session-3");

        assert_eq!(revoked.session_id, "session-3");
    }

    #[test]
    fn extract_ip_prefers_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "203.0.113.195, 70.41.3.18, 150.172.238.178"
                .parse()
                .unwrap(),
        );
        headers.insert("x-real-ip", "192.168.1.100".parse().unwrap());

        assert_eq!(extract_ip(&headers), Some("203.0.113.195".to_string()));
    }

    #[test]
    fn extract_user_agent_trims_value() {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "  Mozilla/5.0  ".parse().unwrap());

        assert_eq!(
            extract_user_agent(&headers),
            Some("Mozilla/5.0".to_string())
        );
    }
}
