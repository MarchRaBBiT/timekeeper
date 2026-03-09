use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    admin::application::users::{extract_ip, extract_user_agent},
    application::{
        clock::{Clock, SYSTEM_CLOCK},
        dto::SessionActionResponse,
        http::forbidden_error,
    },
    error::AppError,
    middleware::request_id::RequestId,
    models::{active_session::ActiveSession, user::User},
    repositories::{active_session, auth as auth_repo},
    services::audit_log::{AuditLogEntry, AuditLogServiceTrait},
    services::token_cache::TokenCacheServiceTrait,
    types::UserId,
    utils::jwt::Claims,
};

#[derive(Debug, Serialize)]
pub struct AdminSessionResponse {
    pub id: String,
    pub user_id: String,
    pub device_label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

impl AdminSessionResponse {
    pub fn from_session(session: ActiveSession, current_jti: &str) -> Self {
        let is_current = session
            .access_jti
            .as_deref()
            .map(|jti| jti == current_jti)
            .unwrap_or(false);
        Self {
            id: session.id,
            user_id: session.user_id.to_string(),
            device_label: session.device_label,
            created_at: session.created_at,
            last_seen_at: session.last_seen_at,
            expires_at: session.expires_at,
            is_current,
        }
    }
}

pub async fn list_user_sessions(
    read_pool: &sqlx::PgPool,
    user: &User,
    claims: &Claims,
    user_id: &str,
) -> Result<Vec<AdminSessionResponse>, AppError> {
    ensure_admin_or_system(user)?;

    let user_id = user_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let sessions = active_session::list_active_sessions_for_user(read_pool, user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    Ok(sessions
        .into_iter()
        .map(|session| AdminSessionResponse::from_session(session, &claims.jti))
        .collect())
}

pub async fn revoke_session(
    write_pool: &sqlx::PgPool,
    token_cache: Option<&Arc<dyn TokenCacheServiceTrait>>,
    user: &User,
    request_id: &RequestId,
    audit_log_service: Arc<dyn AuditLogServiceTrait>,
    headers: &HeaderMap,
    session_id: &str,
) -> Result<SessionActionResponse, AppError> {
    ensure_admin_or_system(user)?;

    if session_id.trim().is_empty() {
        return Err(AppError::BadRequest("Session ID is required".into()));
    }

    let session = active_session::find_active_session_by_id(write_pool, session_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

    revoke_session_tokens(write_pool, token_cache, &session).await?;
    record_session_audit_event(
        Some(audit_log_service),
        &user.id,
        headers,
        request_id,
        "session_destroy",
        Some(session.id.clone()),
        Some(serde_json::json!({
            "reason": "admin_revoke",
            "target_user_id": session.user_id.to_string()
        })),
    );

    Ok(SessionActionResponse::new("Session revoked", session_id))
}

pub async fn revoke_session_tokens(
    pool: &sqlx::PgPool,
    token_cache: Option<&Arc<dyn TokenCacheServiceTrait>>,
    session: &ActiveSession,
) -> Result<(), AppError> {
    if let Some(access_jti) = session.access_jti.as_deref() {
        auth_repo::delete_active_access_token_by_jti(pool, access_jti)
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;
        if let Some(cache) = token_cache {
            let _ = cache.invalidate_token(access_jti).await;
        }
    }

    auth_repo::delete_refresh_token_by_id(pool, &session.refresh_token_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    active_session::delete_active_session_by_id(pool, &session.id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(())
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

fn ensure_admin_or_system(user: &User) -> Result<(), AppError> {
    if user.is_admin() || user.is_system_admin() {
        Ok(())
    } else {
        Err(forbidden_error("Forbidden"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::UserRole;

    fn sample_admin() -> User {
        let now = Utc::now();
        User {
            id: UserId::new(),
            username: "admin".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Admin".to_string(),
            email: "admin@example.com".to_string(),
            role: UserRole::Admin,
            is_system_admin: false,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            failed_login_attempts: 0,
            locked_until: None,
            lock_reason: None,
            lockout_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn ensure_admin_or_system_accepts_admin() {
        assert!(ensure_admin_or_system(&sample_admin()).is_ok());
    }

    #[test]
    fn from_session_marks_current_jti() {
        let session = ActiveSession {
            id: "session-1".to_string(),
            user_id: UserId::new(),
            refresh_token_id: "refresh-1".to_string(),
            access_jti: Some("jti-1".to_string()),
            device_label: Some("browser".to_string()),
            created_at: Utc::now(),
            last_seen_at: None,
            expires_at: Utc::now(),
        };

        let response = AdminSessionResponse::from_session(session, "jti-1");
        assert!(response.is_current);
    }
}
