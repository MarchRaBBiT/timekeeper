use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    db::connection::DbPool, error::AppError, models::active_session::ActiveSession, types::UserId,
};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UserId;

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
}
