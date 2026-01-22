//! Holiday repository.
//!
//! Provides CRUD operations for holidays.

#![allow(dead_code)]

use crate::error::AppError;
use crate::models::holiday::Holiday;
use crate::repositories::repository::Repository;
use crate::types::HolidayId;
use chrono::NaiveDate;
use sqlx::PgPool;

const TABLE_NAME: &str = "holidays";
const SELECT_COLUMNS: &str = "id, holiday_date, name, description, created_at, updated_at";

#[derive(Debug, Default, Clone, Copy)]
pub struct HolidayRepository;

impl HolidayRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn find_by_date(
        &self,
        db: &PgPool,
        date: NaiveDate,
    ) -> Result<Option<Holiday>, AppError> {
        let query = format!(
            "SELECT {} FROM {} WHERE holiday_date = $1",
            SELECT_COLUMNS, TABLE_NAME
        );
        let row = sqlx::query_as::<_, Holiday>(&query)
            .bind(date)
            .fetch_optional(db)
            .await?;
        Ok(row)
    }

    fn base_select_query() -> String {
        format!("SELECT {} FROM {}", SELECT_COLUMNS, TABLE_NAME)
    }
}

impl Repository<Holiday> for HolidayRepository {
    const TABLE: &'static str = TABLE_NAME;
    type Id = HolidayId;

    async fn find_all(&self, db: &PgPool) -> Result<Vec<Holiday>, AppError> {
        let query = format!("{} ORDER BY holiday_date ASC", Self::base_select_query());
        let rows = sqlx::query_as::<_, Holiday>(&query).fetch_all(db).await?;
        Ok(rows)
    }

    async fn find_by_id(&self, db: &PgPool, id: HolidayId) -> Result<Holiday, AppError> {
        let query = format!("{} WHERE id = $1", Self::base_select_query());
        let result = sqlx::query_as::<_, Holiday>(&query)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Holiday not found".into()))?;
        Ok(result)
    }

    async fn create(&self, db: &PgPool, item: &Holiday) -> Result<Holiday, AppError> {
        let query = format!(
            "INSERT INTO {} (id, holiday_date, name, description, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, Holiday>(&query)
            .bind(item.id)
            .bind(item.holiday_date)
            .bind(&item.name)
            .bind(&item.description)
            .bind(item.created_at)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn update(&self, db: &PgPool, item: &Holiday) -> Result<Holiday, AppError> {
        let query = format!(
            "UPDATE {} SET holiday_date = $2, name = $3, description = $4, updated_at = $5 \
             WHERE id = $1 RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, Holiday>(&query)
            .bind(item.id)
            .bind(item.holiday_date)
            .bind(&item.name)
            .bind(&item.description)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn delete(&self, db: &PgPool, id: HolidayId) -> Result<(), AppError> {
        let query = format!("DELETE FROM {} WHERE id = $1", TABLE_NAME);
        sqlx::query(&query).bind(id).execute(db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holiday_select_columns_include_expected_fields() {
        assert!(SELECT_COLUMNS.contains("holiday_date"));
        assert!(SELECT_COLUMNS.contains("description"));
    }
}
