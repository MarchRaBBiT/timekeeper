use axum::{
    extract::{Extension, State},
    http::{header, HeaderMap},
    Json,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::future::Future;
use std::str::FromStr;

use crate::{
    config::Config,
    error::AppError,
    handlers::auth_repo::{self, ActiveAccessToken},
    models::{
        password_reset::{RequestPasswordResetPayload, ResetPasswordPayload},
        user::{
            ChangePasswordRequest, LoginRequest, LoginResponse, MfaCodeRequest, MfaSetupResponse,
            MfaStatusResponse, UpdateProfile, User, UserResponse,
        },
    },
    repositories::password_reset as password_reset_repo,
    state::AppState,
    types::UserId,
    utils::{
        cookies::{
            build_auth_cookie, build_clear_cookie, extract_cookie_value, CookieOptions,
            ACCESS_COOKIE_NAME, ACCESS_COOKIE_PATH, REFRESH_COOKIE_NAME, REFRESH_COOKIE_PATH,
        },
        email::EmailService,
        jwt::{
            create_access_token, create_refresh_token, decode_refresh_token, hash_refresh_token,
            verify_access_token, verify_refresh_token, Claims, RefreshToken,
        },
        mfa::{generate_otpauth_uri, generate_totp_secret, verify_totp_code},
        password::{hash_password, verify_password},
        security::generate_token,
    },
    validation::Validate,
};

pub type HandlerResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub struct AuthSession {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserResponse,
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    payload.validate()?;

    let user = auth_repo::find_user_by_username(&state.write_pool, &payload.username)
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("Invalid username or password"))?;

    let session = process_login_for_user(
        user,
        payload,
        &state.config,
        {
            let pool = state.write_pool.clone();
            move |token| async move {
                persist_refresh_token(&pool, &token, "Failed to store refresh token").await
            }
        },
        {
            let pool = state.write_pool.clone();
            let cache = state.token_cache.clone();
            move |claims, context| async move {
                persist_active_access_token(&pool, &claims, context.clone()).await?;
                if let Some(cache) = cache {
                    let user_id = UserId::from_str(&claims.sub)
                        .map_err(|_| internal_error("Invalid user ID"))?;
                    let ttl = (claims.exp - Utc::now().timestamp()).max(0) as u64;
                    let _ = cache.cache_token(&claims.jti, user_id, ttl).await;
                }
                Ok(())
            }
        },
    )
    .await?;

    let mut headers = HeaderMap::new();
    set_auth_cookies(&mut headers, &session, &state.config);
    Ok((headers, Json(LoginResponse { user: session.user })))
}

pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    let cookie_header = cookie_header_value(&headers);
    let refresh_token_str =
        extract_cookie_value(cookie_header.unwrap_or_default(), REFRESH_COOKIE_NAME)
            .or_else(|| {
                payload
                    .get("refresh_token")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string())
            })
            .ok_or_else(|| bad_request("Refresh token is required"))?;

    let (token_id, secret) = decode_refresh_token(&refresh_token_str)
        .map_err(|_| unauthorized("Invalid refresh token format"))?;

    let stored = auth_repo::fetch_valid_refresh_token(&state.write_pool, &token_id, Utc::now())
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("Invalid or expired refresh token"))?;

    if !verify_refresh_token(&secret, &stored.token_hash)
        .map_err(|_| internal_error("Verification error"))?
    {
        return Err(unauthorized("Invalid refresh token secret"));
    }

    let user = auth_repo::find_user_by_id(&state.write_pool, stored.user_id)
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("User not found"))?;

    let session = create_auth_session(&user, &state.config).await?;

    auth_repo::delete_refresh_token_by_id(&state.write_pool, &stored.id)
        .await
        .map_err(|_| internal_error("Failed to revoke old refresh token"))?;

    let rt_data = session.refresh_token_data(state.config.refresh_token_expiration_days)?;
    persist_refresh_token(
        &state.write_pool,
        &rt_data,
        "Failed to store new refresh token",
    )
    .await?;

    let claims = session.access_claims(&state.config.jwt_secret)?;
    persist_active_access_token(
        &state.write_pool,
        &claims,
        Some(format!("refresh_{}", stored.id)),
    )
    .await?;

    if let Some(cache) = &state.token_cache {
        let user_id =
            UserId::from_str(&claims.sub).map_err(|_| internal_error("Invalid user ID"))?;
        let ttl = (claims.exp - Utc::now().timestamp()).max(0) as u64;
        let _ = cache.cache_token(&claims.jti, user_id, ttl).await;
    }

    let mut headers = HeaderMap::new();
    set_auth_cookies(&mut headers, &session, &state.config);
    Ok((headers, Json(LoginResponse { user: session.user })))
}

