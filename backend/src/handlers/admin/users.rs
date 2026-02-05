use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::{
    error::AppError,
    models::user::{CreateUser, UpdateUser, User, UserResponse},
    repositories::{auth as auth_repo, user as user_repo},
    state::AppState,
    types::UserId,
    utils::password::{hash_password, validate_password_complexity},
};

pub async fn get_users(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    if !(user.is_admin() || user.is_system_admin()) {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    // Normalize role to snake_case at read to be resilient to legacy rows
    let users = user_repo::list_users(state.read_pool())
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let responses = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}

pub async fn create_user(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<UserResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    payload.validate()?;
    validate_password_complexity(&payload.password, &state.config)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Check if username already exists
    if let Some(_) = auth_repo::find_user_by_username(&state.write_pool, &payload.username)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
    {
        return Err(AppError::BadRequest("Username already exists".into()));
    }

    let password_to_hash = payload.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || hash_password(&password_to_hash))
        .await
        .map_err(|_| {
            AppError::InternalServerError(anyhow::anyhow!("Password hashing task failed"))
        })?
        .map_err(|e| {
            AppError::InternalServerError(anyhow::anyhow!("Failed to hash password: {}", e))
        })?;

    let user = User::new(
        payload.username,
        password_hash,
        payload.full_name,
        payload.email,
        payload.role,
        payload.is_system_admin,
    );

    user_repo::create_user(&state.write_pool, &user)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let response = UserResponse::from(user);
    Ok(Json(response))
}

pub async fn update_user(
    State(state): State<AppState>,
    Extension(admin): Extension<User>,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    if !admin.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    payload.validate()?;

    let user_id_obj =
        UserId::from_str(&user_id).map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let existing_user = auth_repo::find_user_by_id(&state.write_pool, user_id_obj)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    if let Some(ref email) = payload.email {
        let email_exists = user_repo::email_exists_for_other_user(
            &state.write_pool,
            email,
            &existing_user.id.to_string(),
        )
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

        if email_exists {
            return Err(AppError::BadRequest("Email already in use".into()));
        }
    }

    let full_name = payload.full_name.unwrap_or(existing_user.full_name);
    let email = payload.email.unwrap_or(existing_user.email);
    let role = payload.role.unwrap_or(existing_user.role);
    let is_system_admin = payload
        .is_system_admin
        .unwrap_or(existing_user.is_system_admin);

    let updated_user = user_repo::update_user(
        &state.write_pool,
        &user_id,
        &full_name,
        &email,
        role,
        is_system_admin,
    )
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(Json(UserResponse::from(updated_user)))
}

#[derive(Deserialize, ToSchema)]
pub struct ResetMfaPayload {
    pub user_id: String,
}

