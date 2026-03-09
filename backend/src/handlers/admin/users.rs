use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::IntoParams;

use crate::{
    admin::application::users as application,
    error::AppError,
    middleware::request_id::RequestId,
    models::user::{CreateUser, UpdateUser, User, UserResponse},
    services::audit_log::AuditLogServiceTrait,
    state::AppState,
};

pub async fn get_users(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse, AppError> {
    let result = application::get_users(state.read_pool(), &state.config, &user).await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-PII-Masked",
        HeaderValue::from_static(if result.pii_masked { "true" } else { "false" }),
    );
    Ok((headers, Json(result.users)))
}

pub async fn create_user(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<UserResponse>, AppError> {
    Ok(Json(
        application::create_user(&state.write_pool, &state.config, &user, payload).await?,
    ))
}

pub async fn update_user(
    State(state): State<AppState>,
    Extension(admin): Extension<User>,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    Ok(Json(
        application::update_user(&state.write_pool, &state.config, &admin, &user_id, payload)
            .await?,
    ))
}

pub async fn reset_user_mfa(
    State(state): State<AppState>,
    Extension(requester): Extension<User>,
    request_id: Option<Extension<RequestId>>,
    audit_log_service: Option<Extension<Arc<dyn AuditLogServiceTrait>>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<Json<application::UserActionMessage>, AppError> {
    let response = application::reset_user_mfa(
        &state.write_pool,
        &requester,
        &user_id,
        request_id.map(|Extension(id)| id),
        audit_log_service.map(|Extension(service)| service),
        &headers,
    )
    .await?;

    Ok(Json(response))
}

pub async fn unlock_user_account(
    State(state): State<AppState>,
    Extension(requester): Extension<User>,
    request_id: Option<Extension<RequestId>>,
    audit_log_service: Option<Extension<Arc<dyn AuditLogServiceTrait>>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<Json<application::UserActionMessage>, AppError> {
    let response = application::unlock_user_account(
        &state.write_pool,
        &requester,
        &user_id,
        request_id.map(|Extension(id)| id),
        audit_log_service.map(|Extension(service)| service),
        &headers,
    )
    .await?;

    Ok(Json(response))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct DeleteUserParams {
    #[serde(default)]
    pub hard: bool,
}

pub async fn delete_user(
    State(state): State<AppState>,
    Extension(requester): Extension<User>,
    Path(user_id): Path<String>,
    Query(params): Query<DeleteUserParams>,
) -> Result<Json<application::DeleteUserMessage>, AppError> {
    let response =
        application::delete_user(&state.write_pool, &requester, &user_id, params.hard).await?;
    Ok(Json(response))
}

#[derive(Debug, Serialize)]
pub struct ArchivedUserResponse {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub is_system_admin: bool,
    pub archived_at: String,
    pub archived_by: Option<String>,
}

pub async fn get_archived_users(
    State(state): State<AppState>,
    Extension(requester): Extension<User>,
) -> Result<Json<Vec<ArchivedUserResponse>>, AppError> {
    let response = application::get_archived_users(state.read_pool(), &state.config, &requester)
        .await?
        .into_iter()
        .map(ArchivedUserResponse::from)
        .collect();

    Ok(Json(response))
}

pub async fn restore_archived_user(
    State(state): State<AppState>,
    Extension(requester): Extension<User>,
    Path(user_id): Path<String>,
) -> Result<Json<application::UserActionMessage>, AppError> {
    let response =
        application::restore_archived_user(&state.write_pool, &state.config, &requester, &user_id)
            .await?;
    Ok(Json(response))
}

pub async fn delete_archived_user(
    State(state): State<AppState>,
    Extension(requester): Extension<User>,
    Path(user_id): Path<String>,
) -> Result<Json<application::UserActionMessage>, AppError> {
    let response =
        application::delete_archived_user(&state.write_pool, &requester, &user_id).await?;
    Ok(Json(response))
}

impl From<application::ArchivedUserResponse> for ArchivedUserResponse {
    fn from(value: application::ArchivedUserResponse) -> Self {
        Self {
            id: value.id,
            username: value.username,
            full_name: value.full_name,
            role: value.role,
            is_system_admin: value.is_system_admin,
            archived_at: value.archived_at,
            archived_by: value.archived_by,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UserId;
    use std::str::FromStr;

    #[test]
    fn test_delete_user_params_default() {
        let params = DeleteUserParams { hard: false };
        assert!(!params.hard);
    }

    #[test]
    fn test_delete_user_params_hard_delete() {
        let params = DeleteUserParams { hard: true };
        assert!(params.hard);
    }

    #[test]
    fn test_archived_user_response_structure() {
        let response = ArchivedUserResponse {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            full_name: "Test User".to_string(),
            role: "employee".to_string(),
            is_system_admin: false,
            archived_at: "2024-01-15T10:00:00Z".to_string(),
            archived_by: Some("admin-id".to_string()),
        };
        assert_eq!(response.id, "test-id");
        assert_eq!(response.username, "testuser");
        assert_eq!(response.full_name, "Test User");
        assert_eq!(response.role, "employee");
        assert!(!response.is_system_admin);
        assert_eq!(response.archived_at, "2024-01-15T10:00:00Z");
        assert_eq!(response.archived_by, Some("admin-id".to_string()));
    }

    #[test]
    fn test_archived_user_response_without_archived_by() {
        let response = ArchivedUserResponse {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            full_name: "Test User".to_string(),
            role: "admin".to_string(),
            is_system_admin: true,
            archived_at: "2024-01-15T10:00:00Z".to_string(),
            archived_by: None,
        };
        assert!(response.is_system_admin);
        assert_eq!(response.archived_by, None);
    }

    #[test]
    fn test_archived_user_response_from_application_response() {
        let response = ArchivedUserResponse::from(application::ArchivedUserResponse {
            id: "user-1".to_string(),
            username: "alice".to_string(),
            full_name: "Alice".to_string(),
            role: "employee".to_string(),
            is_system_admin: false,
            archived_at: "2026-03-09T00:00:00Z".to_string(),
            archived_by: Some("admin-1".to_string()),
        });

        assert_eq!(response.id, "user-1");
        assert_eq!(response.username, "alice");
        assert_eq!(response.archived_by.as_deref(), Some("admin-1"));
    }

    #[test]
    fn test_user_id_parsing_valid() {
        let valid_id = UserId::new();
        let id_string = valid_id.to_string();
        let parsed = UserId::from_str(&id_string);
        assert!(parsed.is_ok());
        assert_eq!(parsed.expect("parse user id"), valid_id);
    }

    #[test]
    fn test_user_id_parsing_invalid() {
        let parsed = UserId::from_str("not-a-valid-uuid");
        assert!(parsed.is_err());
    }
}
