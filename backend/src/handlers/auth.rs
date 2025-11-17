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
    handlers::auth_repo::{self, StoredRefreshToken},
    models::user::{
        ChangePasswordRequest, LoginRequest, LoginResponse, MfaCodeRequest, MfaSetupResponse,
        MfaStatusResponse, User, UserResponse,
    },
    utils::{
        jwt::{
            create_access_token, create_refresh_token, decode_refresh_token, verify_refresh_token,
            RefreshToken,
        },
        mfa::{generate_otpauth_uri, generate_totp_secret, verify_totp_code},
        password::{hash_password, verify_password},
    },
};

type HandlerError = (StatusCode, Json<Value>);
type HandlerResult<T> = Result<T, HandlerError>;

pub async fn login(
    State((pool, config)): State<(PgPool, Config)>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<Value>)> {
    let user = auth_repo::find_user_by_username(&pool, &payload.username)
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("Invalid username or password"))?;

    ensure_password_matches(
        &payload.password,
        &user.password_hash,
        "Invalid username or password",
    )?;
    enforce_mfa(&user, payload.totp_code.as_deref())?;

    let access_token = create_access_token(
        user.id.clone(),
        user.username.clone(),
        user.role.as_str().to_string(),
        &config.jwt_secret,
        config.jwt_expiration_hours,
    )
    .map_err(|_| internal_error("Token creation error"))?;

    let refresh_token_data =
        create_refresh_token(user.id.clone(), config.refresh_token_expiration_days)
            .map_err(|_| internal_error("Refresh token creation error"))?;

    persist_refresh_token(&pool, &refresh_token_data, "Failed to store refresh token").await?;

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
        .ok_or_else(|| bad_request("Refresh token is required"))?;
    let (refresh_token_id, refresh_token_secret) = decode_refresh_token(refresh_token)
        .map_err(|_| unauthorized("Invalid or expired refresh token"))?;

    let token_record = fetch_refresh_token_or_unauthorized(&pool, &refresh_token_id).await?;
    verify_refresh_secret(&refresh_token_secret, &token_record.token_hash)?;

    let user = auth_repo::find_user_by_id(&pool, &token_record.user_id)
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("User not found"))?;

    let access_token = create_access_token(
        user.id.clone(),
        user.username.clone(),
        user.role.as_str().to_string(),
        &config.jwt_secret,
        config.jwt_expiration_hours,
    )
    .map_err(|_| internal_error("Token creation error"))?;

    let new_refresh_token_data =
        create_refresh_token(user.id.clone(), config.refresh_token_expiration_days)
            .map_err(|_| internal_error("Refresh token creation error"))?;

    revoke_refresh_token_by_id(
        &pool,
        &refresh_token_id,
        "Failed to delete old refresh token",
    )
    .await?;
    persist_refresh_token(
        &pool,
        &new_refresh_token_data,
        "Failed to store new refresh token",
    )
    .await?;

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
    let secret = user
        .mfa_secret
        .as_ref()
        .ok_or_else(|| bad_request("MFA setup not initiated"))?;

    let code = payload.code.trim().to_string();
    if !verify_totp_code(secret, &code).map_err(|_| internal_error("MFA verification error"))? {
        return Err(unauthorized("Invalid MFA code"));
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

    revoke_tokens_for_user(&pool, &user.id, "Failed to revoke refresh tokens").await?;

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

    ensure_password_matches(
        &payload.current_password,
        &user.password_hash,
        "Current password is incorrect",
    )?;

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

    revoke_tokens_for_user(&pool, &user.id, "Failed to revoke refresh tokens").await?;

    Ok(Json(json!({"message": "Password updated successfully"})))
}

