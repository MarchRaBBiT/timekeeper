use axum::{
    extract::{Extension, State},
    http::{header, HeaderMap},
    Json,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::future::Future;
use std::str::FromStr;

use crate::{
    config::Config,
    error::AppError,
    handlers::auth_repo::{self, ActiveAccessToken, StoredRefreshToken},
    models::user::{
        ChangePasswordRequest, LoginRequest, LoginResponse, MfaCodeRequest, MfaSetupResponse,
        MfaStatusResponse, User, UserResponse,
    },
    types::UserId,
    utils::{
        cookies::{
            build_auth_cookie, build_clear_cookie, extract_cookie_value, CookieOptions,
            ACCESS_COOKIE_NAME, ACCESS_COOKIE_PATH, REFRESH_COOKIE_NAME, REFRESH_COOKIE_PATH,
        },
        jwt::{
            create_access_token, create_refresh_token, decode_refresh_token, verify_refresh_token,
            Claims, RefreshToken,
        },
        mfa::{generate_otpauth_uri, generate_totp_secret, verify_totp_code},
        password::{hash_password, verify_password},
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
    State((pool, config)): State<(PgPool, Config)>,
    Json(payload): Json<LoginRequest>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    payload.validate()?;

    let user = auth_repo::find_user_by_username(&pool, &payload.username)
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("Invalid username or password"))?;

    let session = process_login_for_user(
        user,
        payload,
        &config,
        {
            let pool = pool.clone();
            move |token| async move {
                persist_refresh_token(&pool, &token, "Failed to store refresh token").await
            }
        },
        {
            let pool = pool.clone();
            move |claims, context| async move {
                persist_active_access_token(&pool, &claims, context).await
            }
        },
    )
    .await?;

    let mut headers = HeaderMap::new();
    set_auth_cookies(&mut headers, &session, &config);
    Ok((headers, Json(LoginResponse { user: session.user })))
}

pub async fn refresh(
    State((pool, config)): State<(PgPool, Config)>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    let cookie_header = cookie_header_value(&headers);
    let refresh_token =
        extract_cookie_value(cookie_header.unwrap_or_default(), REFRESH_COOKIE_NAME)
            .or_else(|| {
                payload
                    .get("refresh_token")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string())
            })
            .ok_or_else(|| bad_request("Refresh token is required"))?;
    let (refresh_token_id, refresh_token_secret) = decode_refresh_token(&refresh_token)
        .map_err(|_| unauthorized("Invalid or expired refresh token"))?;

    let token_record = fetch_refresh_token_or_unauthorized(&pool, &refresh_token_id).await?;
    verify_refresh_secret(&refresh_token_secret, &token_record.token_hash).await?;

    let user = auth_repo::find_user_by_id(&pool, token_record.user_id)
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("User not found"))?;

    let (access_token, claims) = create_access_token(
        user.id.to_string(),
        user.username.clone(),
        user.role.as_str().to_string(),
        &config.jwt_secret,
        config.jwt_expiration_hours,
    )
    .map_err(|_| internal_error("Token creation error"))?;

    let new_refresh_token_data =
        create_refresh_token(user.id.to_string(), config.refresh_token_expiration_days)
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
    let device_context = payload
        .get("device_label")
        .and_then(|v| v.as_str())
        .or(Some("refresh"));
    persist_active_access_token(&pool, &claims, device_context.map(|s| s.to_string())).await?;
    if let Some(old_jti) = payload.get("previous_jti").and_then(|v| v.as_str()) {
        let _ = revoke_active_access_token(&pool, old_jti).await;
    }

    let session = AuthSession {
        access_token,
        refresh_token: new_refresh_token_data.encoded(),
        user: UserResponse::from(user),
    };

    let mut response_headers = HeaderMap::new();
    set_auth_cookies(&mut response_headers, &session, &config);
    Ok((response_headers, Json(LoginResponse { user: session.user })))
}

pub async fn mfa_status(
    Extension(user): Extension<User>,
) -> HandlerResult<Json<MfaStatusResponse>> {
    Ok(Json(MfaStatusResponse {
        enabled: user.is_mfa_enabled(),
        pending: user.has_pending_mfa(),
    }))
}

