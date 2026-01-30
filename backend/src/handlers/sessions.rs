use axum::{
    extract::{Extension, Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::{
    error::AppError,
    models::{active_session::ActiveSession, user::User},
    repositories::{active_session, auth as auth_repo},
    services::token_cache::TokenCacheServiceTrait,
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

impl SessionResponse {
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

pub async fn list_sessions(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<SessionResponse>>, AppError> {
    let sessions = active_session::list_active_sessions_for_user(state.read_pool(), user.id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    let responses = sessions
        .into_iter()
        .map(|session| SessionResponse::from_session(session, &claims.jti))
        .collect();
    Ok(Json(responses))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if session_id.trim().is_empty() {
        return Err(AppError::BadRequest("Session ID is required".into()));
    }

    let session = active_session::find_active_session_by_id(&state.write_pool, &session_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

    if session.user_id != user.id {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    if session
        .access_jti
        .as_deref()
        .map(|jti| jti == claims.jti.as_str())
        .unwrap_or(false)
    {
        return Err(AppError::BadRequest(
            "Cannot revoke current session; use logout instead".into(),
        ));
    }

    revoke_session_tokens(
        &state.write_pool,
        state.token_cache.as_ref(),
        &session,
    )
    .await?;

    Ok(Json(json!({
        "message": "Session revoked",
        "session_id": session_id
    })))
}

async fn revoke_session_tokens(
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
