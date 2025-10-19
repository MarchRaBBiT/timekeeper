use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    config::Config,
    models::user::{LoginRequest, LoginResponse, User, UserResponse},
    utils::{
        jwt::{create_access_token, create_refresh_token},
        password::verify_password,
    },
};

pub async fn login(
    State((pool, config)): State<(PgPool, Config)>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<Value>)> {
    // Find user by username
    let user = match sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, LOWER(role) as role, created_at, updated_at FROM users WHERE username = ?"
    )
    .bind(&payload.username)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid username or password"})),
            ));
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            ));
        }
    };

    // Verify password
    if !verify_password(&payload.password, &user.password_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Password verification error"})),
        )
    })? {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid username or password"})),
        ));
    }

    // Create access token
    let access_token = create_access_token(
        user.id.clone(),
        user.username.clone(),
        user.role.as_str().to_string(),
        &config.jwt_secret,
        config.jwt_expiration_hours,
    )
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Token creation error"})),
        )
    })?;

    // Create refresh token
    let refresh_token_data = create_refresh_token(user.id.clone()).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Refresh token creation error"})),
        )
    })?;

    // Store refresh token in database
    if let Err(_) = sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&refresh_token_data.id)
    .bind(&refresh_token_data.user_id)
    .bind(&refresh_token_data.token_hash)
    .bind(&refresh_token_data.expires_at)
    .execute(&pool)
    .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to store refresh token"})),
        ));
    }

    let response = LoginResponse {
        access_token,
        refresh_token: refresh_token_data.id, // Return the unhashed token ID
        user: UserResponse::from(user),
    };

    Ok(Json(response))
}

pub async fn refresh(
    State((pool, config)): State<(PgPool, Config)>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<Value>)> {
    let refresh_token = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Refresh token is required"})),
            )
        })?;

    // Find refresh token in database
    use sqlx::Row;
    let token_row = sqlx::query(
        "SELECT id, user_id, token_hash, expires_at FROM refresh_tokens WHERE id = ? AND expires_at > ?"
    )
    .bind(refresh_token)
    .bind(Utc::now())
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))?;
    let token_record = match token_row {
        Some(row) => row,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired refresh token"})),
            ));
        }
    };

    // Get user information
    let user = match sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name, LOWER(role) as role, created_at, updated_at FROM users WHERE id = ?"
    )
    .bind(&token_record.try_get::<String,_>("user_id").unwrap_or_default())
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "User not found"})),
            ));
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            ));
        }
    };

    // Create new access token
    let access_token = create_access_token(
        user.id.clone(),
        user.username.clone(),
        user.role.as_str().to_string(),
        &config.jwt_secret,
        config.jwt_expiration_hours,
    )
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Token creation error"})),
        )
    })?;

    // Create new refresh token
    let new_refresh_token_data = create_refresh_token(user.id.clone()).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Refresh token creation error"})),
        )
    })?;

    // Delete old refresh token and store new one
    if let Err(_) = sqlx::query("DELETE FROM refresh_tokens WHERE id = ?")
        .bind(refresh_token)
        .execute(&pool)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to delete old refresh token"})),
        ));
    }

    if let Err(_) = sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&new_refresh_token_data.id)
    .bind(&new_refresh_token_data.user_id)
    .bind(&new_refresh_token_data.token_hash)
    .bind(&new_refresh_token_data.expires_at)
    .execute(&pool)
    .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to store new refresh token"})),
        ));
    }

    let response = LoginResponse {
        access_token,
        refresh_token: new_refresh_token_data.id,
        user: UserResponse::from(user),
    };

    Ok(Json(response))
}

pub async fn logout(
    State((pool, _config)): State<(PgPool, Config)>,
    axum::extract::Extension(user): axum::extract::Extension<User>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<Value>)> {
    // If `all` is true, revoke all refresh tokens for this user
    let all = payload
        .get("all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if all {
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
            .bind(&user.id)
            .execute(&pool)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error":"Failed to revoke tokens"})),
                )
            })?;
        return Ok(Json(json!({"message":"Logged out"})));
    }

    // Otherwise, revoke a specific refresh token by id if provided
    if let Some(rt) = payload.get("refresh_token").and_then(|v| v.as_str()) {
        sqlx::query("DELETE FROM refresh_tokens WHERE id = ? AND user_id = ?")
            .bind(rt)
            .bind(&user.id)
            .execute(&pool)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error":"Failed to revoke token"})),
                )
            })?;
        return Ok(Json(json!({"message":"Logged out"})));
    }

    // Default: revoke all for safety if no token provided
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
        .bind(&user.id)
        .execute(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Failed to revoke tokens"})),
            )
        })?;
    Ok(Json(json!({"message":"Logged out"})))
}
