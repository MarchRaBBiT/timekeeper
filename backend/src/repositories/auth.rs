use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::types::UserId;
use crate::{models::user::User, utils::jwt::RefreshToken};

#[allow(dead_code)]
#[derive(Debug, FromRow)]
/// Represents a stored refresh token in the database.
pub struct StoredRefreshToken {
    pub id: String,
    pub user_id: UserId,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug)]
/// Represents an active access token to be stored for revocation checks.
pub struct ActiveAccessToken<'a> {
    pub jti: &'a str,
    pub user_id: UserId,
    pub expires_at: DateTime<Utc>,
    pub context: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct LockoutPolicy {
    pub threshold: i32,
    pub duration_minutes: i64,
    pub backoff_enabled: bool,
    pub max_duration_hours: i64,
}

#[derive(Debug, Clone)]
pub struct LoginFailureState {
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub lockout_count: i32,
    pub became_locked: bool,
}

/// Finds a user by their username.
pub async fn find_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, COALESCE(full_name_enc, '') as full_name, \
         COALESCE(email_enc, '') as email, LOWER(role) as role, is_system_admin, \
         mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at \
         FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

/// Finds a user by their ID.
pub async fn find_user_by_id(pool: &PgPool, user_id: UserId) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, COALESCE(full_name_enc, '') as full_name, \
         COALESCE(email_enc, '') as email, LOWER(role) as role, is_system_admin, \
         mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at \
         FROM users WHERE id = $1",
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
}

/// Inserts a new refresh token into the database.
pub async fn insert_refresh_token(pool: &PgPool, token: &RefreshToken) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token.id)
    .bind(token.user_id.to_string())
    .bind(&token.token_hash)
    .bind(token.expires_at)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Deletes a specific refresh token by its ID.
pub async fn delete_refresh_token_by_id(pool: &PgPool, token_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Deletes all refresh tokens for a specific user.
pub async fn delete_refresh_tokens_for_user(
    pool: &PgPool,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .map(|_| ())
}

/// Fetches a valid (non-expired) refresh token by its ID.
pub async fn fetch_valid_refresh_token(
    pool: &PgPool,
    token_id: &str,
    now: DateTime<Utc>,
) -> Result<Option<StoredRefreshToken>, sqlx::Error> {
    sqlx::query_as::<_, StoredRefreshToken>(
        "SELECT id, user_id, token_hash, expires_at FROM refresh_tokens \
         WHERE id = $1 AND expires_at > $2",
    )
    .bind(token_id)
    .bind(now)
    .fetch_optional(pool)
    .await
}

/// Inserts an active access token into the database (for revocation tracking).
pub async fn insert_active_access_token(
    pool: &PgPool,
    token: &ActiveAccessToken<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO active_access_tokens (jti, user_id, expires_at, context) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(token.jti)
    .bind(token.user_id.to_string())
    .bind(token.expires_at)
    .bind(token.context)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Checks if an access token exists (is active/valid) by its JTI.
pub async fn access_token_exists(pool: &PgPool, jti: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM active_access_tokens WHERE jti = $1 LIMIT 1)",
    )
    .bind(jti)
    .fetch_one(pool)
    .await
}

