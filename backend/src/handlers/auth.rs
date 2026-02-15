use axum::{
    extract::{Extension, State},
    http::{header, header::USER_AGENT, HeaderMap},
    Json,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::future::Future;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    config::Config,
    error::AppError,
    middleware::request_id::RequestId,
    models::{
        active_session::ActiveSession,
        password_reset::{RequestPasswordResetPayload, ResetPasswordPayload},
        user::{
            ChangePasswordRequest, LoginRequest, LoginResponse, MfaCodeRequest, MfaSetupResponse,
            MfaStatusResponse, UpdateProfile, User, UserResponse,
        },
    },
    repositories::{
        active_session,
        auth::{self as auth_repo, ActiveAccessToken, LockoutPolicy},
        password_reset as password_reset_repo, user as user_repo,
    },
    services::audit_log::{AuditLogEntry, AuditLogServiceTrait},
    services::token_cache::TokenCacheServiceTrait,
    state::AppState,
    types::UserId,
    utils::{
        cookies::{
            build_auth_cookie, build_clear_cookie, extract_cookie_value, CookieOptions,
            ACCESS_COOKIE_NAME, ACCESS_COOKIE_PATH, REFRESH_COOKIE_NAME, REFRESH_COOKIE_PATH,
        },
        email::EmailService,
        encryption::{decrypt_pii, encrypt_pii, hash_email},
        jwt::{
            create_access_token, create_refresh_token, decode_refresh_token, hash_refresh_token,
            verify_access_token, verify_refresh_token, Claims, RefreshToken,
        },
        mfa::{
            generate_otpauth_uri, generate_totp_secret, protect_totp_secret, recover_totp_secret,
            verify_totp_code,
        },
        password::{
            hash_password, password_matches_any, validate_password_complexity, verify_password,
        },
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
    Extension(request_id): Extension<RequestId>,
    Extension(audit_log_service): Extension<Arc<dyn AuditLogServiceTrait>>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    payload.validate()?;

    let mut user = auth_repo::find_user_by_username(&state.write_pool, &payload.username)
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("Invalid username or password"))?;
    user.full_name = decrypt_pii(&user.full_name, &state.config)
        .map_err(|_| internal_error("Failed to decrypt user profile"))?;
    user.email = decrypt_pii(&user.email, &state.config)
        .map_err(|_| internal_error("Failed to decrypt user profile"))?;
    if let Some(secret) = user.mfa_secret.clone() {
        user.mfa_secret = Some(
            decrypt_pii(&secret, &state.config)
                .map_err(|_| internal_error("Failed to decrypt MFA secret"))?,
        );
    }

    let audit_context = AuditContext::new(Some(user.id), "user", &headers, Some(&request_id));
    let audit_actor_id = audit_context
        .actor_id
        .ok_or_else(|| internal_error("Missing audit actor ID"))?;
    let now = Utc::now();
    if user.locked_until.map(|until| until > now).unwrap_or(false) {
        record_login_audit_event(
            Some(audit_log_service.clone()),
            audit_context.clone(),
            "login_failure",
            user.id,
            Some(json!({ "reason": "account_locked" })),
        );
        return Err(unauthorized("Invalid username or password"));
    }

    let login_result = process_login_for_user(
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
        {
            let pool = state.write_pool.clone();
            let cache = state.token_cache.clone();
            let config = state.config.clone();
            let audit_log_service = audit_log_service.clone();
            let audit_context = audit_context.clone();
            move |user_id, refresh_token, claims, device_label| async move {
                register_active_session(
                    &pool,
                    RegisterSessionParams {
                        token_cache: cache.as_ref(),
                        audit_log_service: Some(audit_log_service.clone()),
                        audit_context: audit_context.clone(),
                        config: &config,
                        user_id,
                        refresh_token,
                        claims,
                        device_label,
                        source: "login",
                    },
                )
                .await
            }
        },
    )
    .await;

    let session = match login_result {
        Ok(session) => {
            auth_repo::clear_login_failures(&state.write_pool, session.user.id)
                .await
                .map_err(|_| internal_error("Failed to clear login failure count"))?;
            session
        }
        Err(err) => {
            if should_track_login_failure(&err) {
                let failure_state = auth_repo::record_login_failure(
                    &state.write_pool,
                    audit_actor_id,
                    now,
                    lockout_policy(&state.config),
                )
                .await
                .map_err(|_| internal_error("Failed to update login failure state"))?;

                record_login_audit_event(
                    Some(audit_log_service.clone()),
                    audit_context.clone(),
                    "login_failure",
                    audit_actor_id,
                    Some(json!({
                        "failed_login_attempts": failure_state.failed_login_attempts,
                        "lockout_count": failure_state.lockout_count,
                    })),
                );

                if failure_state.became_locked {
                    record_login_audit_event(
                        Some(audit_log_service.clone()),
                        audit_context.clone(),
                        "account_lockout",
                        audit_actor_id,
                        Some(json!({
                            "locked_until": failure_state.locked_until,
                        })),
                    );
                    if let Some(locked_until) = failure_state.locked_until {
                        let _ = send_lockout_notification(
                            &state.write_pool,
                            &state.config,
                            audit_actor_id,
                            locked_until,
                        )
                        .await;
                    }
                }
            }
            return Err(genericize_login_error(err));
        }
    };

    let mut headers = HeaderMap::new();
    set_auth_cookies(&mut headers, &session, &state.config);
    Ok((headers, Json(LoginResponse { user: session.user })))
}

