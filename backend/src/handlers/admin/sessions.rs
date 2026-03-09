use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    admin::application::sessions as application, application::dto::SessionActionResponse,
    error::AppError, middleware::request_id::RequestId, models::user::User,
    services::audit_log::AuditLogServiceTrait, state::AppState, utils::jwt::Claims,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminSessionResponse {
    pub id: String,
    pub user_id: String,
    pub device_label: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub is_current: bool,
}

impl From<application::AdminSessionResponse> for AdminSessionResponse {
    fn from(value: application::AdminSessionResponse) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            device_label: value.device_label,
            created_at: value.created_at,
            last_seen_at: value.last_seen_at,
            expires_at: value.expires_at,
            is_current: value.is_current,
        }
    }
}

pub async fn list_user_sessions(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<AdminSessionResponse>>, AppError> {
    Ok(Json(
        application::list_user_sessions(state.read_pool(), &user, &claims, &user_id)
            .await?
            .into_iter()
            .map(AdminSessionResponse::from)
            .collect(),
    ))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(request_id): Extension<RequestId>,
    Extension(audit_log_service): Extension<Arc<dyn AuditLogServiceTrait>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<SessionActionResponse>, AppError> {
    Ok(Json(
        application::revoke_session(
            &state.write_pool,
            state.token_cache.as_ref(),
            &user,
            &request_id,
            audit_log_service,
            &headers,
            &session_id,
        )
        .await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn admin_session_response_from_application_response() {
        let response = AdminSessionResponse::from(application::AdminSessionResponse {
            id: "session-1".to_string(),
            user_id: "user-1".to_string(),
            device_label: Some("browser".to_string()),
            created_at: Utc::now(),
            last_seen_at: None,
            expires_at: Utc::now(),
            is_current: true,
        });

        assert_eq!(response.id, "session-1");
        assert_eq!(response.user_id, "user-1");
        assert!(response.is_current);
    }
}
