use axum::http::{header::USER_AGENT, HeaderMap};
use serde::Serialize;
use serde_json::json;
use std::str::FromStr;
use std::sync::Arc;
use validator::Validate;

use crate::{
    application::clock::{Clock, SYSTEM_CLOCK},
    config::Config,
    error::AppError,
    middleware::request_id::RequestId,
    models::user::{CreateUser, UpdateUser, User, UserResponse},
    repositories::{auth as auth_repo, user as user_repo},
    services::audit_log::{AuditLogEntry, AuditLogServiceTrait},
    types::UserId,
    utils::{
        encryption::{decrypt_pii, encrypt_pii, hash_email},
        password::{hash_password, validate_password_complexity},
        pii::{mask_email, mask_name},
    },
};

#[derive(Debug, Serialize)]
pub struct UserListResult {
    pub users: Vec<UserResponse>,
    pub pii_masked: bool,
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

#[derive(Debug, Serialize)]
pub struct UserActionMessage {
    pub message: String,
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteUserMessage {
    pub message: String,
    pub user_id: String,
    pub deletion_type: String,
}

pub async fn get_users(
    read_pool: &sqlx::PgPool,
    config: &Config,
    requester: &User,
) -> Result<UserListResult, AppError> {
    ensure_admin_or_system(requester)?;

    let users = user_repo::list_users(read_pool)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let users = users
        .into_iter()
        .map(|user| decrypt_user(config, user))
        .map(|user| user.map(UserResponse::from))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|response| maybe_mask_user_response(requester, response))
        .collect();

