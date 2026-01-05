use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::{
    config::Config,
    models::user::{CreateUser, User, UserResponse, UserRole},
    repositories::user as user_repo,
    utils::password::hash_password,
    error::AppError,
};

pub async fn get_users(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    if !(user.is_admin() || user.is_system_admin()) {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    // Normalize role to snake_case at read to be resilient to legacy rows
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at FROM users ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    let responses = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}

pub async fn create_user(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<UserResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    payload.validate()?;
    // Check if username already exists
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, role, is_system_admin, mfa_secret, \
         mfa_enabled_at, created_at, updated_at FROM users WHERE username = $1",
    )
    .bind(&payload.username)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    if existing_user.is_some() {
        return Err(AppError::BadRequest("Username already exists".into()));
    }

    let password_hash = hash_password(&payload.password).map_err(|e| {
        AppError::InternalServerError(anyhow::anyhow!("Failed to hash password: {}", e))
    })?;

    let user = User::new(
        payload.username,
        password_hash,
        payload.full_name,
        payload.role,
        payload.is_system_admin,
    );

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(&user.id)
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    // Store enum as snake_case text to match sqlx mapping
    .bind(match user.role {
        UserRole::Employee => "employee",
        UserRole::Admin => "admin",
    })
    .bind(user.is_system_admin)
    .bind(&user.mfa_secret)
    .bind(user.mfa_enabled_at)
    .bind(user.created_at)
    .bind(user.updated_at)
    .execute(&pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    let response = UserResponse::from(user);
    Ok(Json(response))
}

#[derive(Deserialize, ToSchema)]
pub struct ResetMfaPayload {
    pub user_id: String,
}

pub async fn reset_user_mfa(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(requester): Extension<User>,
    Json(payload): Json<ResetMfaPayload>,
) -> Result<Json<Value>, AppError> {
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden("Only system administrators can reset MFA".into()));
    }
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE users SET mfa_secret = NULL, mfa_enabled_at = NULL, updated_at = $1 WHERE id = $2",
    )
    .bind(now)
    .bind(&payload.user_id)
    .execute(&pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".into()));
    }
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(&payload.user_id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?; // Keeping map_err here or using ? if simplified globally, but let's stick to simple ? if possible. Wait, map_err fixes type inference? No, removing it does.

    Ok(Json(json!({
        "message": "MFA reset and refresh tokens revoked",
        "user_id": payload.user_id
    })))
}

// ============================================================================
// User deletion
// ============================================================================

#[derive(Debug, Deserialize, IntoParams)]
pub struct DeleteUserParams {
    /// If true, permanently delete user and all data. If false, archive (soft delete).
    #[serde(default)]
    pub hard: bool,
}

/// Delete a user (soft or hard delete).
/// Soft delete: Archives user and related data (except session tokens).
/// Hard delete: Permanently removes user and all related data.
pub async fn delete_user(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(requester): Extension<User>,
    Path(user_id): Path<String>,
    Query(params): Query<DeleteUserParams>,
) -> Result<Json<Value>, AppError> {
    // Only system admins can delete users
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    // Cannot delete self
    if requester.id == user_id {
        return Err(AppError::BadRequest("Cannot delete yourself".into()));
    }

    // Check if user exists
    let exists = user_repo::user_exists(&pool, &user_id).await.map_err(|e| {
        AppError::InternalServerError(e.into())
    })?;

    if !exists {
        return Err(AppError::NotFound("User not found".into()));
    }

    // Get username for audit log
    let username = user_repo::fetch_username(&pool, &user_id)
        .await
        .map_err(|e| {
            AppError::InternalServerError(e.into())
        })?
        .unwrap_or_default();

    if params.hard {
        // Hard delete - CASCADE will remove all related data
        user_repo::hard_delete_user(&pool, &user_id)
            .await
            .map_err(|e| {
                AppError::InternalServerError(e.into())
            })?;

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
        // Soft delete - move to archive tables
        user_repo::soft_delete_user(&pool, &user_id, &requester.id)
            .await
            .map_err(|e| {
                AppError::InternalServerError(e.into())
            })?;

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

// ============================================================================
// Archived user management
// ============================================================================

/// Response type for archived user list.
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

/// Get all archived users.
pub async fn get_archived_users(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(requester): Extension<User>,
) -> Result<Json<Vec<ArchivedUserResponse>>, AppError> {
    // Only system admins can view archived users
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let rows = user_repo::get_archived_users(&pool).await.map_err(|e| {
        AppError::InternalServerError(e.into())
    })?;

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

/// Restore an archived user.
pub async fn restore_archived_user(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(requester): Extension<User>,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    // Only system admins can restore users
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    // Check if archived user exists
    let exists = user_repo::archived_user_exists(&pool, &user_id)
        .await
        .map_err(|e| {
            AppError::InternalServerError(e.into())
        })?;

    if !exists {
        return Err(AppError::NotFound("Archived user not found".into()));
    }

    // Check if username already exists in active users
    let username = user_repo::fetch_archived_username(&pool, &user_id)
        .await
        .map_err(|e| {
            AppError::InternalServerError(e.into())
        })?
        .unwrap_or_default();

    // Check for username conflict
    let conflict_check: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            AppError::InternalServerError(e.into())
        })?;

    if conflict_check.is_some() {
        return Err(AppError::BadRequest("Username already in use by another user".into()));
    }

    // Restore user
    user_repo::restore_user(&pool, &user_id).await.map_err(|e| {
        AppError::InternalServerError(e.into())
    })?;

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

/// Permanently delete an archived user.
pub async fn delete_archived_user(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(requester): Extension<User>,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    // Only system admins can delete archived users
    if !requester.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    // Check if archived user exists
    let exists = user_repo::archived_user_exists(&pool, &user_id)
        .await
        .map_err(|e| {
            AppError::InternalServerError(e.into())
        })?;

    if !exists {
        return Err(AppError::NotFound("Archived user not found".into()));
    }

    // Get username for logging
    let username = user_repo::fetch_archived_username(&pool, &user_id)
        .await
        .map_err(|e| {
            AppError::InternalServerError(e.into())
        })?
        .unwrap_or_default();

    // Delete archived user
    user_repo::hard_delete_archived_user(&pool, &user_id)
        .await
        .map_err(|e| {
            AppError::InternalServerError(e.into())
        })?;

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