/// Deletes a specific active access token by its JTI (revocation).
pub async fn delete_active_access_token_by_jti(
    pool: &PgPool,
    jti: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_access_tokens WHERE jti = $1")
        .bind(jti)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Deletes all active access tokens for a specific user (revocation).
pub async fn delete_active_access_tokens_for_user(
    pool: &PgPool,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_access_tokens WHERE user_id = $1")
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .map(|_| ())
}

/// Removes all expired access tokens from the database.
pub async fn cleanup_expired_access_tokens(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_access_tokens WHERE expires_at <= NOW()")
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn find_user_by_email_hash(
    pool: &PgPool,
    email_hash: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, COALESCE(full_name_enc, '') as full_name, \
         COALESCE(email_enc, '') as email, LOWER(role) as role, is_system_admin, \
         mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at \
         FROM users WHERE email_hash = $1",
    )
    .bind(email_hash)
    .fetch_optional(pool)
    .await
}

/// Updates a user's password hash.
pub async fn update_user_password(
    pool: &PgPool,
    user_id: UserId,
    new_password_hash: &str,
    previous_password_hash: &str,
    history_limit: u32,
) -> Result<User, sqlx::Error> {
    if history_limit > 0 {
        let mut tx = pool.begin().await?;

        sqlx::query(
            "INSERT INTO password_histories (id, user_id, password_hash, changed_at) \
             VALUES ($1, $2, $3, NOW())",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(user_id.to_string())
        .bind(previous_password_hash)
        .execute(&mut *tx)
        .await?;

        let history_limit = history_limit as i64;
        sqlx::query(
            "DELETE FROM password_histories WHERE id IN (\
             SELECT id FROM password_histories WHERE user_id = $1 \
             ORDER BY changed_at DESC OFFSET $2)",
        )
        .bind(user_id.to_string())
        .bind(history_limit)
        .execute(&mut *tx)
        .await?;

        let user = sqlx::query_as::<_, User>(
            "UPDATE users SET password_hash = $1, password_changed_at = NOW(), updated_at = NOW() \
             WHERE id = $2 \
             RETURNING id, username, password_hash, COALESCE(full_name_enc, '') as full_name, \
             COALESCE(email_enc, '') as email, LOWER(role) as role, is_system_admin, \
             mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at",
        )
        .bind(new_password_hash)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(user)
    } else {
        sqlx::query_as::<_, User>(
            "UPDATE users SET password_hash = $1, password_changed_at = NOW(), updated_at = NOW() \
             WHERE id = $2 \
             RETURNING id, username, password_hash, COALESCE(full_name_enc, '') as full_name, \
             COALESCE(email_enc, '') as email, LOWER(role) as role, is_system_admin, \
             mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at",
        )
        .bind(new_password_hash)
        .bind(user_id)
        .fetch_one(pool)
        .await
    }
}

/// Fetches recent password hashes for history enforcement.
pub async fn fetch_recent_password_hashes(
    pool: &PgPool,
    user_id: UserId,
    limit: u32,
) -> Result<Vec<String>, sqlx::Error> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    sqlx::query_scalar::<_, String>(
        "SELECT password_hash FROM password_histories WHERE user_id = $1 \
         ORDER BY changed_at DESC LIMIT $2",
    )
    .bind(user_id.to_string())
    .bind(limit as i64)
    .fetch_all(pool)
    .await
}

/// Deletes all refresh tokens for a specific user (alias).
pub async fn delete_all_refresh_tokens_for_user(
    pool: &PgPool,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .map(|_| ())
}

fn lockout_duration_minutes(lockout_count: i32, policy: LockoutPolicy) -> i64 {
    let base = policy.duration_minutes.max(1);
    if !policy.backoff_enabled {
        return base;
    }

    let exponent = lockout_count.saturating_sub(1).clamp(0, 20) as u32;
    let multiplier = 2_i64.saturating_pow(exponent);
    let minutes = base.saturating_mul(multiplier);
    let max_minutes = policy.max_duration_hours.max(1).saturating_mul(60);
    minutes.min(max_minutes)
}

pub async fn record_login_failure(
    pool: &PgPool,
    user_id: UserId,
    now: DateTime<Utc>,
    policy: LockoutPolicy,
) -> Result<LoginFailureState, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query_as::<_, (i32, Option<DateTime<Utc>>, i32)>(
        "SELECT failed_login_attempts, locked_until, lockout_count \
         FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(user_id.to_string())
    .fetch_optional(&mut *tx)
    .await?;

    let Some((failed_attempts, locked_until, lockout_count)) = row else {
        tx.rollback().await?;
        return Ok(LoginFailureState {
            failed_login_attempts: 0,
            locked_until: None,
            lockout_count: 0,
            became_locked: false,
        });
    };

    let is_still_locked = locked_until.map(|until| until > now).unwrap_or(false);
    if is_still_locked {
        tx.commit().await?;
        return Ok(LoginFailureState {
            failed_login_attempts: failed_attempts,
            locked_until,
            lockout_count,
            became_locked: false,
        });
    }

    let threshold = policy.threshold.max(1);
    let next_failed_attempts = failed_attempts + 1;
    if next_failed_attempts >= threshold {
        let next_lockout_count = lockout_count + 1;
        let duration = lockout_duration_minutes(next_lockout_count, policy);
        let next_locked_until = now + chrono::Duration::minutes(duration);
        sqlx::query(
            "UPDATE users \
             SET failed_login_attempts = 0, \
                 locked_until = $1, \
                 lock_reason = $2, \
                 lockout_count = $3, \
                 updated_at = NOW() \
             WHERE id = $4",
        )
        .bind(next_locked_until)
        .bind("too_many_failed_attempts")
        .bind(next_lockout_count)
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(LoginFailureState {
            failed_login_attempts: 0,
            locked_until: Some(next_locked_until),
            lockout_count: next_lockout_count,
            became_locked: true,
        })
    } else {
        sqlx::query(
            "UPDATE users \
             SET failed_login_attempts = $1, updated_at = NOW() \
             WHERE id = $2",
        )
        .bind(next_failed_attempts)
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(LoginFailureState {
            failed_login_attempts: next_failed_attempts,
            locked_until: None,
            lockout_count,
            became_locked: false,
        })
    }
}

pub async fn clear_login_failures(pool: &PgPool, user_id: UserId) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users \
         SET failed_login_attempts = 0, \
             locked_until = NULL, \
             lock_reason = NULL, \
             lockout_count = 0, \
             updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(user_id.to_string())
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn unlock_user_account(pool: &PgPool, user_id: UserId) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users \
         SET failed_login_attempts = 0, \
             locked_until = NULL, \
             lock_reason = NULL, \
             lockout_count = 0, \
             updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_recent_password_hashes_returns_empty_for_zero_limit() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://127.0.0.1:1/timekeeper")
            .expect("create lazy pool");

        let result = fetch_recent_password_hashes(&pool, UserId::new(), 0).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Vec::<String>::new());
    }

    #[test]
    fn active_access_token_struct_has_fields() {
        let jti = "test-jti";
        let user_id = UserId::new();
        let expires_at = chrono::Utc::now();
        let context = Some("test-context");

        let token = ActiveAccessToken {
            jti,
            user_id,
            expires_at,
            context,
        };

        assert_eq!(token.jti, jti);
        assert_eq!(token.user_id, user_id);
        assert_eq!(token.expires_at, expires_at);
        assert_eq!(token.context, context);
    }

    #[test]
    fn active_access_token_struct_allows_none_context() {
        let token = ActiveAccessToken {
            jti: "test-jti",
            user_id: UserId::new(),
            expires_at: chrono::Utc::now(),
            context: None,
        };

        assert!(token.context.is_none());
    }

    #[test]
    fn stored_refresh_token_struct_derives_debug() {
        let token = StoredRefreshToken {
            id: "test-id".to_string(),
            user_id: UserId::new(),
            token_hash: "hash".to_string(),
            expires_at: chrono::Utc::now(),
        };

        let debug_str = format!("{:?}", token);
        assert!(debug_str.contains("StoredRefreshToken"));
    }

    #[test]
    fn lockout_duration_minutes_applies_backoff_and_cap() {
        let policy = LockoutPolicy {
            threshold: 5,
            duration_minutes: 15,
            backoff_enabled: true,
            max_duration_hours: 24,
        };
        assert_eq!(lockout_duration_minutes(1, policy), 15);
        assert_eq!(lockout_duration_minutes(2, policy), 30);
        assert_eq!(lockout_duration_minutes(3, policy), 60);
        assert_eq!(lockout_duration_minutes(8, policy), 1440);
    }

    #[test]
    fn lockout_duration_minutes_fixed_when_backoff_disabled() {
        let policy = LockoutPolicy {
            threshold: 5,
            duration_minutes: 15,
            backoff_enabled: false,
            max_duration_hours: 24,
        };
        assert_eq!(lockout_duration_minutes(1, policy), 15);
        assert_eq!(lockout_duration_minutes(5, policy), 15);
    }
}