pub async fn logout(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(claims): Extension<Claims>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    let all = payload
        .get("all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if all {
        auth_repo::delete_refresh_tokens_for_user(&state.write_pool, user.id)
            .await
            .map_err(|_| internal_error("Failed to revoke tokens"))?;
        auth_repo::delete_active_access_tokens_for_user(&state.write_pool, user.id)
            .await
            .map_err(|_| internal_error("Failed to revoke access tokens"))?;

        if let Some(cache) = &state.token_cache {
            let _ = cache.invalidate_user_tokens(user.id).await;
        }
        let mut response_headers = HeaderMap::new();
        clear_auth_cookies(&mut response_headers, &state.config);
        return Ok((
            response_headers,
            Json(json!({"message":"Logged out from all devices"})),
        ));
    }

    if let Some(rt_str) = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .or_else(|| {
            cookie_header_value(&headers)
                .and_then(|value| extract_cookie_value(value, REFRESH_COOKIE_NAME))
        })
    {
        if let Ok((token_id, _)) = decode_refresh_token(&rt_str) {
            auth_repo::delete_refresh_token_by_id(&state.write_pool, &token_id)
                .await
                .map_err(|_| internal_error("Failed to revoke token"))?;
        }
    }

    auth_repo::delete_active_access_token_by_jti(&state.write_pool, &claims.jti)
        .await
        .map_err(|_| internal_error("Failed to revoke access token"))?;

    if let Some(cache) = &state.token_cache {
        let _ = cache.invalidate_token(&claims.jti).await;
    }
    let mut response_headers = HeaderMap::new();
    clear_auth_cookies(&mut response_headers, &state.config);
    Ok((response_headers, Json(json!({"message":"Logged out"}))))
}

pub async fn me(Extension(user): Extension<User>) -> HandlerResult<Json<UserResponse>> {
    Ok(Json(UserResponse::from(user)))
}

pub async fn mfa_status(
    Extension(user): Extension<User>,
) -> HandlerResult<Json<MfaStatusResponse>> {
    Ok(Json(MfaStatusResponse {
        enabled: user.is_mfa_enabled(),
        pending: user.has_pending_mfa(),
    }))
}

pub async fn update_profile(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<UpdateProfile>,
) -> HandlerResult<Json<UserResponse>> {
    payload.validate()?;

    if let Some(ref email) = payload.email {
        let email_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND id != $2)",
        )
        .bind(email)
        .bind(user.id)
        .fetch_one(&state.write_pool)
        .await
        .map_err(|_| internal_error("Database error"))?;

        if email_exists {
            return Err(bad_request("Email already in use"));
        }
    }

    let full_name = payload.full_name.unwrap_or(user.full_name);
    let email = payload.email.unwrap_or(user.email);

    let updated_user = sqlx::query_as::<_, User>(
        "UPDATE users SET full_name = $1, email = $2, updated_at = NOW() \
         WHERE id = $3 \
         RETURNING id, username, password_hash, full_name, email, LOWER(role) as role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at",
    )
    .bind(&full_name)
    .bind(&email)
    .bind(user.id)
    .fetch_one(&state.write_pool)
    .await
    .map_err(|_| internal_error("Failed to update profile"))?;

    Ok(Json(UserResponse::from(updated_user)))
}

pub async fn mfa_register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
) -> HandlerResult<Json<MfaSetupResponse>> {
    crate::utils::security::verify_request_origin(&headers, &state.config)?;
    let response = begin_mfa_enrollment(&state.write_pool, &state.config, &user).await?;
    Ok(Json(response))
}

