use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use serde_json::Value;
use std::sync::Arc;

use crate::{
    config::Config,
    error::AppError,
    identity::application::auth::{
        activate_mfa as activate_mfa_use_case,
        begin_mfa_enrollment as begin_mfa_enrollment_use_case,
        change_password as change_password_use_case, clear_auth_cookies,
        disable_mfa as disable_mfa_use_case, login as login_use_case, logout as logout_use_case,
        refresh as refresh_use_case, request_password_reset as request_password_reset_use_case,
        reset_password as reset_password_use_case, set_auth_cookies,
        update_profile as update_profile_use_case, AuthSession,
    },
    middleware::request_id::RequestId,
    models::{
        password_reset::{RequestPasswordResetPayload, ResetPasswordPayload},
        user::{
            ChangePasswordRequest, LoginRequest, LoginResponse, MfaCodeRequest, MfaSetupResponse,
            MfaStatusResponse, UpdateProfile, User, UserResponse,
        },
    },
    services::audit_log::AuditLogServiceTrait,
    state::AppState,
    utils::security::verify_request_origin,
    validation::Validate,
};

pub use crate::identity::application::auth::{
    cookie_header_value, create_auth_session, enforce_mfa, enforce_password_expiration,
    ensure_password_matches, ensure_password_not_reused, extract_ip, extract_user_agent,
    lockout_policy, login_audit_result, parse_ip_from_header_value, process_login_for_user,
    sanitize_device_label,
};

pub type HandlerResult<T> = Result<T, AppError>;

pub async fn login(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(audit_log_service): Extension<Arc<dyn AuditLogServiceTrait>>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    payload.validate()?;
    let session = login_use_case(
        &state.write_pool,
        state.token_cache.as_ref(),
        &state.config,
        &request_id,
        Some(audit_log_service),
        &headers,
        payload,
    )
    .await?;

    Ok(build_login_response(session, &state.config))
}

pub async fn refresh(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(audit_log_service): Extension<Arc<dyn AuditLogServiceTrait>>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    let refresh_token = crate::utils::cookies::extract_cookie_value(
        headers
            .get(axum::http::header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default(),
        crate::utils::cookies::REFRESH_COOKIE_NAME,
    )
    .or_else(|| {
        payload
            .get("refresh_token")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
    })
    .ok_or_else(|| AppError::BadRequest("Refresh token is required".into()))?;

    let session = refresh_use_case(
        &state.write_pool,
        state.token_cache.as_ref(),
        &state.config,
        &request_id,
        Some(audit_log_service),
        &headers,
        &refresh_token,
    )
    .await?;

    Ok(build_login_response(session, &state.config))
}

pub async fn logout(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(claims): Extension<crate::utils::jwt::Claims>,
    Extension(request_id): Extension<RequestId>,
    Extension(audit_log_service): Extension<Arc<dyn AuditLogServiceTrait>>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    let all = payload
        .get("all")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let refresh_token = payload
        .get("refresh_token")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| {
            headers
                .get(axum::http::header::COOKIE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| {
                    crate::utils::cookies::extract_cookie_value(
                        value,
                        crate::utils::cookies::REFRESH_COOKIE_NAME,
                    )
                })
        });

    let result = logout_use_case(
        &state.write_pool,
        state.token_cache.as_ref(),
        &user,
        &claims,
        &state.config,
        &request_id,
        Some(audit_log_service),
        &headers,
        all,
        refresh_token.as_deref(),
    )
    .await?;

    let mut response_headers = HeaderMap::new();
    clear_auth_cookies(&mut response_headers, &state.config);
    Ok((response_headers, Json(result)))
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
    Ok(Json(
        update_profile_use_case(&state.write_pool, &state.config, &user, payload).await?,
    ))
}

pub async fn mfa_register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
) -> HandlerResult<Json<MfaSetupResponse>> {
    verify_request_origin(&headers, &state.config)?;
    Ok(Json(
        begin_mfa_enrollment_use_case(&state.write_pool, &state.config, &user).await?,
    ))
}