pub async fn reset_user_mfa(
    State(state): State<AppState>,
    Extension(requester): Extension<User>,
    Json(payload): Json<ResetMfaPayload>,
) -> Result<Json<Value>, AppError> {
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden(
            "Only system administrators can reset MFA".into(),
        ));
    }
    let success = user_repo::reset_mfa(&state.write_pool, &payload.user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if !success {
        return Err(AppError::NotFound("User not found".into()));
    }

    let user_id = UserId::from_str(&payload.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".into()))?;

    auth_repo::delete_refresh_tokens_for_user(&state.write_pool, user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(Json(json!({
        "message": "MFA reset and refresh tokens revoked",
        "user_id": payload.user_id
    })))
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
) -> Result<Json<Value>, AppError> {
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let parsed_user_id = UserId::from_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".into()))?;

    if requester.id == parsed_user_id {
        return Err(AppError::BadRequest("Cannot delete yourself".into()));
    }

    let exists = user_repo::user_exists(&state.write_pool, &user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if !exists {
        return Err(AppError::NotFound("User not found".into()));
    }

    let username = user_repo::fetch_username(&state.write_pool, &user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .unwrap_or_default();

    if params.hard {
        user_repo::hard_delete_user(&state.write_pool, &user_id).await?;

        tracing::info!(
            user_id = %user_id,
            username = %username,
            requester_id = %requester.id,
            "user hard deleted"
        );

        Ok(Json(json!({
            "message": "User permanently deleted",
            "user_id": user_id,
            "deletion_type": "hard"
        })))
    } else {
        user_repo::soft_delete_user(&state.write_pool, &user_id, &requester.id.to_string()).await?;

        tracing::info!(
            user_id = %user_id,
            username = %username,
            requester_id = %requester.id,
            "user soft deleted (archived)"
        );

        Ok(Json(json!({
            "message": "User archived",
            "user_id": user_id,
            "deletion_type": "soft"
        })))
    }
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
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let rows = user_repo::get_archived_users(state.read_pool())
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let response: Vec<ArchivedUserResponse> = rows
        .into_iter()
        .map(|row| ArchivedUserResponse {
            id: row.id,
            username: row.username,
            full_name: row.full_name,
            role: row.role,
            is_system_admin: row.is_system_admin,
            archived_at: row.archived_at.to_rfc3339(),
            archived_by: row.archived_by,
        })
        .collect();

    Ok(Json(response))
}

pub async fn restore_archived_user(
    State(state): State<AppState>,
    Extension(requester): Extension<User>,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let exists = user_repo::archived_user_exists(&state.write_pool, &user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if !exists {
        return Err(AppError::NotFound("Archived user not found".into()));
    }

    let username = user_repo::fetch_archived_username(&state.write_pool, &user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .unwrap_or_default();

    let conflict_check = user_repo::username_exists(&state.write_pool, &username)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if conflict_check {
        return Err(AppError::BadRequest(
            "Username already in use by another user".into(),
        ));
    }

    user_repo::restore_user(&state.write_pool, &user_id).await?;

    tracing::info!(
        user_id = %user_id,
        username = %username,
        requester_id = %requester.id,
        "user restored from archive"
    );

    Ok(Json(json!({
        "message": "User restored",
        "user_id": user_id
    })))
}

pub async fn delete_archived_user(
    State(state): State<AppState>,
    Extension(requester): Extension<User>,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let exists = user_repo::archived_user_exists(&state.write_pool, &user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if !exists {
        return Err(AppError::NotFound("Archived user not found".into()));
    }

    let username = user_repo::fetch_archived_username(&state.write_pool, &user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .unwrap_or_default();

    user_repo::hard_delete_archived_user(&state.write_pool, &user_id).await?;

    tracing::info!(
        user_id = %user_id,
        username = %username,
        requester_id = %requester.id,
        "archived user permanently deleted"
    );

    Ok(Json(json!({
        "message": "Archived user permanently deleted",
        "user_id": user_id
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_mfa_payload_structure() {
        let payload = ResetMfaPayload {
            user_id: "test-user-id".to_string(),
        };
        assert_eq!(payload.user_id, "test-user-id");
    }

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
    fn test_user_id_parsing_valid() {
        let valid_id = UserId::new();
        let id_string = valid_id.to_string();
        let parsed = UserId::from_str(&id_string);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap(), valid_id);
    }

    #[test]
    fn test_user_id_parsing_invalid() {
        let invalid_id = "not-a-valid-uuid";
        let parsed = UserId::from_str(invalid_id);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_user_role_admin_check() {
        use crate::models::user::{User, UserRole};
        use chrono::Utc;

        let admin_user = User {
            id: UserId::new(),
            username: "admin".to_string(),
            email: "admin@example.com".to_string(),
            full_name: "Admin User".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Admin,
            is_system_admin: false,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(admin_user.is_admin());
        assert!(!admin_user.is_system_admin());
    }

    #[test]
    fn test_user_role_system_admin_check() {
        use crate::models::user::{User, UserRole};
        use chrono::Utc;

        let sys_admin_user = User {
            id: UserId::new(),
            username: "sysadmin".to_string(),
            email: "sysadmin@example.com".to_string(),
            full_name: "System Admin".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Admin,
            is_system_admin: true,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(sys_admin_user.is_admin());
        assert!(sys_admin_user.is_system_admin());
    }

    #[test]
    fn test_user_role_employee_check() {
        use crate::models::user::{User, UserRole};
        use chrono::Utc;

        let employee_user = User {
            id: UserId::new(),
            username: "employee".to_string(),
            email: "employee@example.com".to_string(),
            full_name: "Employee User".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Employee,
            is_system_admin: false,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!employee_user.is_admin());
        assert!(!employee_user.is_system_admin());
    }
}