pub async fn mfa_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
) -> HandlerResult<Json<MfaSetupResponse>> {
    crate::utils::security::verify_request_origin(&headers, &state.config)?;
    let response = begin_mfa_enrollment(&state.write_pool, &state.config, &user).await?;
    Ok(Json(response))
}

pub async fn mfa_activate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
    Json(payload): Json<MfaCodeRequest>,
) -> HandlerResult<Json<Value>> {
    crate::utils::security::verify_request_origin(&headers, &state.config)?;
    let secret = user
        .mfa_secret
        .as_ref()
        .ok_or_else(|| bad_request("MFA setup not initiated"))?;

    let code = payload.code.trim().to_string();
    if !verify_totp_code(secret, &code).map_err(|_| internal_error("MFA verification error"))? {
        return Err(unauthorized("Invalid MFA code"));
    }

    let now = Utc::now();
    if sqlx::query("UPDATE users SET mfa_enabled_at = $1, updated_at = $1 WHERE id = $2")
        .bind(now)
        .bind(user.id.to_string())
        .execute(&state.write_pool)
        .await
        .is_err()
    {
        return Err(internal_error("Failed to enable MFA"));
    }

    auth_repo::delete_refresh_tokens_for_user(&state.write_pool, user.id)
        .await
        .map_err(|_| internal_error("Failed to revoke refresh tokens"))?;
    auth_repo::delete_active_access_tokens_for_user(&state.write_pool, user.id)
        .await
        .map_err(|_| internal_error("Failed to revoke access tokens"))?;

    if let Some(cache) = &state.token_cache {
        let _ = cache.invalidate_user_tokens(user.id).await;
    }

    Ok(Json(json!({"message": "MFA enabled"})))
}

pub async fn mfa_disable(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
    Json(payload): Json<MfaCodeRequest>,
) -> HandlerResult<Json<Value>> {
    crate::utils::security::verify_request_origin(&headers, &state.config)?;
    if !user.is_mfa_enabled() {
        return Err(bad_request("MFA is not enabled"));
    }

    let secret = user
        .mfa_secret
        .as_ref()
        .ok_or_else(|| internal_error("MFA secret missing"))?;

    let code = payload.code.trim().to_string();
    if !verify_totp_code(secret, &code).map_err(|_| internal_error("MFA verification error"))? {
        return Err(unauthorized("Invalid MFA code"));
    }

    if sqlx::query(
        "UPDATE users SET mfa_secret = NULL, mfa_enabled_at = NULL, updated_at = $1 WHERE id = $2",
    )
    .bind(Utc::now())
    .bind(user.id.to_string())
    .execute(&state.write_pool)
    .await
    .is_err()
    {
        return Err(internal_error("Failed to disable MFA"));
    }

    auth_repo::delete_refresh_tokens_for_user(&state.write_pool, user.id)
        .await
        .map_err(|_| internal_error("Failed to revoke refresh tokens"))?;
    auth_repo::delete_active_access_tokens_for_user(&state.write_pool, user.id)
        .await
        .map_err(|_| internal_error("Failed to revoke access tokens"))?;

    if let Some(cache) = &state.token_cache {
        let _ = cache.invalidate_user_tokens(user.id).await;
    }

    Ok(Json(json!({"message": "MFA disabled"})))
}

pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
    Json(payload): Json<ChangePasswordRequest>,
) -> HandlerResult<Json<Value>> {
    crate::utils::security::verify_request_origin(&headers, &state.config)?;
    payload.validate()?;
    if payload.new_password == payload.current_password {
        return Err(bad_request(
            "New password must differ from current password",
        ));
    }

    ensure_password_matches(
        &payload.current_password,
        &user.password_hash,
        "Current password is incorrect",
    )
    .await?;

    let new_hash = tokio::task::spawn_blocking({
        let password = payload.new_password.clone();
        move || hash_password(&password)
    })
    .await
    .map_err(|_| internal_error("Hashing task failed"))?
    .map_err(|_| internal_error("Failed to hash password"))?;

    auth_repo::update_user_password(&state.write_pool, user.id, &new_hash)
        .await
        .map_err(|_| internal_error("Failed to update password"))?;

    auth_repo::delete_refresh_tokens_for_user(&state.write_pool, user.id)
        .await
        .map_err(|_| internal_error("Failed to revoke sessions after password change"))?;
    auth_repo::delete_active_access_tokens_for_user(&state.write_pool, user.id)
        .await
        .map_err(|_| internal_error("Failed to revoke access tokens"))?;

    if let Some(cache) = &state.token_cache {
        let _ = cache.invalidate_user_tokens(user.id).await;
    }

    Ok(Json(json!({"message": "Password changed successfully"})))
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(payload): Json<RequestPasswordResetPayload>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    payload.validate()?;

    let user_opt = auth_repo::find_user_by_email(&state.write_pool, &payload.email)
        .await
        .map_err(|_| internal_error("Database error"))?;

    if let Some(user) = user_opt {
        let token = generate_token(32);
        password_reset_repo::create_password_reset(&state.write_pool, user.id, &token)
            .await
            .map_err(|_| internal_error("Failed to create password reset"))?;

        let email_service =
            EmailService::new().map_err(|_| internal_error("Email service error"))?;
        if let Err(e) = email_service.send_password_reset_email(&user.email, &token) {
            tracing::error!("Failed to send password reset email: {:?}", e);
        }
    }

    Ok(Json(json!({
        "message": "If the email exists, a password reset link has been sent"
    })))
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordPayload>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    payload.validate()?;

    let reset_record =
        password_reset_repo::find_valid_reset_by_token(&state.write_pool, &payload.token)
            .await
            .map_err(|_| internal_error("Database error"))?
            .ok_or_else(|| bad_request("Invalid or expired reset token"))?;

    let new_password_hash = tokio::task::spawn_blocking({
        let password = payload.new_password.clone();
        move || hash_password(&password)
    })
    .await
    .map_err(|_| internal_error("Task join error"))?
    .map_err(|_| internal_error("Failed to hash password"))?;

    let user = auth_repo::update_user_password(
        &state.write_pool,
        reset_record.user_id,
        &new_password_hash,
    )
    .await
    .map_err(|_| internal_error("Failed to update password"))?;

    password_reset_repo::mark_token_as_used(&state.write_pool, &reset_record.id)
        .await
        .map_err(|_| internal_error("Failed to mark token as used"))?;

    auth_repo::delete_all_refresh_tokens_for_user(&state.write_pool, user.id)
        .await
        .map_err(|_| internal_error("Failed to revoke refresh tokens"))?;

    auth_repo::delete_active_access_tokens_for_user(&state.write_pool, user.id)
        .await
        .map_err(|_| internal_error("Failed to revoke access tokens"))?;

    if let Some(cache) = &state.token_cache {
        let _ = cache.invalidate_user_tokens(user.id).await;
    }

    let email_service = EmailService::new().map_err(|_| internal_error("Email service error"))?;
    if let Err(e) = email_service.send_password_changed_notification(&user.email, &user.username) {
        tracing::error!("Failed to send password changed notification: {:?}", e);
    }

    Ok(Json(json!({
        "message": "Password has been reset successfully"
    })))
}

// Helper methods

async fn create_auth_session(user: &User, config: &Config) -> HandlerResult<AuthSession> {
    let (access_token, _) = create_access_token(
        user.id.to_string(),
        user.username.clone(),
        user.role.as_str().to_string(),
        &config.jwt_secret,
        config.jwt_expiration_hours,
    )
    .map_err(|_| internal_error("Failed to create access token"))?;

    let refresh_token =
        create_refresh_token(user.id.to_string(), config.refresh_token_expiration_days)
            .map_err(|_| internal_error("Failed to create refresh token"))?
            .encoded();

    Ok(AuthSession {
        access_token,
        refresh_token,
        user: UserResponse::from(user.clone()),
    })
}

