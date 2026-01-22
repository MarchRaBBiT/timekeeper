use chrono::NaiveDate;
use sqlx::PgPool;

use crate::models::holiday_exception::HolidayException;
use crate::types::{HolidayExceptionId, UserId};

/// Inserts a new holiday exception into the database.
pub async fn insert_holiday_exception(
    pool: &PgPool,
    exception: &HolidayException,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO holiday_exceptions \
            (id, user_id, exception_date, override, reason, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(exception.id.to_string())
    .bind(exception.user_id.to_string())
    .bind(exception.exception_date)
    .bind(exception.is_holiday_override)
    .bind(&exception.reason)
    .bind(exception.created_by.to_string())
    .bind(exception.created_at)
    .bind(exception.updated_at)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Lists holiday exceptions for a user within a given date range.
pub async fn list_holiday_exceptions_for_user(
    pool: &PgPool,
    user_id: UserId,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<Vec<HolidayException>, sqlx::Error> {
    sqlx::query_as::<_, HolidayException>(
        r#"
        SELECT id, user_id, exception_date, override, reason, created_by, created_at, updated_at
        FROM holiday_exceptions
        WHERE user_id = $1
          AND ($2::date IS NULL OR exception_date >= $2)
          AND ($3::date IS NULL OR exception_date <= $3)
        ORDER BY exception_date
        "#,
    )
    .bind(user_id.to_string())
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
}

/// Deletes a holiday exception by ID and User ID.
pub async fn delete_holiday_exception(
    pool: &PgPool,
    id: HolidayExceptionId,
    user_id: UserId,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM holiday_exceptions
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await
    .map(|res| res.rows_affected())
}
