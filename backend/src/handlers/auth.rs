use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    config::Config,
    models::user::{
        ChangePasswordRequest, LoginRequest, LoginResponse, MfaCodeRequest, MfaSetupResponse,
        MfaStatusResponse, User, UserResponse,
    },
    utils::{
        jwt::{
            create_access_token, create_refresh_token, decode_refresh_token, verify_refresh_token,
        },
        mfa::{generate_otpauth_uri, generate_totp_secret, verify_totp_code},
        password::{hash_password, verify_password},
    },
};

pub async fn login(
    State((pool, config)): State<(PgPool, Config)>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<Value>)> {
    // Find user by username
    let user = match sqlx::query_as::<_, User>("SELECT id, username, password_hash, full_name, LOWER(role) as role, is_system_admin, mfa_secret, mfa_enabled_at, created_at, updated_at FROM users WHERE username = $1")
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

    if user.is_mfa_enabled() {
        let totp_code = payload
            .totp_code
            .as_deref()
            .map(str::trim)
            .filter(|code| !code.is_empty())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "MFA code required"})),
                )
            })?;

        let secret = user.mfa_secret.as_ref().ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "MFA secret missing"})),
            )
        })?;

        if !verify_totp_code(secret, totp_code).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "MFA verification error"})),
            )
        })? {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid MFA code"})),
            ));
        }
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
    let refresh_token_data =
        create_refresh_token(user.id.clone(), config.refresh_token_expiration_days).map_err(
            |_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Refresh token creation error"})),
                )
            },
        )?;

    // Store refresh token in database
    if let Err(_) = sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
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
        refresh_token: refresh_token_data.encoded(),
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
    let (refresh_token_id, refresh_token_secret) =
        decode_refresh_token(refresh_token).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired refresh token"})),
            )
        })?;

    // Find refresh token in database
    use sqlx::Row;
    let token_row = sqlx::query(
        "SELECT id, user_id, token_hash, expires_at FROM refresh_tokens WHERE id = $1 AND expires_at > $2"
    )
    .bind(&refresh_token_id)
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
    let token_hash: String = match token_record.try_get::<String, _>("token_hash") {
        Ok(hash) if !hash.is_empty() => hash,
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired refresh token"})),
            ))
        }
    };
    let user_id: String = match token_record.try_get::<String, _>("user_id") {
        Ok(id) if !id.is_empty() => id,
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired refresh token"})),
            ))
        }
    };
    if user_id.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired refresh token"})),
        ));
    }
    let valid = verify_refresh_token(&refresh_token_secret, &token_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Refresh token verification error"})),
        )
    })?;
    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired refresh token"})),
        ));
    }

    // Get user information
    let user = match sqlx::query_as::<_, User>("SELECT id, username, password_hash, full_name, LOWER(role) as role, is_system_admin, mfa_secret, mfa_enabled_at, created_at, updated_at FROM users WHERE id = $1")
    .bind(&user_id)
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
    let new_refresh_token_data =
        create_refresh_token(user.id.clone(), config.refresh_token_expiration_days).map_err(
            |_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Refresh token creation error"})),
                )
            },
        )?;

    // Delete old refresh token and store new one
    if let Err(_) = sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
        .bind(&refresh_token_id)
        .execute(&pool)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to delete old refresh token"})),
        ));
    }

    if let Err(_) = sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
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
        refresh_token: new_refresh_token_data.encoded(),
        user: UserResponse::from(user),
    };

    Ok(Json(response))
}

pub async fn mfa_status(
    Extension(user): Extension<User>,
) -> Result<Json<MfaStatusResponse>, (StatusCode, Json<Value>)> {
    Ok(Json(MfaStatusResponse {
        enabled: user.is_mfa_enabled(),
        pending: user.has_pending_mfa(),
    }))
}

