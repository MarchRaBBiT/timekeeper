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
