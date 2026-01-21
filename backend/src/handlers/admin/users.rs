use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::{
    error::AppError,
    models::user::{CreateUser, UpdateUser, User, UserResponse, UserRole},
    repositories::user as user_repo,
    state::AppState,
    types::UserId,
    utils::password::hash_password,
};

pub async fn get_users(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    if !(user.is_admin() || user.is_system_admin()) {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    // Normalize role to snake_case at read to be resilient to legacy rows
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at FROM users ORDER BY created_at DESC",
    )
    .fetch_all(state.read_pool())
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
    // Check if username already exists
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, email, role, is_system_admin, mfa_secret, \
         mfa_enabled_at, created_at, updated_at FROM users WHERE username = $1",
    )
    .bind(&payload.username)
    .fetch_optional(&state.write_pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    if existing_user.is_some() {
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

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, email, role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(user.id.to_string())
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    .bind(&user.email)
    .bind(match user.role {
        UserRole::Employee => "employee",
        UserRole::Admin => "admin",
    })
    .bind(user.is_system_admin)
    .bind(&user.mfa_secret)
    .bind(user.mfa_enabled_at)
    .bind(user.created_at)
    .bind(user.updated_at)
    .execute(&state.write_pool)
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

    let user_id =
        UserId::from_str(&user_id).map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let existing_user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.write_pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?
    .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    if let Some(ref email) = payload.email {
        let email_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND id != $2)",
        )
        .bind(email)
        .bind(user_id)
        .fetch_one(&state.write_pool)
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

    let updated_user = sqlx::query_as::<_, User>(
        "UPDATE users SET full_name = $1, email = $2, role = $3, is_system_admin = $4, updated_at = NOW() \
         WHERE id = $5 \
         RETURNING id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at",
    )
    .bind(&full_name)
    .bind(&email)
    .bind(role.as_str())
    .bind(is_system_admin)
    .bind(user_id)
    .fetch_one(&state.write_pool)
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
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE users SET mfa_secret = NULL, mfa_enabled_at = NULL, updated_at = $1 WHERE id = $2",
    )
    .bind(now)
    .bind(&payload.user_id)
    .execute(&state.write_pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".into()));
    }
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(&payload.user_id)
        .execute(&state.write_pool)
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
        user_repo::hard_delete_user(&state.write_pool, &user_id)
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;

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
        user_repo::soft_delete_user(&state.write_pool, &user_id, &requester.id.to_string())
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;

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

    let conflict_check: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = $1")
            .bind(&username)
            .fetch_optional(&state.write_pool)
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;

    if conflict_check.is_some() {
        return Err(AppError::BadRequest(
            "Username already in use by another user".into(),
        ));
    }

    user_repo::restore_user(&state.write_pool, &user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

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

    user_repo::hard_delete_archived_user(&state.write_pool, &user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

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