pub async fn logout(
    State((pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<Value>)> {
    let all = payload
        .get("all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if all {
        revoke_tokens_for_user(&pool, &user.id, "Failed to revoke tokens").await?;
        return Ok(Json(json!({"message":"Logged out"})));
    }

    if let Some(rt) = payload.get("refresh_token").and_then(|v| v.as_str()) {
        let (token_id, _) =
            decode_refresh_token(rt).map_err(|_| bad_request("Invalid refresh token"))?;
        revoke_refresh_token_for_user(&pool, &token_id, &user.id).await?;
        return Ok(Json(json!({"message":"Logged out"})));
    }

    revoke_tokens_for_user(&pool, &user.id, "Failed to revoke tokens").await?;
    Ok(Json(json!({"message":"Logged out"})))
}

fn handler_error(status: StatusCode, message: &'static str) -> HandlerError {
    (status, Json(json!({ "error": message })))
}

fn bad_request(message: &'static str) -> HandlerError {
    handler_error(StatusCode::BAD_REQUEST, message)
}

fn unauthorized(message: &'static str) -> HandlerError {
    handler_error(StatusCode::UNAUTHORIZED, message)
}

fn internal_error(message: &'static str) -> HandlerError {
    handler_error(StatusCode::INTERNAL_SERVER_ERROR, message)
}

fn ensure_password_matches(
    candidate: &str,
    expected_hash: &str,
    unauthorized_message: &'static str,
) -> HandlerResult<()> {
    let matches = verify_password(candidate, expected_hash)
        .map_err(|_| internal_error("Password verification error"))?;
    if matches {
        Ok(())
    } else {
        Err(unauthorized(unauthorized_message))
    }
}

fn enforce_mfa(user: &User, code: Option<&str>) -> HandlerResult<()> {
    if !user.is_mfa_enabled() {
        return Ok(());
    }
    let totp_code = code
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .ok_or_else(|| unauthorized("MFA code required"))?;
    let secret = user
        .mfa_secret
        .as_ref()
        .ok_or_else(|| internal_error("MFA secret missing"))?;
    let valid = verify_totp_code(secret, totp_code)
        .map_err(|_| internal_error("MFA verification error"))?;
    if valid {
        Ok(())
    } else {
        Err(unauthorized("Invalid MFA code"))
    }
}

fn verify_refresh_secret(secret: &str, hash: &str) -> HandlerResult<()> {
    let valid = verify_refresh_token(secret, hash)
        .map_err(|_| internal_error("Refresh token verification error"))?;
    if valid {
        Ok(())
    } else {
        Err(unauthorized("Invalid or expired refresh token"))
    }
}

async fn persist_refresh_token(
    pool: &PgPool,
    token: &RefreshToken,
    error_message: &'static str,
) -> HandlerResult<()> {
    auth_repo::insert_refresh_token(pool, token)
        .await
        .map_err(|_| internal_error(error_message))
}

async fn fetch_refresh_token_or_unauthorized(
    pool: &PgPool,
    token_id: &str,
) -> HandlerResult<StoredRefreshToken> {
    auth_repo::fetch_valid_refresh_token(pool, token_id, Utc::now())
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("Invalid or expired refresh token"))
}

async fn revoke_refresh_token_by_id(
    pool: &PgPool,
    token_id: &str,
    error_message: &'static str,
) -> HandlerResult<()> {
    auth_repo::delete_refresh_token_by_id(pool, token_id)
        .await
        .map_err(|_| internal_error(error_message))
}

async fn revoke_refresh_token_for_user(
    pool: &PgPool,
    token_id: &str,
    user_id: &str,
) -> HandlerResult<()> {
    auth_repo::delete_refresh_token_for_user(pool, token_id, user_id)
        .await
        .map_err(|_| internal_error("Failed to revoke token"))
}

async fn revoke_tokens_for_user(
    pool: &PgPool,
    user_id: &str,
    error_message: &'static str,
) -> HandlerResult<()> {
    auth_repo::delete_refresh_tokens_for_user(pool, user_id)
        .await
        .map_err(|_| internal_error(error_message))
}
