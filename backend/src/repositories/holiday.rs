//! Holiday repository.
//!
//! Provides CRUD operations for holidays.

#![allow(dead_code)]

use crate::error::AppError;
use crate::models::holiday::{AdminHolidayKind, AdminHolidayListItem, Holiday};
use crate::repositories::{common::push_clause, repository::Repository};
use crate::types::HolidayId;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use std::str::FromStr;

const TABLE_NAME: &str = "holidays";
const SELECT_COLUMNS: &str = "id, holiday_date, name, description, created_at, updated_at";

#[derive(Debug, Default, Clone, Copy)]
pub struct HolidayRepository;

impl HolidayRepository {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, FromRow)]
struct AdminHolidayRow {
    id: String,
    kind: String,
    applies_from: NaiveDate,
    applies_to: Option<NaiveDate>,
    date: Option<NaiveDate>,
    weekday: Option<i16>,
    starts_on: Option<NaiveDate>,
    ends_on: Option<NaiveDate>,
    name: Option<String>,
    description: Option<String>,
    user_id: Option<String>,
    reason: Option<String>,
    created_by: Option<String>,
    created_at: DateTime<Utc>,
    is_override: Option<bool>,
}

impl TryFrom<AdminHolidayRow> for AdminHolidayListItem {
    type Error = ();

    fn try_from(row: AdminHolidayRow) -> Result<Self, Self::Error> {
        let kind = AdminHolidayKind::from_str(&row.kind)?;
        Ok(Self {
            id: row.id,
            kind,
            applies_from: row.applies_from,
            applies_to: row.applies_to,
            date: row.date,
            weekday: row.weekday,
            starts_on: row.starts_on,
            ends_on: row.ends_on,
            name: row.name,
            description: row.description,
            user_id: row.user_id,
            reason: row.reason,
            created_by: row.created_by,
            created_at: row.created_at,
            is_override: row.is_override,
        })
    }
}

fn apply_holiday_filters(
    builder: &mut QueryBuilder<'_, Postgres>,
    has_clause: &mut bool,
    kind: Option<AdminHolidayKind>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) {
    if let Some(kind) = kind {
        push_clause(builder, has_clause);
        builder.push("kind = ").push_bind(kind.as_str());
    }
    if let Some(from) = from {
        push_clause(builder, has_clause);
        builder.push("applies_from >= ").push_bind(from);
    }
    if let Some(to) = to {
        push_clause(builder, has_clause);
        builder.push("applies_from <= ").push_bind(to);
    }
}

impl HolidayRepository {
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

    pub async fn list_paginated_admin(
        &self,
        pool: &PgPool,
        kind: Option<AdminHolidayKind>,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        per_page: i64,
        offset: i64,
    ) -> Result<(Vec<AdminHolidayListItem>, i64), AppError> {
        let mut data_builder = QueryBuilder::new(
            r#"
        WITH unioned AS (
            SELECT id,
                   'public'::text AS kind,
                   holiday_date AS applies_from,
                   holiday_date AS applies_to,
                   holiday_date AS date,
                   NULL::smallint AS weekday,
                   NULL::date AS starts_on,
                   NULL::date AS ends_on,
                   name,
                   description,
                   NULL::text AS user_id,
                   description AS reason,
                   NULL::text AS created_by,
                   created_at,
                   NULL::boolean AS is_override
            FROM holidays
            UNION ALL
            SELECT id,
                   'weekly'::text AS kind,
                   enforced_from AS applies_from,
                   enforced_to AS applies_to,
                   NULL::date AS date,
                   weekday,
                   starts_on,
                   ends_on,
                   NULL::text AS name,
                   NULL::text AS description,
                   NULL::text AS user_id,
                   NULL::text AS reason,
                   created_by,
                   created_at,
                   NULL::boolean AS is_override
            FROM weekly_holidays
            UNION ALL
            SELECT id,
                   'exception'::text AS kind,
                   exception_date AS applies_from,
                   exception_date AS applies_to,
                   exception_date AS date,
                   NULL::smallint AS weekday,
                   NULL::date AS starts_on,
                   NULL::date AS ends_on,
                   NULL::text AS name,
                   NULL::text AS description,
                   user_id,
                   reason,
                   created_by,
                   created_at,
                   override AS is_override
            FROM holiday_exceptions
        )
        SELECT id, kind, applies_from, applies_to, date, weekday, starts_on, ends_on,
               name, description, user_id, reason, created_by, created_at, is_override
        FROM unioned
        "#,
        );

        let mut data_has_clause = false;
        apply_holiday_filters(&mut data_builder, &mut data_has_clause, kind, from, to);

        data_builder
            .push(" ORDER BY applies_from DESC, kind ASC, created_at DESC")
            .push(" LIMIT ")
            .push_bind(per_page)
            .push(" OFFSET ")
            .push_bind(offset);

        let mut count_builder = QueryBuilder::new(
            r#"
        SELECT COUNT(*) FROM (
            WITH unioned AS (
                SELECT id,
                       'public'::text AS kind,
                       holiday_date AS applies_from
                FROM holidays
            UNION ALL
                SELECT id,
                       'weekly'::text AS kind,
                       enforced_from AS applies_from
                FROM weekly_holidays
            UNION ALL
                SELECT id,
                       'exception'::text AS kind,
                       exception_date AS applies_from
                FROM holiday_exceptions
            )
            SELECT 1
            FROM unioned
        "#,
        );

        let mut count_has_clause = false;
        apply_holiday_filters(&mut count_builder, &mut count_has_clause, kind, from, to);
        count_builder.push(") AS counted");

        let rows = data_builder
            .build_query_as::<AdminHolidayRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;

        let total = count_builder
            .build_query_scalar::<i64>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;

        let items = rows
            .into_iter()
            .map(AdminHolidayListItem::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::InternalServerError(anyhow::anyhow!("Invalid holiday kind")))?;

        Ok((items, total))
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
