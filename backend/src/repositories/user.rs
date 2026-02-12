//! Repository functions for user management operations.

use chrono::Utc;
use sqlx::PgPool;

use crate::error::AppError;
use crate::repositories::transaction;

use crate::models::user::{User, UserRole};

/// Fetches all users ordered by creation date (descending).
pub async fn list_users(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, full_name_enc as full_name, \
         email_enc as email, LOWER(role) as role, is_system_admin, \
         mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at \
         FROM users ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
}

/// Creates a new user.
pub async fn create_user(
    pool: &PgPool,
    user: &User,
    email_hash: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (id, username, password_hash, full_name_enc, email_enc, email_hash, role, is_system_admin, \
         mfa_secret_enc, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17) \
         RETURNING id, username, password_hash, full_name_enc as full_name, \
         email_enc as email, LOWER(role) as role, is_system_admin, \
         mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at",
    )
    .bind(user.id.to_string())
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    .bind(&user.email)
    .bind(email_hash)
    .bind(match user.role {
        UserRole::Employee => "employee",
        UserRole::Admin => "admin",
    })
    .bind(user.is_system_admin)
    .bind(&user.mfa_secret)
    .bind(user.mfa_enabled_at)
    .bind(user.password_changed_at)
    .bind(user.failed_login_attempts)
    .bind(user.locked_until)
    .bind(&user.lock_reason)
    .bind(user.lockout_count)
    .bind(user.created_at)
    .bind(user.updated_at)
    .fetch_one(pool)
    .await
}

/// Updates an existing user's profile.
pub async fn update_user(
    pool: &PgPool,
    user_id: &str,
    full_name: &str,
    email: &str,
    email_hash: &str,
    role: UserRole,
    is_system_admin: bool,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET full_name_enc = $1, email_enc = $2, email_hash = $3, role = $4, is_system_admin = $5, updated_at = NOW() \
         WHERE id = $6 \
         RETURNING id, username, password_hash, full_name_enc as full_name, \
         email_enc as email, LOWER(role) as role, is_system_admin, \
         mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at",
    )
    .bind(full_name)
    .bind(email)
    .bind(email_hash)
    .bind(role.as_str())
    .bind(is_system_admin)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

/// Checks if an email exists for a different user (for uniqueness check).
pub async fn email_exists_for_other_user(
    pool: &PgPool,
    email_hash: &str,
    exclude_user_id: &str,
) -> Result<bool, sqlx::Error> {
    let result: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM users WHERE email_hash = $1 AND id != $2)")
            .bind(email_hash)
            .bind(exclude_user_id)
            .fetch_one(pool)
            .await?;
    Ok(result.0)
}

/// Checks if an email exists (for conflict checks).
pub async fn email_exists(pool: &PgPool, email_hash: &str) -> Result<bool, sqlx::Error> {
    let result: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM users WHERE email_hash = $1)")
            .bind(email_hash)
            .fetch_one(pool)
            .await?;
    Ok(result.0)
}