async fn begin_mfa_enrollment(
    pool: &sqlx::PgPool,
    config: &Config,
    user: &User,
) -> HandlerResult<MfaSetupResponse> {
    if user.is_mfa_enabled() {
        return Err(bad_request("MFA already enabled"));
    }

    let secret = generate_totp_secret();
    let otpauth_url = generate_otpauth_uri(&config.mfa_issuer, &user.username, &secret)
        .map_err(|_| internal_error("Failed to issue MFA secret"))?;

    if sqlx::query(
        "UPDATE users SET mfa_secret = $1, mfa_enabled_at = NULL, updated_at = $2 WHERE id = $3",
    )
    .bind(&secret)
    .bind(Utc::now())
    .bind(user.id.to_string())
    .execute(pool)
    .await
    .is_err()
    {
        return Err(internal_error("Failed to persist MFA secret"));
    }

    Ok(MfaSetupResponse {
        secret,
        otpauth_url,
    })
}

pub async fn process_login_for_user<PF, AF, PFut, AFut>(
    user: User,
    payload: LoginRequest,
    config: &Config,
    persist_refresh_token: PF,
    persist_active_access_token: AF,
) -> HandlerResult<AuthSession>
where
    PF: FnOnce(RefreshToken) -> PFut,
    PFut: Future<Output = HandlerResult<()>>,
    AF: FnOnce(Claims, Option<String>) -> AFut,
    AFut: Future<Output = HandlerResult<()>>,
{
    ensure_password_matches(
        &payload.password,
        &user.password_hash,
        "Invalid username or password",
    )
    .await?;
    enforce_mfa(&user, payload.totp_code.as_deref())?;

    let (access_token, claims) = create_access_token(
        user.id.to_string(),
        user.username.clone(),
        user.role.as_str().to_string(),
        &config.jwt_secret,
        config.jwt_expiration_hours,
    )
    .map_err(|_| internal_error("Token creation error"))?;

    let refresh_token_data =
        create_refresh_token(user.id.to_string(), config.refresh_token_expiration_days)
            .map_err(|_| internal_error("Refresh token creation error"))?;
    let refresh_token = refresh_token_data.encoded();

    persist_refresh_token(refresh_token_data).await?;
    let context = payload
        .device_label
        .clone()
        .map(|label| label.trim().to_string());
    persist_active_access_token(claims, context).await?;

    let response = AuthSession {
        access_token,
        refresh_token,
        user: UserResponse::from(user),
    };

    Ok(response)
}

async fn persist_refresh_token(
    pool: &sqlx::PgPool,
    token: &RefreshToken,
    error_message: &'static str,
) -> HandlerResult<()> {
    auth_repo::insert_refresh_token(pool, token)
        .await
        .map_err(|_| internal_error(error_message))
}

async fn persist_active_access_token(
    pool: &sqlx::PgPool,
    claims: &Claims,
    context: Option<String>,
) -> HandlerResult<()> {
    let expires_at = DateTime::<Utc>::from_timestamp(claims.exp, 0)
        .ok_or_else(|| internal_error("Token expiration overflow"))?;

    auth_repo::cleanup_expired_access_tokens(pool)
        .await
        .map_err(|_| internal_error("Failed to cleanup expired tokens"))?;

    let sanitized_context = context.and_then(|ctx| {
        let trimmed = ctx.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.chars().take(128).collect::<String>())
        }
    });
    let user_id =
        UserId::from_str(&claims.sub).map_err(|_| internal_error("Invalid user ID in claims"))?;
    let token = ActiveAccessToken {
        jti: &claims.jti,
        user_id,
        expires_at,
        context: sanitized_context.as_deref(),
    };
    auth_repo::insert_active_access_token(pool, &token)
        .await
        .map_err(|_| internal_error("Failed to register access token"))
}

impl AuthSession {
    pub fn access_claims(&self, secret: &str) -> HandlerResult<Claims> {
        verify_access_token(&self.access_token, secret)
            .map_err(|_| internal_error("Failed to decode access token"))
    }