pub async fn me(Extension(user): Extension<User>) -> HandlerResult<Json<UserResponse>> {
    Ok(Json(UserResponse::from(user)))
}

async fn begin_mfa_enrollment(
    pool: &PgPool,
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

pub async fn mfa_setup(
    State((pool, config)): State<(PgPool, Config)>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
) -> HandlerResult<Json<MfaSetupResponse>> {
    crate::utils::security::verify_request_origin(&headers, &config)?;
    let response = begin_mfa_enrollment(&pool, &config, &user).await?;
    Ok(Json(response))
}

pub async fn mfa_register(
    State((pool, config)): State<(PgPool, Config)>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
) -> HandlerResult<Json<MfaSetupResponse>> {
    crate::utils::security::verify_request_origin(&headers, &config)?;
    let response = begin_mfa_enrollment(&pool, &config, &user).await?;
    Ok(Json(response))
}

pub async fn mfa_activate(
    State((pool, config)): State<(PgPool, Config)>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
    Json(payload): Json<MfaCodeRequest>,
) -> HandlerResult<Json<Value>> {
    crate::utils::security::verify_request_origin(&headers, &config)?;
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
        .execute(&pool)
        .await
        .is_err()
    {
        return Err(internal_error("Failed to enable MFA"));
    }

    revoke_tokens_for_user(&pool, user.id, "Failed to revoke refresh tokens").await?;
    revoke_active_tokens_for_user(&pool, user.id).await?;

    Ok(Json(json!({"message": "MFA enabled"})))
}

pub async fn mfa_disable(
    State((pool, config)): State<(PgPool, Config)>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
    Json(payload): Json<MfaCodeRequest>,
) -> HandlerResult<Json<Value>> {
    crate::utils::security::verify_request_origin(&headers, &config)?;
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
    .execute(&pool)
    .await
    .is_err()
    {
        return Err(internal_error("Failed to disable MFA"));
    }

    if sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user.id.to_string())
        .execute(&pool)
        .await
        .is_err()
    {
        return Err(internal_error("Failed to revoke refresh tokens"));
    }

    revoke_active_tokens_for_user(&pool, user.id).await?;

    Ok(Json(json!({"message": "MFA disabled"})))
}

pub async fn change_password(
    State((pool, config)): State<(PgPool, Config)>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
    Json(payload): Json<ChangePasswordRequest>,
) -> HandlerResult<Json<Value>> {
    crate::utils::security::verify_request_origin(&headers, &config)?;
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

    // Hash and persist the new password.
    let password_to_hash = payload.new_password.clone();
    let new_hash = tokio::task::spawn_blocking(move || hash_password(&password_to_hash))
        .await
        .map_err(|_| internal_error("Password hashing task failed"))?
        .map_err(|_| internal_error("Failed to hash password"))?;

    if sqlx::query("UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3")
        .bind(&new_hash)
        .bind(Utc::now())
        .bind(user.id.to_string())
        .execute(&pool)
        .await
        .is_err()
    {
        return Err(internal_error("Failed to update password"));
    }

    revoke_tokens_for_user(&pool, user.id, "Failed to revoke refresh tokens").await?;
    revoke_active_tokens_for_user(&pool, user.id).await?;

    Ok(Json(json!({"message": "Password updated successfully"})))
}

