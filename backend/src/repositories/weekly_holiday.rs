//! Weekly holiday repository.
//!
//! Provides CRUD operations for weekly holidays.

#![allow(dead_code)]

use crate::error::AppError;
use crate::models::holiday::WeeklyHoliday;
use crate::repositories::repository::Repository;
use crate::types::WeeklyHolidayId;
use sqlx::{PgPool, Row};

const TABLE_NAME: &str = "weekly_holidays";
const SELECT_COLUMNS: &str = "id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at";

#[derive(Debug, Default, Clone, Copy)]
pub struct WeeklyHolidayRepository;

impl WeeklyHolidayRepository {
    pub fn new() -> Self {
        Self
    }

    fn base_select_query() -> String {
        format!("SELECT {} FROM {}", SELECT_COLUMNS, TABLE_NAME)
    }
}

impl Repository<WeeklyHoliday> for WeeklyHolidayRepository {
    const TABLE: &'static str = TABLE_NAME;
    type Id = WeeklyHolidayId;

    async fn find_all(&self, db: &PgPool) -> Result<Vec<WeeklyHoliday>, AppError> {
        let query = format!("{} ORDER BY enforced_from, weekday", Self::base_select_query());
        let rows = sqlx::query_as::<_, WeeklyHoliday>(&query).fetch_all(db).await?;
        Ok(rows)
    }

    async fn find_by_id(&self, db: &PgPool, id: WeeklyHolidayId) -> Result<WeeklyHoliday, AppError> {
        let query = format!("{} WHERE id = $1", Self::base_select_query());
        let result = sqlx::query_as::<_, WeeklyHoliday>(&query)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Weekly holiday not found".into()))?;
        Ok(result)
    }

    async fn create(&self, db: &PgPool, item: &WeeklyHoliday) -> Result<WeeklyHoliday, AppError> {
        let query = format!(
            "INSERT INTO {} (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, WeeklyHoliday>(&query)
            .bind(item.id)
            .bind(item.weekday)
            .bind(item.starts_on)
            .bind(item.ends_on)
            .bind(item.enforced_from)
            .bind(item.enforced_to)
            .bind(item.created_by)
            .bind(item.created_at)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn update(&self, db: &PgPool, item: &WeeklyHoliday) -> Result<WeeklyHoliday, AppError> {
        let query = format!(
            "UPDATE {} SET weekday = $2, starts_on = $3, ends_on = $4, enforced_from = $5, enforced_to = $6, updated_at = $7 \
             WHERE id = $1 RETURNING {}",
            TABLE_NAME, SELECT_COLUMNS
        );
        let row = sqlx::query_as::<_, WeeklyHoliday>(&query)
            .bind(item.id)
            .bind(item.weekday)
            .bind(item.starts_on)
            .bind(item.ends_on)
            .bind(item.enforced_from)
            .bind(item.enforced_to)
            .bind(item.updated_at)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    async fn delete(&self, db: &PgPool, id: WeeklyHolidayId) -> Result<(), AppError> {
        let query = format!("DELETE FROM {} WHERE id = $1", TABLE_NAME);
        let result = sqlx::query(&query).bind(id).execute(db).await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Weekly holiday not found".into()));
        }
        Ok(())
    }
}