pub async fn refresh(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(audit_log_service): Extension<Arc<dyn AuditLogServiceTrait>>,
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

    let mut user = auth_repo::find_user_by_id(&state.write_pool, stored.user_id)
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| unauthorized("User not found"))?;
    user.full_name = decrypt_pii(&user.full_name, &state.config)
        .map_err(|_| internal_error("Failed to decrypt user profile"))?;
    user.email = decrypt_pii(&user.email, &state.config)
        .map_err(|_| internal_error("Failed to decrypt user profile"))?;
    if let Some(secret) = user.mfa_secret.clone() {
        user.mfa_secret = Some(
            decrypt_pii(&secret, &state.config)
                .map_err(|_| internal_error("Failed to decrypt MFA secret"))?,
        );
    }

    enforce_password_expiration(&user, &state.config)?;

    let session = create_auth_session(&user, &state.config).await?;

    let rt_data = session.refresh_token_data(state.config.refresh_token_expiration_days)?;
    let claims = session.access_claims(&state.config.jwt_secret)?;
    let access_expires_at = DateTime::<Utc>::from_timestamp(claims.exp, 0)
        .ok_or_else(|| internal_error("Token expiration overflow"))?;
    let previous_session =
        active_session::find_active_session_by_refresh_token_id(&state.write_pool, &stored.id)
            .await
            .map_err(|_| internal_error("Failed to fetch active session"))?;
    let previous_device_label = sanitize_device_label(
        previous_session
            .as_ref()
            .and_then(|session| session.device_label.clone()),
    );
    let previous_access_jti = previous_session
        .as_ref()
        .and_then(|session| session.access_jti.clone());
    let audit_context = AuditContext::new(Some(user.id), "user", &headers, Some(&request_id));
    let mut tx = state
        .write_pool
        .begin()
        .await
        .map_err(|_| internal_error("Failed to start refresh token rotation transaction"))?;
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&rt_data.id)
    .bind(rt_data.user_id.to_string())
    .bind(&rt_data.token_hash)
    .bind(rt_data.expires_at)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error("Failed to store new refresh token"))?;

    sqlx::query(
        "INSERT INTO active_access_tokens (jti, user_id, expires_at, context) VALUES ($1, $2, $3, $4)",
    )
    .bind(&claims.jti)
    .bind(claims.sub.clone())
    .bind(access_expires_at)
    .bind(Some(format!("refresh_{}", stored.id)))
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error("Failed to store active access token"))?;

    let refreshed_session_id = if let Some(session_id) = sqlx::query_scalar::<_, String>(
        r#"
        UPDATE active_sessions
        SET refresh_token_id = $1,
            access_jti = $2,
            last_seen_at = $3,
            expires_at = $4
        WHERE refresh_token_id = $5
        RETURNING id
        "#,
    )
    .bind(&rt_data.id)
    .bind(&claims.jti)
    .bind(now)
    .bind(rt_data.expires_at)
    .bind(&stored.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error("Failed to update active session"))?
    {
        session_id
    } else {
        let session_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO active_sessions \
             (id, user_id, refresh_token_id, access_jti, device_label, last_seen_at, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&session_id)
        .bind(user.id)
        .bind(&rt_data.id)
        .bind(&claims.jti)
        .bind(previous_device_label.as_deref())
        .bind(now)
        .bind(rt_data.expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error("Failed to create active session"))?;
        session_id
    };

    let revoked_old = sqlx::query_scalar::<_, String>(
        "DELETE FROM refresh_tokens WHERE id = $1 AND token_hash = $2 RETURNING id",
    )
    .bind(&stored.id)
    .bind(&stored.token_hash)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error("Failed to revoke old refresh token"))?;

    if revoked_old.is_none() {
        return Err(unauthorized("Invalid or already-rotated refresh token"));
    }

    tx.commit()
        .await
        .map_err(|_| internal_error("Failed to commit refresh token rotation"))?;

    record_session_audit_event(
        Some(audit_log_service.clone()),
        audit_context.clone(),
        "session_create",
        Some(refreshed_session_id),
        Some(json!({
            "source": "refresh",
            "device_label": previous_device_label
        })),
    );

    enforce_session_limit(
        &state.write_pool,
        state.token_cache.as_ref(),
        Some(audit_log_service.clone()),
        audit_context,
        &state.config,
        user.id,
    )
    .await?;

    if let Some(access_jti) = previous_access_jti.as_deref() {
        if let Err(err) =
            auth_repo::delete_active_access_token_by_jti(&state.write_pool, access_jti).await
        {
            tracing::warn!(
                error = %err,
                access_jti = %access_jti,
                "Failed to revoke previous access token after successful refresh rotation"
            );
        }
        if let Some(cache) = &state.token_cache {
            let _ = cache.invalidate_token(access_jti).await;
        }
    }

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
    Extension(request_id): Extension<RequestId>,
    Extension(audit_log_service): Extension<Arc<dyn AuditLogServiceTrait>>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    let all = payload
        .get("all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if all {
        let sessions = active_session::list_active_sessions_for_user(&state.write_pool, user.id)
            .await
            .map_err(|_| internal_error("Failed to list active sessions"))?;
        auth_repo::delete_refresh_tokens_for_user(&state.write_pool, user.id)
            .await
            .map_err(|_| internal_error("Failed to revoke tokens"))?;
        auth_repo::delete_active_access_tokens_for_user(&state.write_pool, user.id)
            .await
            .map_err(|_| internal_error("Failed to revoke access tokens"))?;
        active_session::delete_active_sessions_for_user(&state.write_pool, user.id)
            .await
            .map_err(|_| internal_error("Failed to revoke active sessions"))?;

        if let Some(cache) = &state.token_cache {
            let _ = cache.invalidate_user_tokens(user.id).await;
        }
        let audit_context = AuditContext::new(Some(user.id), "user", &headers, Some(&request_id));
        for session in sessions {
            record_session_audit_event(
                Some(audit_log_service.clone()),
                audit_context.clone(),
                "session_destroy",
                Some(session.id),
                Some(json!({ "reason": "logout_all" })),
            );
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

    let session = active_session::find_active_session_by_access_jti(&state.write_pool, &claims.jti)
        .await
        .map_err(|_| internal_error("Failed to fetch active session"))?;

    auth_repo::delete_active_access_token_by_jti(&state.write_pool, &claims.jti)
        .await
        .map_err(|_| internal_error("Failed to revoke access token"))?;

    active_session::delete_active_session_by_access_jti(&state.write_pool, &claims.jti)
        .await
        .map_err(|_| internal_error("Failed to revoke active session"))?;

    if let Some(cache) = &state.token_cache {
        let _ = cache.invalidate_token(&claims.jti).await;
    }
    if let Some(session) = session {
        let audit_context = AuditContext::new(Some(user.id), "user", &headers, Some(&request_id));
        record_session_audit_event(
            Some(audit_log_service.clone()),
            audit_context,
            "session_destroy",
            Some(session.id),
            Some(json!({ "reason": "logout" })),
        );
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
        let email_hash = hash_email(email, &state.config);
        let email_exists = user_repo::email_exists_for_other_user(
            &state.write_pool,
            &email_hash,
            &user.id.to_string(),
        )
        .await
        .map_err(|_| internal_error("Database error"))?;

        if email_exists {
            return Err(bad_request("Email already in use"));
        }
    }

    let full_name = payload.full_name.unwrap_or(user.full_name);
    let email = payload.email.unwrap_or(user.email);
    let encrypted_full_name = encrypt_pii(&full_name, &state.config)
        .map_err(|_| internal_error("Failed to encrypt full_name"))?;
    let encrypted_email = encrypt_pii(&email, &state.config)
        .map_err(|_| internal_error("Failed to encrypt email"))?;
    let email_hash = hash_email(&email, &state.config);

    let updated_user = user_repo::update_profile(
        &state.write_pool,
        &user.id.to_string(),
        &encrypted_full_name,
        &encrypted_email,
        &email_hash,
    )
    .await
    .map_err(|_| internal_error("Failed to update profile"))?;
    let mut updated_user = updated_user;
    updated_user.full_name = decrypt_pii(&updated_user.full_name, &state.config)
        .map_err(|_| internal_error("Failed to decrypt user profile"))?;
    updated_user.email = decrypt_pii(&updated_user.email, &state.config)
        .map_err(|_| internal_error("Failed to decrypt user profile"))?;
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
        .ok_or_else(|| bad_request("MFA setup not initiated"))
        .and_then(|stored| {
            recover_totp_secret(stored, &state.config)
                .map_err(|_| internal_error("Failed to decrypt MFA secret"))
        })?;

    let code = payload.code.trim().to_string();
    if !verify_totp_code(&secret, &code).map_err(|_| internal_error("MFA verification error"))? {
        return Err(unauthorized("Invalid MFA code"));
    }

    let now = Utc::now();
    if !user_repo::enable_mfa(&state.write_pool, &user.id.to_string(), now)
        .await
        .map_err(|_| internal_error("Failed to enable MFA"))?
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
        .ok_or_else(|| internal_error("MFA secret missing"))
        .and_then(|stored| {
            recover_totp_secret(stored, &state.config)
                .map_err(|_| internal_error("Failed to decrypt MFA secret"))
        })?;

    let code = payload.code.trim().to_string();
    if !verify_totp_code(&secret, &code).map_err(|_| internal_error("MFA verification error"))? {
        return Err(unauthorized("Invalid MFA code"));
    }

    if !user_repo::disable_mfa(&state.write_pool, &user.id.to_string())
        .await
        .map_err(|_| internal_error("Failed to disable MFA"))?
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
    validate_password_complexity(&payload.new_password, &state.config)
        .map_err(|e| bad_request(e.to_string()))?;
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

    ensure_password_not_reused(
        &state.write_pool,
        user.id,
        &payload.new_password,
        &user.password_hash,
        state.config.password_history_count,
    )
    .await?;

    let new_hash = tokio::task::spawn_blocking({
        let password = payload.new_password.clone();
        move || hash_password(&password)
    })
    .await
    .map_err(|_| internal_error("Hashing task failed"))?
    .map_err(|_| internal_error("Failed to hash password"))?;

    auth_repo::update_user_password(
        &state.write_pool,
        user.id,
        &new_hash,
        &user.password_hash,
        state.config.password_history_count,
    )
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

    let email_hash = hash_email(&payload.email, &state.config);
    let user_opt = auth_repo::find_user_by_email_hash(&state.write_pool, &email_hash)
        .await
        .map_err(|_| internal_error("Database error"))?;

    if let Some(user) = user_opt {
        let resolved_email =
            decrypt_pii(&user.email, &state.config).unwrap_or_else(|_| payload.email.clone());
        let token = generate_token(32);
        password_reset_repo::create_password_reset(&state.write_pool, user.id, &token)
            .await
            .map_err(|_| internal_error("Failed to create password reset"))?;

        let email_service =
            EmailService::new().map_err(|_| internal_error("Email service error"))?;
        if let Err(e) = email_service.send_password_reset_email(&resolved_email, &token) {
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
    validate_password_complexity(&payload.new_password, &state.config)
        .map_err(|e| bad_request(e.to_string()))?;

    let reset_record =
        password_reset_repo::find_valid_reset_by_token(&state.write_pool, &payload.token)
            .await
            .map_err(|_| internal_error("Database error"))?
            .ok_or_else(|| bad_request("Invalid or expired reset token"))?;

    let mut user = auth_repo::find_user_by_id(&state.write_pool, reset_record.user_id)
        .await
        .map_err(|_| internal_error("Database error"))?
        .ok_or_else(|| bad_request("User not found"))?;
    user.full_name = decrypt_pii(&user.full_name, &state.config)
        .map_err(|_| internal_error("Failed to decrypt user profile"))?;
    user.email = decrypt_pii(&user.email, &state.config)
        .map_err(|_| internal_error("Failed to decrypt user profile"))?;

    ensure_password_not_reused(
        &state.write_pool,
        user.id,
        &payload.new_password,
        &user.password_hash,
        state.config.password_history_count,
    )
    .await?;

    let new_password_hash = tokio::task::spawn_blocking({
        let password = payload.new_password.clone();
        move || hash_password(&password)
    })
    .await
    .map_err(|_| internal_error("Task join error"))?
    .map_err(|_| internal_error("Failed to hash password"))?;

    let user = auth_repo::update_user_password(
        &state.write_pool,
        user.id,
        &new_password_hash,
        &user.password_hash,
        state.config.password_history_count,
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

    let protected_secret = protect_totp_secret(&secret, config)
        .map_err(|_| internal_error("Failed to encrypt MFA secret"))?;

    if !user_repo::set_mfa_secret(pool, &user.id.to_string(), &protected_secret, Utc::now())
        .await
        .map_err(|_| internal_error("Failed to persist MFA secret"))?
    {
        return Err(internal_error("Failed to persist MFA secret"));
    }

    Ok(MfaSetupResponse {
        secret,
        otpauth_url,
    })
}

pub async fn process_login_for_user<PF, AF, SF, PFut, AFut, SFut>(
    user: User,
    payload: LoginRequest,
    config: &Config,
    persist_refresh_token: PF,
    persist_active_access_token: AF,
    persist_active_session: SF,
) -> HandlerResult<AuthSession>
where
    PF: FnOnce(RefreshToken) -> PFut,
    PFut: Future<Output = HandlerResult<()>>,
    AF: FnOnce(Claims, Option<String>) -> AFut,
    AFut: Future<Output = HandlerResult<()>>,
    SF: FnOnce(UserId, RefreshToken, Claims, Option<String>) -> SFut,
    SFut: Future<Output = HandlerResult<()>>,
{
    ensure_password_matches(
        &payload.password,
        &user.password_hash,
        "Invalid username or password",
    )
    .await?;
    enforce_mfa(&user, payload.totp_code.as_deref(), config)?;
    enforce_password_expiration(&user, config)?;

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

    persist_refresh_token(refresh_token_data.clone()).await?;
    let context = payload
        .device_label
        .clone()
        .map(|label| label.trim().to_string());
    persist_active_access_token(claims.clone(), context.clone()).await?;
    persist_active_session(user.id, refresh_token_data, claims, context).await?;

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

fn sanitize_device_label(label: Option<String>) -> Option<String> {
    label.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.chars().take(128).collect::<String>())
        }
    })
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

    let sanitized_context = sanitize_device_label(context);
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

struct RegisterSessionParams<'a> {
    token_cache: Option<&'a Arc<dyn TokenCacheServiceTrait>>,
    audit_log_service: Option<Arc<dyn AuditLogServiceTrait>>,
    audit_context: AuditContext,
    config: &'a Config,
    user_id: UserId,
    refresh_token: RefreshToken,
    claims: Claims,
    device_label: Option<String>,
    source: &'static str,
}

async fn register_active_session(
    pool: &sqlx::PgPool,
    params: RegisterSessionParams<'_>,
) -> HandlerResult<()> {
    let RegisterSessionParams {
        token_cache,
        audit_log_service,
        audit_context,
        config,
        user_id,
        refresh_token,
        claims,
        device_label,
        source,
    } = params;
    let device_label = sanitize_device_label(device_label);
    let session = active_session::create_active_session(
        pool,
        user_id,
        &refresh_token.id,
        &claims.jti,
        device_label.as_deref(),
        refresh_token.expires_at,
    )
    .await
    .map_err(|_| internal_error("Failed to create active session"))?;

    record_session_audit_event(
        audit_log_service.clone(),
        audit_context.clone(),
        "session_create",
        Some(session.id),
        Some(json!({
            "source": source,
            "device_label": device_label
        })),
    );

    enforce_session_limit(
        pool,
        token_cache,
        audit_log_service,
        audit_context,
        config,
        user_id,
    )
    .await?;
    Ok(())
}

async fn enforce_session_limit(
    pool: &sqlx::PgPool,
    token_cache: Option<&Arc<dyn TokenCacheServiceTrait>>,
    audit_log_service: Option<Arc<dyn AuditLogServiceTrait>>,
    audit_context: AuditContext,
    config: &Config,
    user_id: UserId,
) -> HandlerResult<()> {
    let limit = config.max_concurrent_sessions as usize;
    if limit == 0 {
        return Ok(());
    }

    let sessions = active_session::list_active_sessions_for_user(pool, user_id)
        .await
        .map_err(|_| internal_error("Failed to list active sessions"))?;

    if sessions.len() <= limit {
        return Ok(());
    }

    for session in sessions.iter().skip(limit) {
        revoke_active_session(pool, token_cache, session).await?;
        record_session_audit_event(
            audit_log_service.clone(),
            audit_context.clone(),
            "session_destroy",
            Some(session.id.clone()),
            Some(json!({
                "reason": "max_concurrent_sessions",
                "limit": config.max_concurrent_sessions
            })),
        );
    }

    Ok(())
}

async fn revoke_active_session(
    pool: &sqlx::PgPool,
    token_cache: Option<&Arc<dyn TokenCacheServiceTrait>>,
    session: &ActiveSession,
) -> HandlerResult<()> {
    if let Some(access_jti) = session.access_jti.as_deref() {
        auth_repo::delete_active_access_token_by_jti(pool, access_jti)
            .await
            .map_err(|_| internal_error("Failed to revoke access token"))?;
        if let Some(cache) = token_cache {
            let _ = cache.invalidate_token(access_jti).await;
        }
    }

    auth_repo::delete_refresh_token_by_id(pool, &session.refresh_token_id)
        .await
        .map_err(|_| internal_error("Failed to revoke refresh token"))?;

    Ok(())
}

#[derive(Clone)]
struct AuditContext {
    actor_id: Option<UserId>,
    actor_type: String,
    ip: Option<String>,
    user_agent: Option<String>,
    request_id: Option<String>,
}

impl AuditContext {
    fn new(
        actor_id: Option<UserId>,
        actor_type: &str,
        headers: &HeaderMap,
        request_id: Option<&RequestId>,
    ) -> Self {
        Self {
            actor_id,
            actor_type: actor_type.to_string(),
            ip: extract_ip(headers),
            user_agent: extract_user_agent(headers),
            request_id: request_id.map(|id| id.0.clone()),
        }
    }
}

fn record_session_audit_event(
    audit_log_service: Option<Arc<dyn AuditLogServiceTrait>>,
    context: AuditContext,
    event_type: &'static str,
    session_id: Option<String>,
    metadata: Option<Value>,
) {
    let Some(audit_log_service) = audit_log_service else {
        return;
    };
    let entry = AuditLogEntry {
        occurred_at: Utc::now(),
        actor_id: context.actor_id,
        actor_type: context.actor_type,
        event_type: event_type.to_string(),
        target_type: Some("session".to_string()),
        target_id: session_id,
        result: "success".to_string(),
        error_code: None,
        metadata,
        ip: context.ip,
        user_agent: context.user_agent,
        request_id: context.request_id,
    };

    tokio::spawn(async move {
        if let Err(err) = audit_log_service.record_event(entry).await {
            tracing::warn!(
                error = ?err,
                event_type = %event_type,
                "Failed to record session audit log"
            );
        }
    });
}

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        return value
            .split(',')
            .next()
            .map(|ip| ip.trim().to_string())
            .filter(|ip| !ip.is_empty());
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|agent| agent.trim().to_string())
        .filter(|agent| !agent.is_empty())
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

