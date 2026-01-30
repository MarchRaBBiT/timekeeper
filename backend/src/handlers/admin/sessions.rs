use axum::{
    extract::{Extension, Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::str::FromStr;
use utoipa::ToSchema;

use crate::{
    error::AppError,
    models::{active_session::ActiveSession, user::User},
    repositories::{active_session, auth as auth_repo},
    services::token_cache::TokenCacheServiceTrait,
    state::AppState,
    types::UserId,
    utils::jwt::Claims,
};

#[derive(Debug, Serialize, ToSchema)]
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
    fn from_session(session: ActiveSession, current_jti: &str) -> Self {
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
    State(state): State<AppState>,
    Extension(_user): Extension<User>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<AdminSessionResponse>>, AppError> {
    let user_id = UserId::from_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let sessions = active_session::list_active_sessions_for_user(state.read_pool(), user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    let responses = sessions
        .into_iter()
        .map(|session| AdminSessionResponse::from_session(session, &claims.jti))
        .collect();
    Ok(Json(responses))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(_user): Extension<User>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if session_id.trim().is_empty() {
        return Err(AppError::BadRequest("Session ID is required".into()));
    }

    let session = active_session::find_active_session_by_id(&state.write_pool, &session_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

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
