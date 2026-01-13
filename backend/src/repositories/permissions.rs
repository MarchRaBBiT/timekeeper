use sqlx::PgPool;

pub const AUDIT_LOG_READ: &str = "audit_log_read";

pub async fn user_has_permission(
    pool: &PgPool,
    user_id: &str,
    permission: &str,
) -> Result<bool, sqlx::Error> {
    let exists: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM user_permissions WHERE user_id = $1 AND permission_name = $2",
    )
    .bind(user_id)
    .bind(permission)
    .fetch_optional(pool)
    .await?;
    Ok(exists.is_some())
}
