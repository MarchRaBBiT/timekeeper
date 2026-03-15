//! Repository functions for department hierarchy and manager assignment.

use sqlx::PgPool;

use crate::models::department::{Department, DepartmentManager};
use crate::types::{DepartmentId, UserId};

/// Lists all departments ordered by name.
pub async fn list_departments(pool: &PgPool) -> Result<Vec<Department>, sqlx::Error> {
    sqlx::query_as::<_, Department>(
        "SELECT id, name, parent_id, created_at, updated_at FROM departments ORDER BY name",
    )
    .fetch_all(pool)
    .await
}

/// Fetches a department by ID.
pub async fn find_department_by_id(
    pool: &PgPool,
    id: &str,
) -> Result<Option<Department>, sqlx::Error> {
    sqlx::query_as::<_, Department>(
        "SELECT id, name, parent_id, created_at, updated_at FROM departments WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Checks whether a department has any children.
pub async fn department_has_children(pool: &PgPool, id: &str) -> Result<bool, sqlx::Error> {
    let result: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM departments WHERE parent_id = $1)")
            .bind(id)
            .fetch_one(pool)
            .await?;
    Ok(result.0)
}

/// Creates a new department.
pub async fn create_department(
    pool: &PgPool,
    id: &str,
    name: &str,
    parent_id: Option<&str>,
) -> Result<Department, sqlx::Error> {
    sqlx::query_as::<_, Department>(
        "INSERT INTO departments (id, name, parent_id) \
         VALUES ($1, $2, $3) \
         RETURNING id, name, parent_id, created_at, updated_at",
    )
    .bind(id)
    .bind(name)
    .bind(parent_id)
    .fetch_one(pool)
    .await
}

/// Updates a department's name and/or parent.
pub async fn update_department(
    pool: &PgPool,
    id: &str,
    name: Option<&str>,
    parent_id: Option<Option<&str>>,
) -> Result<Option<Department>, sqlx::Error> {
    // Build SET clause dynamically based on provided fields
    let mut set_parts: Vec<String> = vec!["updated_at = NOW()".to_string()];
    if name.is_some() {
        set_parts.push("name = $2".to_string());
    }
    // parent_id updates handled below
    let dept = if let Some(pid_option) = parent_id {
        // Update both name (if provided) and parent_id
        if let Some(n) = name {
            sqlx::query_as::<_, Department>(
                "UPDATE departments SET name = $2, parent_id = $3, updated_at = NOW() \
                 WHERE id = $1 \
                 RETURNING id, name, parent_id, created_at, updated_at",
            )
            .bind(id)
            .bind(n)
            .bind(pid_option)
            .fetch_optional(pool)
            .await?
        } else {
            sqlx::query_as::<_, Department>(
                "UPDATE departments SET parent_id = $2, updated_at = NOW() \
                 WHERE id = $1 \
                 RETURNING id, name, parent_id, created_at, updated_at",
            )
            .bind(id)
            .bind(pid_option)
            .fetch_optional(pool)
            .await?
        }
    } else if let Some(n) = name {
        sqlx::query_as::<_, Department>(
            "UPDATE departments SET name = $2, updated_at = NOW() \
             WHERE id = $1 \
             RETURNING id, name, parent_id, created_at, updated_at",
        )
        .bind(id)
        .bind(n)
        .fetch_optional(pool)
        .await?
    } else {
        find_department_by_id(pool, id).await?
    };
    Ok(dept)
}

/// Deletes a department by ID.
pub async fn delete_department(pool: &PgPool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM departments WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Lists all managers for a department.
pub async fn list_department_managers(
    pool: &PgPool,
    department_id: &str,
) -> Result<Vec<DepartmentManager>, sqlx::Error> {
    sqlx::query_as::<_, DepartmentManager>(
        "SELECT department_id, user_id, assigned_at FROM department_managers WHERE department_id = $1",
    )
    .bind(department_id)
    .fetch_all(pool)
    .await
}

/// Assigns a manager to a department (idempotent upsert).
pub async fn assign_manager(
    pool: &PgPool,
    department_id: &str,
    user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO department_managers (department_id, user_id) \
         VALUES ($1, $2) \
         ON CONFLICT (department_id, user_id) DO NOTHING",
    )
    .bind(department_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Removes a manager assignment.
pub async fn remove_manager(
    pool: &PgPool,
    department_id: &str,
    user_id: &str,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM department_managers WHERE department_id = $1 AND user_id = $2")
            .bind(department_id)
            .bind(user_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

/// Checks whether a manager can approve a request for the given applicant.
///
/// Returns `true` if `applicant_id` is in the manager's direct or subordinate departments.
pub async fn can_manager_approve(
    pool: &PgPool,
    manager_id: UserId,
    applicant_id: UserId,
) -> Result<bool, sqlx::Error> {
    let result: (bool,) = sqlx::query_as(
        r#"
        WITH RECURSIVE subordinate_depts AS (
            SELECT dm.department_id
            FROM department_managers dm
            WHERE dm.user_id = $1
            UNION ALL
            SELECT d.id
            FROM departments d
            INNER JOIN subordinate_depts sd ON d.parent_id = sd.department_id
        )
        SELECT EXISTS (
            SELECT 1 FROM users u
            WHERE u.id = $2
              AND u.department_id IN (SELECT department_id FROM subordinate_depts)
        ) AS can_approve
        "#,
    )
    .bind(manager_id.to_string())
    .bind(applicant_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(result.0)
}

#[cfg(test)]
mod tests {
    // Unit-testable logic lives in integration tests.
}