pub async fn logout(
    State((pool, config)): State<(PgPool, Config)>,
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
        revoke_tokens_for_user(&pool, user.id, "Failed to revoke tokens").await?;
        revoke_active_tokens_for_user(&pool, user.id).await?;
        let mut response_headers = HeaderMap::new();
        clear_auth_cookies(&mut response_headers, &config);
        return Ok((response_headers, Json(json!({"message":"Logged out"}))));
    }

    if let Some(rt) = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .or_else(|| {
            cookie_header_value(&headers)
                .and_then(|value| extract_cookie_value(value, REFRESH_COOKIE_NAME))
        })
    {
        let (token_id, _) =
            decode_refresh_token(&rt).map_err(|_| bad_request("Invalid refresh token"))?;
        revoke_refresh_token_for_user(&pool, &token_id, user.id).await?;
        revoke_active_access_token(&pool, &claims.jti).await?;
        let mut response_headers = HeaderMap::new();
        clear_auth_cookies(&mut response_headers, &config);
        return Ok((response_headers, Json(json!({"message":"Logged out"}))));
    }

    revoke_active_access_token(&pool, &claims.jti).await?;
    let mut response_headers = HeaderMap::new();
    clear_auth_cookies(&mut response_headers, &config);
    Ok((response_headers, Json(json!({"message":"Logged out"}))))
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
    persist_active_access_token(claims.clone(), context).await?;

    let response = AuthSession {
        access_token,
        refresh_token,
        user: UserResponse::from(user),
    };

    Ok(response)
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
    let access_value = access_cookie
        .parse()
        .expect("valid Set-Cookie header for access token");
    let refresh_value = refresh_cookie
        .parse()
        .expect("valid Set-Cookie header for refresh token");
    headers.append(header::SET_COOKIE, access_value);
    headers.append(header::SET_COOKIE, refresh_value);
}

fn clear_auth_cookies(headers: &mut HeaderMap, config: &Config) {
    let options = CookieOptions {
        secure: config.cookie_secure,
        same_site: config.cookie_same_site,
    };
    let access_cookie = build_clear_cookie(ACCESS_COOKIE_NAME, ACCESS_COOKIE_PATH, options);
    let refresh_cookie = build_clear_cookie(REFRESH_COOKIE_NAME, REFRESH_COOKIE_PATH, options);
    let access_value = access_cookie
        .parse()
        .expect("valid Set-Cookie header for access token");
    let refresh_value = refresh_cookie
        .parse()
        .expect("valid Set-Cookie header for refresh token");
    headers.append(header::SET_COOKIE, access_value);
    headers.append(header::SET_COOKIE, refresh_value);
}

fn cookie_header_value(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
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
    let valid = verify_totp_code(secret, &totp_code)
        .map_err(|_| internal_error("MFA verification error"))?;
    if valid {
        Ok(())
    } else {
        Err(unauthorized("Invalid MFA code"))
    }
}

async fn verify_refresh_secret(secret: &str, hash: &str) -> HandlerResult<()> {
    let secret = secret.to_owned();
    let hash = hash.to_owned();
    let valid = tokio::task::spawn_blocking(move || verify_refresh_token(&secret, &hash))
        .await
        .map_err(|_| internal_error("Refresh token verification task failed"))?
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
    user_id: UserId,
) -> HandlerResult<()> {
    auth_repo::delete_refresh_token_for_user(pool, token_id, user_id)
        .await
        .map_err(|_| internal_error("Failed to revoke token"))
}

async fn revoke_tokens_for_user(
    pool: &PgPool,
    user_id: UserId,
    error_message: &'static str,
) -> HandlerResult<()> {
    auth_repo::delete_refresh_tokens_for_user(pool, user_id)
        .await
        .map_err(|_| internal_error(error_message))
}

async fn persist_active_access_token(
    pool: &PgPool,
    claims: &Claims,
    context: Option<String>,
) -> HandlerResult<()> {
    let expires_at = claims_expiration_datetime(claims)?;
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

async fn revoke_active_access_token(pool: &PgPool, jti: &str) -> HandlerResult<()> {
    auth_repo::delete_active_access_token_by_jti(pool, jti)
        .await
        .map_err(|_| internal_error("Failed to revoke access token"))
}

async fn revoke_active_tokens_for_user(pool: &PgPool, user_id: UserId) -> HandlerResult<()> {
    auth_repo::delete_active_access_tokens_for_user(pool, user_id)
        .await
        .map_err(|_| internal_error("Failed to revoke access tokens"))
}

fn claims_expiration_datetime(claims: &Claims) -> HandlerResult<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(claims.exp, 0)
        .ok_or_else(|| internal_error("Token expiration overflow"))
}