pub async fn mfa_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
) -> HandlerResult<Json<MfaSetupResponse>> {
    verify_request_origin(&headers, &state.config)?;
    Ok(Json(
        begin_mfa_enrollment_use_case(&state.write_pool, &state.config, &user).await?,
    ))
}

pub async fn mfa_activate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
    Json(payload): Json<MfaCodeRequest>,
) -> HandlerResult<Json<Value>> {
    verify_request_origin(&headers, &state.config)?;
    Ok(Json(
        activate_mfa_use_case(
            &state.write_pool,
            state.token_cache.as_ref(),
            &state.config,
            &user,
            payload,
        )
        .await?,
    ))
}

pub async fn mfa_disable(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
    Json(payload): Json<MfaCodeRequest>,
) -> HandlerResult<Json<Value>> {
    verify_request_origin(&headers, &state.config)?;
    Ok(Json(
        disable_mfa_use_case(
            &state.write_pool,
            state.token_cache.as_ref(),
            &state.config,
            &user,
            payload,
        )
        .await?,
    ))
}

pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<User>,
    Json(payload): Json<ChangePasswordRequest>,
) -> HandlerResult<Json<Value>> {
    verify_request_origin(&headers, &state.config)?;
    payload.validate()?;
    Ok(Json(
        change_password_use_case(
            &state.write_pool,
            state.token_cache.as_ref(),
            &state.config,
            &user,
            payload,
        )
        .await?,
    ))
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(payload): Json<RequestPasswordResetPayload>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    payload.validate()?;
    Ok(Json(
        request_password_reset_use_case(&state.write_pool, &state.config, payload).await?,
    ))
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordPayload>,
) -> HandlerResult<impl axum::response::IntoResponse> {
    payload.validate()?;
    Ok(Json(
        reset_password_use_case(
            &state.write_pool,
            state.token_cache.as_ref(),
            &state.config,
            payload,
        )
        .await?,
    ))
}

fn build_login_response(session: AuthSession, config: &Config) -> (HeaderMap, Json<LoginResponse>) {
    let mut headers = HeaderMap::new();
    set_auth_cookies(&mut headers, &session, config);
    (headers, Json(LoginResponse { user: session.user }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::{models::user::UserRole, types::UserId};

    fn sample_user() -> User {
        let now = Utc::now();
        User {
            id: UserId::new(),
            username: "alice".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            role: UserRole::Employee,
            is_system_admin: false,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            failed_login_attempts: 0,
            locked_until: None,
            lock_reason: None,
            lockout_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    fn config_stub() -> Config {
        use chrono_tz::UTC;

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
            cookie_secure: false,
            cookie_same_site: crate::utils::cookies::SameSite::Lax,
            cors_allow_origins: vec!["http://localhost:3000".to_string()],
            time_zone: UTC,
            mfa_issuer: "Timekeeper".into(),
            rate_limit_ip_max_requests: 10,
            rate_limit_ip_window_seconds: 60,
            rate_limit_user_max_requests: 30,
            rate_limit_user_window_seconds: 60,
            redis_url: None,
            redis_pool_size: 4,
            redis_connect_timeout: 5,
            feature_redis_cache_enabled: false,
            feature_read_replica_enabled: false,
            password_min_length: 12,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_numbers: true,
            password_require_symbols: true,
            password_expiration_days: 90,
            password_history_count: 5,
            account_lockout_threshold: 5,
            account_lockout_duration_minutes: 15,
            account_lockout_backoff_enabled: true,
            account_lockout_max_duration_hours: 24,
            production_mode: false,
        }
    }

    #[tokio::test]
    async fn build_login_response_sets_auth_cookies() {
        let session =
            crate::identity::application::auth::create_auth_session(&sample_user(), &config_stub())
                .await
                .expect("auth session");

        let (headers, Json(body)) = build_login_response(session, &config_stub());

        assert_eq!(body.user.username, "alice");
        assert_eq!(
            headers
                .get_all(axum::http::header::SET_COOKIE)
                .iter()
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn mfa_status_reflects_user_flags() {
        let mut user = sample_user();
        user.mfa_secret = Some("pending-secret".to_string());

        let Json(status) = mfa_status(Extension(user)).await.expect("mfa status");

        assert!(!status.enabled);
        assert!(status.pending);
    }
}