fn bad_request(message: impl Into<String>) -> AppError {
    AppError::BadRequest(message.into())
}

fn unauthorized(message: impl Into<String>) -> AppError {
    AppError::Unauthorized(message.into())
}

fn internal_error(message: impl Into<String>) -> AppError {
    AppError::InternalServerError(anyhow::anyhow!(message.into()))
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

pub fn enforce_mfa(user: &User, code: Option<&str>, config: &Config) -> HandlerResult<()> {
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
        .ok_or_else(|| internal_error("MFA secret missing"))
        .and_then(|stored| {
            recover_totp_secret(stored, config)
                .map_err(|_| internal_error("Failed to decrypt MFA secret"))
        })?;
    if verify_totp_code(&secret, &totp_code)
        .map_err(|_| internal_error("MFA verification error"))?
    {
        Ok(())
    } else {
        Err(unauthorized("Invalid MFA code"))
    }
}

fn enforce_password_expiration(user: &User, config: &Config) -> HandlerResult<()> {
    if config.password_expiration_days == 0 {
        return Ok(());
    }
    let expiry = user
        .password_changed_at
        .checked_add_signed(chrono::Duration::days(
            config.password_expiration_days as i64,
        ))
        .ok_or_else(|| internal_error("Password expiration overflow"))?;
    if Utc::now() > expiry {
        Err(unauthorized("Password expired"))
    } else {
        Ok(())
    }
}

fn lockout_policy(config: &Config) -> LockoutPolicy {
    LockoutPolicy {
        threshold: config.account_lockout_threshold as i32,
        duration_minutes: config.account_lockout_duration_minutes as i64,
        backoff_enabled: config.account_lockout_backoff_enabled,
        max_duration_hours: config.account_lockout_max_duration_hours as i64,
    }
}

fn should_track_login_failure(err: &AppError) -> bool {
    match err {
        AppError::Unauthorized(message) => message != "Password expired",
        _ => false,
    }
}

fn genericize_login_error(err: AppError) -> AppError {
    match err {
        AppError::Unauthorized(_) => unauthorized("Invalid username or password"),
        other => other,
    }
}

fn record_login_audit_event(
    audit_log_service: Option<Arc<dyn AuditLogServiceTrait>>,
    context: AuditContext,
    event_type: &'static str,
    user_id: UserId,
    metadata: Option<Value>,
) {
    let Some(audit_log_service) = audit_log_service else {
        return;
    };
    let entry = AuditLogEntry {
        occurred_at: Utc::now(),
        actor_id: context.actor_id,
        actor_type: context.actor_type,
        event_type: event_type.to_string(),
        target_type: Some("user".to_string()),
        target_id: Some(user_id.to_string()),
        result: "success".to_string(),
        error_code: None,
        metadata,
        ip: context.ip,
        user_agent: context.user_agent,
        request_id: context.request_id,
    };
    tokio::spawn(async move {
        if let Err(err) = audit_log_service.record_event(entry).await {
            tracing::warn!(
                error = ?err,
                event_type = %event_type,
                "Failed to record login audit log"
            );
        }
    });
}

async fn send_lockout_notification(
    pool: &sqlx::PgPool,
    config: &Config,
    user_id: UserId,
    locked_until: DateTime<Utc>,
) -> HandlerResult<()> {
    let Some(mut user) = auth_repo::find_user_by_id(pool, user_id)
        .await
        .map_err(|_| internal_error("Failed to load user for lockout notification"))?
    else {
        return Ok(());
    };
    user.email = decrypt_pii(&user.email, config)
        .map_err(|_| internal_error("Failed to decrypt lockout email"))?;

    let email_service = EmailService::new().map_err(|_| internal_error("Email service error"))?;
    if let Err(err) =
        email_service.send_account_lockout_notification(&user.email, &user.username, locked_until)
    {
        tracing::warn!(error = ?err, user_id = %user.id, "Failed to send lockout notification");
    }
    Ok(())
}

async fn ensure_password_not_reused(
    pool: &sqlx::PgPool,
    user_id: UserId,
    candidate: &str,
    current_hash: &str,
    history_limit: u32,
) -> HandlerResult<()> {
    if history_limit == 0 {
        return Ok(());
    }
    let history_hashes = auth_repo::fetch_recent_password_hashes(pool, user_id, history_limit)
        .await
        .map_err(|_| internal_error("Failed to load password history"))?;
    let candidate = candidate.to_owned();
    let current_hash = current_hash.to_owned();
    let reused = tokio::task::spawn_blocking(move || {
        let mut hashes = history_hashes;
        hashes.push(current_hash);
        password_matches_any(&candidate, &hashes)
    })
    .await
    .map_err(|_| internal_error("Password reuse check failed"))?
    .map_err(|_| internal_error("Password reuse check error"))?;

    if reused {
        Err(bad_request("Password was used recently"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header, HeaderMap};
    use base32::Alphabet::RFC4648;
    use chrono::{Duration, Utc};
    use chrono_tz::UTC;
    use sqlx::postgres::PgPoolOptions;
    use std::time::{SystemTime, UNIX_EPOCH};
    use totp_rs::{Algorithm, TOTP};

    fn config_stub() -> Config {
        Config {
            database_url: "postgres://127.0.0.1/timekeeper_test".to_string(),
            read_database_url: None,
            jwt_secret: "0123456789abcdef0123456789abcdef".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            max_concurrent_sessions: 3,
            audit_log_retention_days: 1825,
            audit_log_retention_forever: false,
            consent_log_retention_days: 1825,
            consent_log_retention_forever: false,
            aws_region: "ap-northeast-1".into(),
            aws_kms_key_id: "alias/timekeeper-test".into(),
            aws_audit_log_bucket: "timekeeper-audit-logs".into(),
            aws_cloudtrail_enabled: true,
            cookie_secure: true,
            cookie_same_site: crate::utils::cookies::SameSite::Lax,
            cors_allow_origins: vec!["http://localhost:8000".into()],
            time_zone: UTC,
            mfa_issuer: "Timekeeper".into(),
            rate_limit_ip_max_requests: 15,
            rate_limit_ip_window_seconds: 900,
            rate_limit_user_max_requests: 20,
            rate_limit_user_window_seconds: 3600,
            redis_url: None,
            redis_pool_size: 5,
            redis_connect_timeout: 5,
            feature_redis_cache_enabled: true,
            feature_read_replica_enabled: true,
            password_min_length: 12,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_numbers: true,
            password_require_symbols: true,
            password_expiration_days: 30,
            password_history_count: 5,
            account_lockout_threshold: 5,
            account_lockout_duration_minutes: 15,
            account_lockout_backoff_enabled: true,
            account_lockout_max_duration_hours: 24,
            production_mode: false,
        }
    }

    fn build_user() -> User {
        let password_hash = hash_password("Secret123!").expect("hash password for tests");
        User::new(
            "tester".into(),
            password_hash,
            "Test User".into(),
            "test@example.com".into(),
            crate::models::user::UserRole::Employee,
            false,
        )
    }

    fn current_totp_code(secret: &str) -> String {
        let cleaned = secret.trim().replace(' ', "").to_uppercase();
        let secret_bytes =
            base32::decode(RFC4648 { padding: false }, cleaned.as_str()).expect("decode secret");
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            None,
            "".to_string(),
        )
        .expect("build totp");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_secs();
        totp.generate(now)
    }

    #[tokio::test]
    async fn ensure_password_matches_accepts_correct_password() {
        let user = build_user();
        ensure_password_matches("Secret123!", &user.password_hash, "invalid")
            .await
            .expect("password should match");
    }

    #[tokio::test]
    async fn ensure_password_matches_rejects_wrong_password() {
        let user = build_user();
        let err = ensure_password_matches("wrong", &user.password_hash, "invalid")
            .await
            .expect_err("should fail");
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[test]
    fn enforce_mfa_accepts_valid_code() {
        let mut user = build_user();
        let config = config_stub();
        let secret = generate_totp_secret();
        user.mfa_secret = Some(secret.clone());
        user.mfa_enabled_at = Some(Utc::now());
        let code = current_totp_code(&secret);
        enforce_mfa(&user, Some(&code), &config).expect("valid mfa code should pass");
    }

    #[test]
    fn enforce_mfa_rejects_missing_code() {
        let mut user = build_user();
        let config = config_stub();
        user.mfa_secret = Some(generate_totp_secret());
        user.mfa_enabled_at = Some(Utc::now());
        let err = enforce_mfa(&user, None, &config).expect_err("code required");
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[test]
    fn enforce_password_expiration_behaves_as_expected() {
        let mut user = build_user();
        let mut config = config_stub();
        config.password_expiration_days = 1;
        user.password_changed_at = Utc::now() - Duration::days(2);
        assert!(enforce_password_expiration(&user, &config).is_err());
        user.password_changed_at = Utc::now();
        assert!(enforce_password_expiration(&user, &config).is_ok());
        config.password_expiration_days = 0;
        assert!(enforce_password_expiration(&user, &config).is_ok());
    }

    #[tokio::test]
    async fn ensure_password_not_reused_allows_history_zero() {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://127.0.0.1:1/timekeeper")
            .expect("create lazy pool");
        ensure_password_not_reused(&pool, UserId::new(), "candidate", "hash", 0)
            .await
            .expect("history limit zero should skip db");
    }

    #[tokio::test]
    async fn auth_session_exposes_claims_and_refresh_token_data() {
        let mut user = build_user();
        user.mfa_enabled_at = Some(Utc::now());
        user.mfa_secret = Some(generate_totp_secret());
        let config = config_stub();
        let session = create_auth_session(&user, &config)
            .await
            .expect("create session");
        let claims = session
            .access_claims(&config.jwt_secret)
            .expect("claims decode");
        assert_eq!(claims.sub, user.id.to_string());
        let refresh_data = session
            .refresh_token_data(config.refresh_token_expiration_days)
            .expect("refresh data");
        assert_eq!(refresh_data.user_id, user.id.to_string());
    }

    #[test]
    fn sanitize_device_label_trims_and_limits_length() {
        assert!(sanitize_device_label(Some("   ".into())).is_none());
        assert_eq!(
            sanitize_device_label(Some("foo".into())),
            Some("foo".to_string())
        );
        let long = "x".repeat(200);
        assert_eq!(
            sanitize_device_label(Some(long.clone())),
            Some(long.chars().take(128).collect())
        );
    }

    #[test]
    fn extract_ip_prefers_forwarded_for_and_falls_back_to_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", " 1.2.3.4 , 5.6.7.8 ".parse().unwrap());
        headers.insert("x-real-ip", "9.9.9.9".parse().unwrap());
        assert_eq!(extract_ip(&headers), Some("1.2.3.4".to_string()));
        headers.remove("x-forwarded-for");
        assert_eq!(extract_ip(&headers), Some("9.9.9.9".to_string()));
    }

    #[test]
    fn extract_user_agent_trims_value() {
        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, "  TestAgent/1.0 ".parse().unwrap());
        assert_eq!(
            extract_user_agent(&headers),
            Some("TestAgent/1.0".to_string())
        );
    }

    #[test]
    fn cookie_header_value_reads_cookie_string() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, "foo=bar; baz=qux".parse().unwrap());
        assert_eq!(cookie_header_value(&headers), Some("foo=bar; baz=qux"));
    }

    #[tokio::test]
    async fn set_and_clear_auth_cookies_append_expected_headers() {
        let mut headers = HeaderMap::new();
        let user = build_user();
        let config = config_stub();
        let session = create_auth_session(&user, &config)
            .await
            .expect("create session");
        set_auth_cookies(&mut headers, &session, &config);
        assert_eq!(headers.get_all(header::SET_COOKIE).iter().count(), 2);
        clear_auth_cookies(&mut headers, &config);
        assert_eq!(headers.get_all(header::SET_COOKIE).iter().count(), 4);
    }
}
