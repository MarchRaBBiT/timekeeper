use chrono::NaiveDate;
use sqlx::PgPool;

use crate::{
    handlers::holiday_exception_repo,
    models::holiday_exception::{CreateHolidayExceptionPayload, HolidayException},
};

#[derive(Debug)]
pub enum HolidayExceptionError {
    Conflict,
    NotFound,
    UserNotFound,
    Database(sqlx::Error),
}

#[derive(Clone)]
pub struct HolidayExceptionService {
    pool: PgPool,
}

impl HolidayExceptionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_for_user(
        &self,
        user_id: &str,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<HolidayException>, HolidayExceptionError> {
        if !self.user_exists(user_id).await? {
            return Err(HolidayExceptionError::UserNotFound);
        }
        let result =
            holiday_exception_repo::list_holiday_exceptions_for_user(&self.pool, user_id, from, to)
                .await?;
        Ok(result)
    }

    pub async fn create_workday_override(
        &self,
        user_id: &str,
        payload: CreateHolidayExceptionPayload,
        created_by: &str,
    ) -> Result<HolidayException, HolidayExceptionError> {
        if !self.user_exists(user_id).await? {
            return Err(HolidayExceptionError::UserNotFound);
        }

        let exception = HolidayException::new(
            user_id.to_string(),
            payload.exception_date,
            payload.reason,
            created_by.to_string(),
        );

        holiday_exception_repo::insert_holiday_exception(&self.pool, &exception)
            .await
            .map_err(map_unique_violation)?;

        Ok(exception)
    }

    pub async fn delete_for_user(
        &self,
        id: &str,
        user_id: &str,
    ) -> Result<(), HolidayExceptionError> {
        let affected =
            holiday_exception_repo::delete_holiday_exception(&self.pool, id, user_id).await?;

        if affected == 0 {
            Err(HolidayExceptionError::NotFound)
        } else {
            Ok(())
        }
    }
}

impl HolidayExceptionService {
    async fn user_exists(&self, user_id: &str) -> Result<bool, HolidayExceptionError> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM users WHERE id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .is_some();

        Ok(exists)
    }
}

fn map_unique_violation(err: sqlx::Error) -> HolidayExceptionError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.constraint() == Some("holiday_exceptions_user_date_key") {
            return HolidayExceptionError::Conflict;
        }
    }
    HolidayExceptionError::Database(err)
}

impl From<sqlx::Error> for HolidayExceptionError {
    fn from(value: sqlx::Error) -> Self {
        HolidayExceptionError::Database(value)
    }
}
