use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    application::dto::SessionActionResponse,
    error::AppError,
    identity::application::sessions::{
        list_user_sessions, record_session_audit_event, revoke_user_session, SessionView,
    },
    middleware::request_id::RequestId,
    models::user::User,
    services::audit_log::AuditLogServiceTrait,
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
) -> Result<Json<SessionActionResponse>, AppError> {
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
        Some(serde_json::json!({ "reason": "user_revoke" })),
    );

    Ok(Json(revoked))
}