/// Resets MFA for a user.
pub async fn reset_mfa(pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET mfa_secret_enc = NULL, mfa_enabled_at = NULL, updated_at = NOW() WHERE id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Checks if a username exists (for conflict check).
pub async fn username_exists(pool: &PgPool, username: &str) -> Result<bool, sqlx::Error> {
    let result: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(username)
        .fetch_one(pool)
        .await?;
    Ok(result.0)
}

/// Updates a user's profile (self-service).
pub async fn update_profile(
    pool: &PgPool,
    user_id: &str,
    full_name: &str,
    email: &str,
    email_hash: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET full_name_enc = $1, email_enc = $2, email_hash = $3, updated_at = NOW() \
         WHERE id = $4 \
         RETURNING id, username, password_hash, full_name_enc as full_name, \
         email_enc as email, LOWER(role) as role, is_system_admin, \
         mfa_secret_enc as mfa_secret, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, lockout_count, created_at, updated_at",
    )
    .bind(full_name)
    .bind(email)
    .bind(email_hash)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

/// Enables MFA for a user.
pub async fn enable_mfa(
    pool: &PgPool,
    user_id: &str,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE users SET mfa_enabled_at = $1, updated_at = $1 WHERE id = $2")
        .bind(at)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Disables MFA for a user (self-service).
pub async fn disable_mfa(pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET mfa_secret_enc = NULL, mfa_enabled_at = NULL, updated_at = NOW() WHERE id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Sets the MFA secret for enrollment.
pub async fn set_mfa_secret(
    pool: &PgPool,
    user_id: &str,
    secret: &str,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET mfa_secret_enc = $1, mfa_enabled_at = NULL, updated_at = $2 WHERE id = $3",
    )
    .bind(secret)
    .bind(at)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Archives a user and all related data (soft delete).
/// - Moves user to archived_users
/// - Moves attendance, break_records, leave_requests, overtime_requests, holiday_exceptions to archived tables
/// - Deletes session tokens (refresh_tokens, active_access_tokens)
pub async fn soft_delete_user(
    pool: &PgPool,
    user_id: &str,
    archived_by: &str,
) -> Result<(), AppError> {
    let mut tx = transaction::begin_transaction(pool).await?;
    let now = Utc::now();

    // 1. Archive break records (via attendance)
    sqlx::query(
        r#"
        INSERT INTO archived_break_records (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at, archived_at)
        SELECT br.id, br.attendance_id, br.break_start_time, br.break_end_time, br.duration_minutes, br.created_at, br.updated_at, $2
        FROM break_records br
        INNER JOIN attendance a ON br.attendance_id = a.id
        WHERE a.user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 2. Archive attendance
    sqlx::query(
        r#"
        INSERT INTO archived_attendance (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at, archived_at)
        SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at, $2
        FROM attendance
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 3. Archive leave requests
    sqlx::query(
        r#"
        INSERT INTO archived_leave_requests (id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, created_at, updated_at, archived_at)
        SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, created_at, updated_at, $2
        FROM leave_requests
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 4. Archive overtime requests
    sqlx::query(
        r#"
        INSERT INTO archived_overtime_requests (id, user_id, date, planned_hours, reason, status, approved_by, approved_at, created_at, updated_at, archived_at)
        SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, created_at, updated_at, $2
        FROM overtime_requests
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 5. Archive holiday exceptions
    sqlx::query(
        r#"
        INSERT INTO archived_holiday_exceptions (id, user_id, exception_date, override, reason, created_by, created_at, updated_at, archived_at)
        SELECT id, user_id, exception_date, override, reason, created_by, created_at, updated_at, $2
        FROM holiday_exceptions
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 6. Archive user
    sqlx::query(
        r#"
        INSERT INTO archived_users (
            id, username, password_hash, role, is_system_admin,
            full_name_enc, email_enc, email_hash, mfa_secret_enc, mfa_enabled_at, password_changed_at, failed_login_attempts,
            locked_until, lock_reason, lockout_count, created_at, updated_at, archived_at, archived_by
        )
        SELECT
            id, username, password_hash, role, is_system_admin,
            full_name_enc, email_enc, email_hash, mfa_secret_enc, mfa_enabled_at, password_changed_at, failed_login_attempts,
            locked_until, lock_reason, lockout_count, created_at, updated_at, $2, $3
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(now)
    .bind(archived_by)
    .execute(&mut *tx)
    .await?;

    // 7. Delete user (will cascade delete related records and session tokens)
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    transaction::commit_transaction(tx).await?;
    Ok(())
}

/// Permanently deletes a user and all related data (hard delete).
/// Uses CASCADE delete to remove all related records.
pub async fn hard_delete_user(pool: &PgPool, user_id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Restores a user from the archive.
pub async fn restore_user(pool: &PgPool, user_id: &str) -> Result<(), AppError> {
    let mut tx = transaction::begin_transaction(pool).await?;

    // 1. Restore user
    sqlx::query(
        r#"
        INSERT INTO users (
            id, username, password_hash, role, is_system_admin,
            full_name_enc, email_enc, email_hash, mfa_secret_enc, mfa_enabled_at, password_changed_at, failed_login_attempts,
            locked_until, lock_reason, lockout_count, created_at, updated_at
        )
        SELECT
            id, username, password_hash, role, is_system_admin,
            full_name_enc, email_enc, email_hash, mfa_secret_enc, mfa_enabled_at, password_changed_at, failed_login_attempts,
            locked_until, lock_reason, lockout_count, created_at, updated_at
        FROM archived_users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // 2. Restore attendance
    sqlx::query(
        r#"
        INSERT INTO attendance (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at)
        SELECT id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at
        FROM archived_attendance
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // 3. Restore break records
    sqlx::query(
        r#"
        INSERT INTO break_records (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at)
        SELECT abr.id, abr.attendance_id, abr.break_start_time, abr.break_end_time, abr.duration_minutes, abr.created_at, abr.updated_at
        FROM archived_break_records abr
        INNER JOIN archived_attendance aa ON abr.attendance_id = aa.id
        WHERE aa.user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // 4. Restore leave requests
    sqlx::query(
        r#"
        INSERT INTO leave_requests (id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, created_at, updated_at)
        SELECT id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, created_at, updated_at
        FROM archived_leave_requests
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // 5. Restore overtime requests
    sqlx::query(
        r#"
        INSERT INTO overtime_requests (id, user_id, date, planned_hours, reason, status, approved_by, approved_at, created_at, updated_at)
        SELECT id, user_id, date, planned_hours, reason, status, approved_by, approved_at, created_at, updated_at
        FROM archived_overtime_requests
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // 6. Restore holiday exceptions
    sqlx::query(
        r#"
        INSERT INTO holiday_exceptions (id, user_id, exception_date, override, reason, created_by, created_at, updated_at)
        SELECT id, user_id, exception_date, override, reason, created_by, created_at, updated_at
        FROM archived_holiday_exceptions
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // 7. Delete from archive
    sqlx::query("DELETE FROM archived_holiday_exceptions WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM archived_overtime_requests WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM archived_leave_requests WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r#"
        DELETE FROM archived_break_records
        WHERE attendance_id IN (SELECT id FROM archived_attendance WHERE user_id = $1)
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM archived_attendance WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM archived_users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    transaction::commit_transaction(tx).await?;
    Ok(())
}

/// Checks if a user exists by ID.
pub async fn user_exists(pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as("SELECT 1::bigint FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(result.is_some())
}

/// Fetches username by user ID.
pub async fn fetch_username(pool: &PgPool, user_id: &str) -> Result<Option<String>, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(result.map(|(u,)| u))
}

// ============================================================================
// Archived user functions
// ============================================================================

/// Permanently deletes an archived user and all related archived data.
pub async fn hard_delete_archived_user(pool: &PgPool, user_id: &str) -> Result<(), AppError> {
    let mut tx = transaction::begin_transaction(pool).await?;

    // Delete in reverse order of dependencies
    sqlx::query(
        r#"
        DELETE FROM archived_break_records
        WHERE attendance_id IN (SELECT id FROM archived_attendance WHERE user_id = $1)
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM archived_attendance WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM archived_leave_requests WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM archived_overtime_requests WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM archived_holiday_exceptions WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM archived_users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    transaction::commit_transaction(tx).await?;
    Ok(())
}

/// Checks if an archived user exists by ID.
pub async fn archived_user_exists(pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
    let result: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM archived_users WHERE id = $1)")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
    Ok(result.0)
}

/// Fetches archived username by user ID.
pub async fn fetch_archived_username(
    pool: &PgPool,
    user_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT username FROM archived_users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    Ok(result.map(|(u,)| u))
}

/// Fetches archived username and email by user ID.
pub async fn fetch_archived_identity(
    pool: &PgPool,
    user_id: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let result: Option<(String, String)> =
        sqlx::query_as("SELECT username, email_enc as email FROM archived_users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    Ok(result)
}

/// Archived user data for API response.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArchivedUserRow {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub is_system_admin: bool,
    pub archived_at: chrono::DateTime<chrono::Utc>,
    pub archived_by: Option<String>,
}

/// Fetches all archived users.
pub async fn get_archived_users(pool: &PgPool) -> Result<Vec<ArchivedUserRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ArchivedUserRow>(
        r#"
        SELECT id, username, full_name_enc as full_name, role, is_system_admin, archived_at, archived_by
        FROM archived_users
        ORDER BY archived_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archived_user_row_struct_exists() {
        let row = ArchivedUserRow {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            full_name: "Test User".to_string(),
            role: "employee".to_string(),
            is_system_admin: false,
            archived_at: Utc::now(),
            archived_by: None,
        };
        assert_eq!(row.id, "test-id");
        assert_eq!(row.username, "testuser");
    }

    #[test]
    fn user_role_to_string_conversion() {
        let employee = UserRole::Employee;
        let admin = UserRole::Admin;

        let emp_str = match employee {
            UserRole::Employee => "employee",
            UserRole::Admin => "admin",
        };
        let adm_str = match admin {
            UserRole::Employee => "employee",
            UserRole::Admin => "admin",
        };

        assert_eq!(emp_str, "employee");
        assert_eq!(adm_str, "admin");
    }
}
