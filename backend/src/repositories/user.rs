//! Repository functions for user management operations.

use chrono::Utc;
use sqlx::PgPool;

/// Archives a user and all related data (soft delete).
/// - Moves user to archived_users
/// - Moves attendance, break_records, leave_requests, overtime_requests, holiday_exceptions to archived tables
/// - Deletes session tokens (refresh_tokens, active_access_tokens)
pub async fn soft_delete_user(
    pool: &PgPool,
    user_id: &str,
    archived_by: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
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
        INSERT INTO archived_users (id, username, password_hash, full_name, role, is_system_admin, mfa_secret, mfa_enabled_at, created_at, updated_at, archived_at, archived_by)
        SELECT id, username, password_hash, full_name, role, is_system_admin, mfa_secret, mfa_enabled_at, created_at, updated_at, $2, $3
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

    tx.commit().await?;
    Ok(())
}

/// Permanently deletes a user and all related data (hard delete).
/// Uses CASCADE delete to remove all related records.
pub async fn hard_delete_user(pool: &PgPool, user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Restores a user from the archive.
pub async fn restore_user(pool: &PgPool, user_id: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 1. Restore user
    sqlx::query(
        r#"
        INSERT INTO users (id, username, password_hash, full_name, role, is_system_admin, mfa_secret, mfa_enabled_at, created_at, updated_at)
        SELECT id, username, password_hash, full_name, role, is_system_admin, mfa_secret, mfa_enabled_at, created_at, updated_at
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

    tx.commit().await?;
    Ok(())
}

/// Checks if a user exists by ID.
pub async fn user_exists(pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM users WHERE id = $1")
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
pub async fn hard_delete_archived_user(pool: &PgPool, user_id: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

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

    tx.commit().await?;
    Ok(())
}

/// Checks if an archived user exists by ID.
pub async fn archived_user_exists(pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM archived_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(result.is_some())
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
        SELECT id, username, full_name, role, is_system_admin, archived_at, archived_by
        FROM archived_users
        ORDER BY archived_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
