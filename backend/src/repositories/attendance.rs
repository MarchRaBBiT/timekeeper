//! Attendance repository.
//!
//! Provides CRUD operations for attendance records.

#![allow(dead_code)]

use crate::error::AppError;
use crate::models::attendance::Attendance;
use crate::repositories::repository::Repository;
use crate::types::{AttendanceId, UserId};
use chrono::NaiveDate;
use sqlx::PgPool;

const TABLE_NAME: &str = "attendance";
const SELECT_COLUMNS: &str =
    "id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at";

#[derive(Debug, Default, Clone, Copy)]
pub struct AttendanceRepository;

impl AttendanceRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn find_by_user_and_date(
        &self,
        db: &PgPool,
        user_id: UserId,
        date: NaiveDate,
    ) -> Result<Option<Attendance>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE user_id = $1 AND date = $2",
            SELECT_COLUMNS, TABLE_NAME
        );
        let row = sqlx::query_as::<_, Attendance>(&query)
            .bind(user_id)
            .bind(date)
            .fetch_optional(db)
            .await?;
        Ok(row)
    }

    fn base_select_query() -> String {
        format!("SELECT {} FROM {}", SELECT_COLUMNS, TABLE_NAME)
    }
}

impl Repository<Attendance> for AttendanceRepository {
    const TABLE: &'static str = TABLE_NAME;
    type Id = AttendanceId;

    async fn find_all(&self, db: &PgPool) -> Result<Vec<Attendance>, AppError> {
        let query = format!("{} ORDER BY date DESC", Self::base_select_query());
        let rows = sqlx::query_as::<_, Attendance>(&query)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    async fn find_by_id(&self, db: &PgPool, id: AttendanceId) -> Result<Attendance, AppError> {
        let query = format!("{} WHERE id = $1", Self::base_select_query());
        let result = sqlx::query_as::<_, Attendance>(&query)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Attendance record not found".into()))?;
        Ok(result)
    }

    async fn create(&self, db: &PgPool, item: &Attendance) -> Result<Attendance, AppError> {
        let query = format!(
            "INSERT INTO {} (id, user_id, date, clock_in_time, clock_out_time, status, total_work_hours, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, Attendance>(&query)
            .bind(item.id)
            .bind(item.user_id)
            .bind(item.date)
            .bind(item.clock_in_time)
            .bind(item.clock_out_time)
            .bind(item.status.db_value())
            .bind(item.total_work_hours)
            .bind(item.created_at)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn update(&self, db: &PgPool, item: &Attendance) -> Result<Attendance, AppError> {
        let query = format!(
            "UPDATE {} SET user_id = $2, date = $3, clock_in_time = $4, clock_out_time = $5, \
             status = $6, total_work_hours = $7, updated_at = $8 WHERE id = $1 \
             RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, Attendance>(&query)
            .bind(item.id)
            .bind(item.user_id)
            .bind(item.date)
            .bind(item.clock_in_time)
            .bind(item.clock_out_time)
            .bind(item.status.db_value())
            .bind(item.total_work_hours)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn delete(&self, db: &PgPool, id: AttendanceId) -> Result<(), AppError> {
        let query = format!("DELETE FROM {} WHERE id = $1", TABLE_NAME);
        sqlx::query(&query).bind(id).execute(db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attendance_select_columns_include_expected_fields() {
        assert!(SELECT_COLUMNS.contains("clock_in_time"));
        assert!(SELECT_COLUMNS.contains("total_work_hours"));
        assert!(SELECT_COLUMNS.contains("updated_at"));
    }
}