async fn begin_mfa_enrollment(
    pool: &PgPool,
    config: &Config,
    user: &User,
) -> Result<MfaSetupResponse, (StatusCode, Json<Value>)> {
    if user.is_mfa_enabled() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "MFA already enabled"})),
        ));
    }

    let secret = generate_totp_secret();
    let otpauth_url =
        generate_otpauth_uri(&config.mfa_issuer, &user.username, &secret).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to issue MFA secret"})),
            )
        })?;

    if let Err(_) = sqlx::query(
        "UPDATE users SET mfa_secret = $1, mfa_enabled_at = NULL, updated_at = $2 WHERE id = $3",
    )
    .bind(&secret)
    .bind(Utc::now())
    .bind(&user.id)
    .execute(pool)
    .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to persist MFA secret"})),
        ));
    }

    Ok(MfaSetupResponse {
        secret,
        otpauth_url,
    })
}

pub async fn mfa_setup(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<MfaSetupResponse>, (StatusCode, Json<Value>)> {
    let response = begin_mfa_enrollment(&pool, &config, &user).await?;
    Ok(Json(response))
}

pub async fn mfa_register(
    State((pool, config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
) -> Result<Json<MfaSetupResponse>, (StatusCode, Json<Value>)> {
    let response = begin_mfa_enrollment(&pool, &config, &user).await?;
    Ok(Json(response))
}

pub async fn mfa_activate(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<MfaCodeRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let secret = user.mfa_secret.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "MFA setup not initiated"})),
        )
    })?;

    let code = payload.code.trim().to_string();
    if !verify_totp_code(secret, &code).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "MFA verification error"})),
        )
    })? {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid MFA code"})),
        ));
    }

    let now = Utc::now();
    if let Err(_) =
        sqlx::query("UPDATE users SET mfa_enabled_at = $1, updated_at = $1 WHERE id = $2")
            .bind(now)
            .bind(&user.id)
            .execute(&pool)
            .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to enable MFA"})),
        ));
    }

    if let Err(_) = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(&user.id)
        .execute(&pool)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to revoke refresh tokens"})),
        ));
    }

    Ok(Json(json!({"message": "MFA enabled"})))
}

pub async fn mfa_disable(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<MfaCodeRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !user.is_mfa_enabled() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "MFA is not enabled"})),
        ));
    }

    let secret = user.mfa_secret.as_ref().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "MFA secret missing"})),
        )
    })?;

    let code = payload.code.trim().to_string();
    if !verify_totp_code(secret, &code).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "MFA verification error"})),
        )
    })? {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid MFA code"})),
        ));
    }

    if let Err(_) = sqlx::query(
        "UPDATE users SET mfa_secret = NULL, mfa_enabled_at = NULL, updated_at = $1 WHERE id = $2",
    )
    .bind(Utc::now())
    .bind(&user.id)
    .execute(&pool)
    .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to disable MFA"})),
        ));
    }

    if let Err(_) = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(&user.id)
        .execute(&pool)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to revoke refresh tokens"})),
        ));
    }

    Ok(Json(json!({"message": "MFA disabled"})))
}

pub async fn change_password(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Basic guardrails for the new password to reduce trivial mistakes.
    if payload.new_password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "New password must be at least 8 characters"})),
        ));
    }
    if payload.new_password == payload.current_password {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "New password must differ from current password"})),
        ));
    }

    // Verify the existing password matches what is stored.
    let matches =
        verify_password(&payload.current_password, &user.password_hash).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Password verification error"})),
            )
        })?;
    if !matches {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Current password is incorrect"})),
        ));
    }

    // Hash and persist the new password.
    let new_hash = hash_password(&payload.new_password).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to hash password"})),
        )
    })?;

    if let Err(_) =
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3")
            .bind(&new_hash)
            .bind(Utc::now())
            .bind(&user.id)
            .execute(&pool)
            .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update password"})),
        ));
    }

    // Revoke outstanding refresh tokens so the user must reauthenticate.
    if let Err(_) = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(&user.id)
        .execute(&pool)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to revoke refresh tokens"})),
        ));
    }

    Ok(Json(json!({"message": "Password updated successfully"})))
}

pub async fn logout(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<Value>)> {
    // If `all` is true, revoke all refresh tokens for this user
    let all = payload
        .get("all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if all {
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
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
        let (token_id, _) = decode_refresh_token(rt).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"Invalid refresh token"})),
            )
        })?;
        sqlx::query("DELETE FROM refresh_tokens WHERE id = $1 AND user_id = $2")
            .bind(&token_id)
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
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
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