    pub fn refresh_token_data(&self, expiration_days: u64) -> HandlerResult<RefreshToken> {
        let (token_id, secret) = decode_refresh_token(&self.refresh_token)
            .map_err(|_| internal_error("Invalid refresh token format"))?;
        let token_hash = hash_refresh_token(&secret)
            .map_err(|_| internal_error("Failed to hash refresh token"))?;
        let expires_at = Utc::now()
            .checked_add_signed(chrono::Duration::days(expiration_days as i64))
            .ok_or_else(|| internal_error("Refresh token expiration overflow"))?;
        Ok(RefreshToken {
            id: token_id,
            user_id: self.user.id.to_string(),
            secret,
            token_hash,
            expires_at,
        })
    }
}

fn set_auth_cookies(headers: &mut HeaderMap, session: &AuthSession, config: &Config) {
    let options = CookieOptions {
        secure: config.cookie_secure,
        same_site: config.cookie_same_site,
    };
    let access_max_age = std::time::Duration::from_secs(config.jwt_expiration_hours * 3600);
    let refresh_max_age =
        std::time::Duration::from_secs(config.refresh_token_expiration_days * 24 * 60 * 60);
    let access_cookie = build_auth_cookie(
        ACCESS_COOKIE_NAME,
        &session.access_token,
        access_max_age,
        ACCESS_COOKIE_PATH,
        options,
    );
    let refresh_cookie = build_auth_cookie(
        REFRESH_COOKIE_NAME,
        &session.refresh_token,
        refresh_max_age,
        REFRESH_COOKIE_PATH,
        options,
    );
    headers.append(header::SET_COOKIE, access_cookie.parse().unwrap());
    headers.append(header::SET_COOKIE, refresh_cookie.parse().unwrap());
}

fn clear_auth_cookies(headers: &mut HeaderMap, config: &Config) {
    let options = CookieOptions {
        secure: config.cookie_secure,
        same_site: config.cookie_same_site,
    };
    let access_cookie = build_clear_cookie(ACCESS_COOKIE_NAME, ACCESS_COOKIE_PATH, options);
    let refresh_cookie = build_clear_cookie(REFRESH_COOKIE_NAME, REFRESH_COOKIE_PATH, options);
    headers.append(header::SET_COOKIE, access_cookie.parse().unwrap());
    headers.append(header::SET_COOKIE, refresh_cookie.parse().unwrap());
}

fn cookie_header_value(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::COOKIE).and_then(|v| v.to_str().ok())
}

pub async fn ensure_password_matches(
    candidate: &str,
    expected_hash: &str,
    unauthorized_message: &'static str,
) -> HandlerResult<()> {
    let candidate = candidate.to_owned();
    let expected_hash = expected_hash.to_owned();
    let matches = tokio::task::spawn_blocking(move || verify_password(&candidate, &expected_hash))
        .await
        .map_err(|_| internal_error("Password verification task failed"))?
        .map_err(|_| internal_error("Password verification error"))?;
    if matches {
        Ok(())
    } else {
        Err(unauthorized(unauthorized_message))
    }
}

pub fn enforce_mfa(user: &User, code: Option<&str>) -> HandlerResult<()> {
    if !user.is_mfa_enabled() {
        return Ok(());
    }
    let totp_code = code
        .map(|raw| {
            raw.chars()
                .filter(|ch| !ch.is_whitespace())
                .collect::<String>()
        })
        .filter(|code| !code.is_empty())
        .ok_or_else(|| unauthorized("MFA code required"))?;
    let secret = user
        .mfa_secret
        .as_ref()
        .ok_or_else(|| internal_error("MFA secret missing"))?;
    if verify_totp_code(secret, &totp_code).map_err(|_| internal_error("MFA verification error"))? {
        Ok(())
    } else {
        Err(unauthorized("Invalid MFA code"))
    }
}

fn bad_request(message: impl Into<String>) -> AppError {
    AppError::BadRequest(message.into())
}

fn unauthorized(message: impl Into<String>) -> AppError {
    AppError::Unauthorized(message.into())
}

fn internal_error(message: impl Into<String>) -> AppError {
    AppError::InternalServerError(anyhow::anyhow!(message.into()))
}
