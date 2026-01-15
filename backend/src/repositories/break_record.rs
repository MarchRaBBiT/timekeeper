//! Break record repository.
//!
//! Provides CRUD operations for break records.

#![allow(dead_code)]

use crate::error::AppError;
use crate::models::break_record::BreakRecord;
use crate::repositories::repository::Repository;
use crate::types::{AttendanceId, BreakRecordId};
use sqlx::PgPool;

const TABLE_NAME: &str = "break_records";
const SELECT_COLUMNS: &str =
    "id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at";

#[derive(Debug, Default, Clone, Copy)]
pub struct BreakRecordRepository;

impl BreakRecordRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn find_by_attendance(
        &self,
        db: &PgPool,
        attendance_id: AttendanceId,
    ) -> Result<Vec<BreakRecord>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE attendance_id = $1 ORDER BY break_start_time ASC",
            SELECT_COLUMNS, TABLE_NAME
        );
        let rows = sqlx::query_as::<_, BreakRecord>(&query)
            .bind(attendance_id)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    fn base_select_query() -> String {
        format!("SELECT {} FROM {}", SELECT_COLUMNS, TABLE_NAME)
    }
}

impl Repository<BreakRecord> for BreakRecordRepository {
    const TABLE: &'static str = TABLE_NAME;
    type Id = BreakRecordId;

    async fn find_all(&self, db: &PgPool) -> Result<Vec<BreakRecord>, AppError> {
        let query = format!(
            "{} ORDER BY break_start_time DESC",
            Self::base_select_query()
        );
        let rows = sqlx::query_as::<_, BreakRecord>(&query)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    async fn find_by_id(&self, db: &PgPool, id: BreakRecordId) -> Result<BreakRecord, AppError> {
        let query = format!("{} WHERE id = $1", Self::base_select_query());
        let result = sqlx::query_as::<_, BreakRecord>(&query)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Break record not found".into()))?;
        Ok(result)
    }

    async fn create(&self, db: &PgPool, item: &BreakRecord) -> Result<BreakRecord, AppError> {
        let query = format!(
            "INSERT INTO {} (id, attendance_id, break_start_time, break_end_time, duration_minutes, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, BreakRecord>(&query)
            .bind(item.id)
            .bind(item.attendance_id)
            .bind(item.break_start_time)
            .bind(item.break_end_time)
            .bind(item.duration_minutes)
            .bind(item.created_at)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn update(&self, db: &PgPool, item: &BreakRecord) -> Result<BreakRecord, AppError> {
        let query = format!(
            "UPDATE {} SET attendance_id = $2, break_start_time = $3, break_end_time = $4, \
             duration_minutes = $5, updated_at = $6 WHERE id = $1 \
             RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, BreakRecord>(&query)
            .bind(item.id)
            .bind(item.attendance_id)
            .bind(item.break_start_time)
            .bind(item.break_end_time)
            .bind(item.duration_minutes)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn delete(&self, db: &PgPool, id: BreakRecordId) -> Result<(), AppError> {
        let query = format!("DELETE FROM {} WHERE id = $1", TABLE_NAME);
        sqlx::query(&query).bind(id).execute(db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_record_select_columns_include_expected_fields() {
        assert!(SELECT_COLUMNS.contains("break_start_time"));
        assert!(SELECT_COLUMNS.contains("duration_minutes"));
    }
}