    Ok(UserListResult {
        users,
        pii_masked: !requester.is_system_admin(),
    })
}

pub async fn create_user(
    write_pool: &sqlx::PgPool,
    config: &Config,
    requester: &User,
    payload: CreateUser,
) -> Result<UserResponse, AppError> {
    ensure_system_admin(requester)?;
    payload.validate()?;
    validate_password_complexity(&payload.password, config)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if auth_repo::find_user_by_username(write_pool, &payload.username)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .is_some()
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

    let encrypted_full_name = encrypt_pii(&payload.full_name, config).map_err(|_| {
        AppError::InternalServerError(anyhow::anyhow!("Failed to encrypt full_name"))
    })?;
    let encrypted_email = encrypt_pii(&payload.email, config)
        .map_err(|_| AppError::InternalServerError(anyhow::anyhow!("Failed to encrypt email")))?;
    let email_hash = hash_email(&payload.email, config);

    let user = User::new(
        payload.username,
        password_hash,
        encrypted_full_name,
        encrypted_email,
        payload.role,
        payload.is_system_admin,
    );

    let created = user_repo::create_user(write_pool, &user, &email_hash)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(UserResponse::from(decrypt_user(config, created)?))
}

pub async fn update_user(
    write_pool: &sqlx::PgPool,
    config: &Config,
    requester: &User,
    user_id: &str,
    payload: UpdateUser,
) -> Result<UserResponse, AppError> {
    ensure_system_admin(requester)?;
    payload.validate()?;

    let user_id_obj = parse_user_id(user_id)?;
    let mut existing_user = auth_repo::find_user_by_id(write_pool, user_id_obj)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;
    existing_user = decrypt_user_best_effort(config, existing_user);

    if let Some(ref email) = payload.email {
        let email_hash = hash_email(email, config);
        let email_exists = user_repo::email_exists_for_other_user(
            write_pool,
            &email_hash,
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
    let encrypted_full_name = encrypt_pii(&full_name, config).map_err(|_| {
        AppError::InternalServerError(anyhow::anyhow!("Failed to encrypt full_name"))
    })?;
    let encrypted_email = encrypt_pii(&email, config)
        .map_err(|_| AppError::InternalServerError(anyhow::anyhow!("Failed to encrypt email")))?;
    let email_hash = hash_email(&email, config);
    let role = payload.role.unwrap_or(existing_user.role);
    let is_system_admin = payload
        .is_system_admin
        .unwrap_or(existing_user.is_system_admin);

    let updated_user = user_repo::update_user(
        write_pool,
        user_id,
        &encrypted_full_name,
        &encrypted_email,
        &email_hash,
        role,
        is_system_admin,
    )
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(UserResponse::from(decrypt_user(config, updated_user)?))
}

pub async fn reset_user_mfa(
    write_pool: &sqlx::PgPool,
    requester: &User,
    user_id: &str,
    request_id: Option<RequestId>,
    audit_log_service: Option<Arc<dyn AuditLogServiceTrait>>,
    headers: &HeaderMap,
) -> Result<UserActionMessage, AppError> {
    ensure_system_admin_with_message(requester, "Only system administrators can reset MFA")?;

    let parsed_user_id = parse_user_id(user_id)?;
    let success =
        user_repo::reset_mfa_and_revoke_refresh_tokens(write_pool, parsed_user_id).await?;

    if !success {
        return Err(AppError::NotFound("User not found".into()));
    }

    spawn_audit_log(
        audit_log_service,
        build_audit_entry(
            requester,
            user_id,
            "mfa_reset",
            json!({ "reason": "admin_reset" }),
            headers,
            request_id,
        ),
        "Failed to record MFA reset audit log",
    );

    Ok(UserActionMessage {
        message: "MFA reset and refresh tokens revoked".to_string(),
        user_id: user_id.to_string(),
    })
}

pub async fn unlock_user_account(
    write_pool: &sqlx::PgPool,
    requester: &User,
    user_id: &str,
    request_id: Option<RequestId>,
    audit_log_service: Option<Arc<dyn AuditLogServiceTrait>>,
    headers: &HeaderMap,
) -> Result<UserActionMessage, AppError> {
    ensure_system_admin_with_message(
        requester,
        "Only system administrators can unlock user accounts",
    )?;

    let user_id_obj = parse_user_id(user_id)?;
    let unlocked = auth_repo::unlock_user_account(write_pool, user_id_obj)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if !unlocked {
        return Err(AppError::NotFound("User not found".into()));
    }

    spawn_audit_log(
        audit_log_service,
        build_audit_entry(
            requester,
            user_id,
            "account_unlock",
            json!({ "reason": "manual_unlock" }),
            headers,
            request_id,
        ),
        "Failed to record account unlock audit log",
    );

    Ok(UserActionMessage {
        message: "User unlocked".to_string(),
        user_id: user_id.to_string(),
    })
}

pub async fn delete_user(
    write_pool: &sqlx::PgPool,
    requester: &User,
    user_id: &str,
    hard: bool,
) -> Result<DeleteUserMessage, AppError> {
    ensure_system_admin(requester)?;

    let parsed_user_id = parse_user_id(user_id)?;
    if requester.id == parsed_user_id {
        return Err(AppError::BadRequest("Cannot delete yourself".into()));
    }

    let exists = user_repo::user_exists(write_pool, user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    if !exists {
        return Err(AppError::NotFound("User not found".into()));
    }

    let username = user_repo::fetch_username(write_pool, user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .unwrap_or_default();

    if hard {
        user_repo::hard_delete_user(write_pool, user_id).await?;
        tracing::info!(
            user_id = %user_id,
            username = %username,
            requester_id = %requester.id,
            "user hard deleted"
        );

        Ok(DeleteUserMessage {
            message: "User permanently deleted".to_string(),
            user_id: user_id.to_string(),
            deletion_type: "hard".to_string(),
        })
    } else {
        user_repo::soft_delete_user(write_pool, user_id, &requester.id.to_string()).await?;
        tracing::info!(
            user_id = %user_id,
            username = %username,
            requester_id = %requester.id,
            "user soft deleted (archived)"
        );

        Ok(DeleteUserMessage {
            message: "User archived".to_string(),
            user_id: user_id.to_string(),
            deletion_type: "soft".to_string(),
        })
    }
}

pub async fn get_archived_users(
    read_pool: &sqlx::PgPool,
    config: &Config,
    requester: &User,
) -> Result<Vec<ArchivedUserResponse>, AppError> {
    ensure_system_admin(requester)?;

    let rows = user_repo::get_archived_users(read_pool)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(rows
        .into_iter()
        .map(|row| ArchivedUserResponse {
            id: row.id,
            username: row.username,
            full_name: decrypt_pii(&row.full_name, config).unwrap_or_else(|_| "***".to_string()),
            role: row.role,
            is_system_admin: row.is_system_admin,
            archived_at: row.archived_at.to_rfc3339(),
            archived_by: row.archived_by,
        })
        .collect())
}

pub async fn restore_archived_user(
    write_pool: &sqlx::PgPool,
    config: &Config,
    requester: &User,
    user_id: &str,
) -> Result<UserActionMessage, AppError> {
    ensure_system_admin(requester)?;

    let exists = user_repo::archived_user_exists(write_pool, user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    if !exists {
        return Err(AppError::NotFound("Archived user not found".into()));
    }

    let (username, encrypted_email) = user_repo::fetch_archived_identity(write_pool, user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .unwrap_or_default();
    let email = decrypt_pii(&encrypted_email, config).map_err(|_| {
        AppError::InternalServerError(anyhow::anyhow!("Failed to decrypt archived email"))
    })?;

    let conflict_check = user_repo::username_exists(write_pool, &username)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    if conflict_check {
        return Err(AppError::BadRequest(
            "Username already in use by another user".into(),
        ));
    }

    let email_hash = hash_email(&email, config);
    let email_conflict_check = user_repo::email_exists(write_pool, &email_hash)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    if email_conflict_check {
        return Err(AppError::BadRequest(
            "Email already in use by another user".into(),
        ));
    }

    user_repo::restore_user(write_pool, user_id).await?;

    tracing::info!(
        user_id = %user_id,
        username = %username,
        requester_id = %requester.id,
        "user restored from archive"
    );

    Ok(UserActionMessage {
        message: "User restored".to_string(),
        user_id: user_id.to_string(),
    })
}

pub async fn delete_archived_user(
    write_pool: &sqlx::PgPool,
    requester: &User,
    user_id: &str,
) -> Result<UserActionMessage, AppError> {
    ensure_system_admin(requester)?;

    let exists = user_repo::archived_user_exists(write_pool, user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    if !exists {
        return Err(AppError::NotFound("Archived user not found".into()));
    }

    let username = user_repo::fetch_archived_username(write_pool, user_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?
        .unwrap_or_default();

    user_repo::hard_delete_archived_user(write_pool, user_id).await?;

    tracing::info!(
        user_id = %user_id,
        username = %username,
        requester_id = %requester.id,
        "archived user permanently deleted"
    );

    Ok(UserActionMessage {
        message: "Archived user permanently deleted".to_string(),
        user_id: user_id.to_string(),
    })
}

pub fn extract_ip(headers: &HeaderMap) -> Option<String> {
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

pub fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|agent| agent.trim().to_string())
        .filter(|agent| !agent.is_empty())
}

fn parse_user_id(user_id: &str) -> Result<UserId, AppError> {
    UserId::from_str(user_id).map_err(|_| AppError::BadRequest("Invalid user ID".into()))
}

fn decrypt_user(config: &Config, mut user: User) -> Result<User, AppError> {
    user.full_name = decrypt_pii(&user.full_name, config).map_err(|_| {
        AppError::InternalServerError(anyhow::anyhow!("Failed to decrypt full_name"))
    })?;
    user.email = decrypt_pii(&user.email, config)
        .map_err(|_| AppError::InternalServerError(anyhow::anyhow!("Failed to decrypt email")))?;
    Ok(user)
}

fn decrypt_user_best_effort(config: &Config, mut user: User) -> User {
    user.full_name =
        decrypt_pii(&user.full_name, config).unwrap_or_else(|_| user.full_name.clone());
    user.email = decrypt_pii(&user.email, config).unwrap_or_else(|_| user.email.clone());
    user
}

fn maybe_mask_user_response(requester: &User, mut response: UserResponse) -> UserResponse {
    if !requester.is_system_admin() {
        response.full_name = mask_name(&response.full_name);
        response.email = mask_email(&response.email);
    }
    response
}

fn ensure_admin_or_system(user: &User) -> Result<(), AppError> {
    if user.is_admin() || user.is_system_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden("Forbidden".into()))
    }
}

fn ensure_system_admin(user: &User) -> Result<(), AppError> {
    ensure_system_admin_with_message(user, "Forbidden")
}

fn ensure_system_admin_with_message(user: &User, message: &str) -> Result<(), AppError> {
    if user.is_system_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden(message.into()))
    }
}

fn build_audit_entry(
    requester: &User,
    user_id: &str,
    event_type: &str,
    metadata: serde_json::Value,
    headers: &HeaderMap,
    request_id: Option<RequestId>,
) -> AuditLogEntry {
    AuditLogEntry {
        occurred_at: SYSTEM_CLOCK.now_utc(&chrono_tz::UTC),
        actor_id: Some(requester.id),
        actor_type: "user".to_string(),
        event_type: event_type.to_string(),
        target_type: Some("user".to_string()),
        target_id: Some(user_id.to_string()),
        result: "success".to_string(),
        error_code: None,
        metadata: Some(metadata),
        ip: extract_ip(headers),
        user_agent: extract_user_agent(headers),
        request_id: request_id.map(|id| id.0),
    }
}

fn spawn_audit_log(
    audit_log_service: Option<Arc<dyn AuditLogServiceTrait>>,
    entry: AuditLogEntry,
    warning_message: &'static str,
) {
    if let Some(audit_log_service) = audit_log_service {
        tokio::spawn(async move {
            if let Err(err) = audit_log_service.record_event(entry).await {
                tracing::warn!(error = ?err, "{warning_message}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{models::user::UserRole, utils::cookies::SameSite};
    use axum::http::{header, HeaderValue};
    use chrono::Utc;
    use chrono_tz::UTC;

    fn sample_user(role: UserRole, is_system_admin: bool) -> User {
        let now = Utc::now();
        User {
            id: UserId::new(),
            username: "user".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Test User".to_string(),
            email: "user@example.com".to_string(),
            role,
            is_system_admin,
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

    fn config() -> Config {
        Config {
            database_url: "postgres://test".to_string(),
            read_database_url: None,
            jwt_secret: "test-jwt-secret-32-chars-minimum!".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            max_concurrent_sessions: 3,
            audit_log_retention_days: 1825,
            audit_log_retention_forever: false,
            consent_log_retention_days: 1825,
            consent_log_retention_forever: false,
            aws_region: "ap-northeast-1".to_string(),
            aws_kms_key_id: "alias/timekeeper-test".to_string(),
            aws_audit_log_bucket: "timekeeper-audit-logs".to_string(),
            aws_cloudtrail_enabled: true,
            cookie_secure: false,
            cookie_same_site: SameSite::Lax,
            cors_allow_origins: vec!["*".to_string()],
            time_zone: UTC,
            mfa_issuer: "Timekeeper".to_string(),
            rate_limit_ip_max_requests: 15,
            rate_limit_ip_window_seconds: 900,
            rate_limit_user_max_requests: 20,
            rate_limit_user_window_seconds: 3600,
            redis_url: None,
            redis_pool_size: 10,
            redis_connect_timeout: 5,
            feature_redis_cache_enabled: true,
            feature_read_replica_enabled: true,
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

    #[test]
    fn extract_ip_prefers_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.10, 10.0.0.1"),
        );
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.7"));

        assert_eq!(extract_ip(&headers).as_deref(), Some("203.0.113.10"));
    }

    #[test]
    fn extract_user_agent_trims_empty_values() {
        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, HeaderValue::from_static(" test-agent "));
        assert_eq!(extract_user_agent(&headers).as_deref(), Some("test-agent"));
    }

    #[test]
    fn parse_user_id_rejects_invalid_input() {
        assert!(matches!(
            parse_user_id("not-a-uuid"),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn maybe_mask_user_response_masks_for_non_system_admin() {
        let requester = sample_user(UserRole::Admin, false);
        let response = UserResponse {
            id: UserId::new(),
            username: "alice".to_string(),
            full_name: "Alice Example".to_string(),
            email: "alice@example.com".to_string(),
            role: "employee".to_string(),
            is_system_admin: false,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
        };

        let masked = maybe_mask_user_response(&requester, response);
        assert_ne!(masked.full_name, "Alice Example");
        assert_ne!(masked.email, "alice@example.com");
    }

    #[test]
    fn ensure_system_admin_rejects_regular_admin() {
        let admin = sample_user(UserRole::Admin, false);
        assert!(matches!(
            ensure_system_admin(&admin),
            Err(AppError::Forbidden(_))
        ));
    }

    #[test]
    fn decrypt_user_best_effort_preserves_ciphertext_on_failure() {
        let user = sample_user(UserRole::Admin, true);
        let config = config();
        let decrypted = decrypt_user_best_effort(&config, user.clone());
        assert_eq!(decrypted.full_name, user.full_name);
        assert_eq!(decrypted.email, user.email);
    }

    #[test]
    fn build_audit_entry_captures_request_metadata() {
        let requester = sample_user(UserRole::Admin, true);
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.7"));
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("timekeeper-test"),
        );

        let entry = build_audit_entry(
            &requester,
            "user-1",
            "mfa_reset",
            json!({ "reason": "admin_reset" }),
            &headers,
            Some(RequestId("req-1".to_string())),
        );

        assert_eq!(entry.event_type, "mfa_reset");
        assert_eq!(entry.target_id.as_deref(), Some("user-1"));
        assert_eq!(entry.ip.as_deref(), Some("198.51.100.7"));
        assert_eq!(entry.user_agent.as_deref(), Some("timekeeper-test"));
        assert_eq!(entry.request_id.as_deref(), Some("req-1"));
    }
}
