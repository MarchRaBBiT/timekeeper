use std::sync::Arc;
use std::str::FromStr;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use sqlx::PgPool;

use crate::{
    config::Config,
    error::AppError,
    models::{
        holiday_exception::{CreateHolidayExceptionPayload, HolidayExceptionResponse},
        user::User,
    },
    services::holiday_exception::{HolidayExceptionError, HolidayExceptionServiceTrait},
    types::{HolidayExceptionId, UserId},
};

#[derive(Debug, serde::Deserialize)]
pub struct HolidayExceptionQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

pub async fn create_holiday_exception(
    State((_pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<dyn HolidayExceptionServiceTrait>>,
    Path(target_user_id): Path<String>,
    Json(payload): Json<CreateHolidayExceptionPayload>,
) -> Result<(StatusCode, Json<HolidayExceptionResponse>), AppError> {
    ensure_admin_or_system(&user)?;

    let user_id = UserId::from_str(&target_user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".into()))?;

    let created = service
        .create_workday_override(user_id, payload, user.id)
        .await
        .map_err(holiday_exception_error_to_app_error)?;

    Ok((
        StatusCode::CREATED,
        Json(HolidayExceptionResponse::from(created)),
    ))
}

pub async fn list_holiday_exceptions(
    State((_pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<dyn HolidayExceptionServiceTrait>>,
    Path(target_user_id): Path<String>,
    Query(query): Query<HolidayExceptionQuery>,
) -> Result<Json<Vec<HolidayExceptionResponse>>, AppError> {
    ensure_admin_or_system(&user)?;

    let user_id = UserId::from_str(&target_user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".into()))?;

    let exceptions =service
        .list_for_user(user_id, query.from, query.to)
        .await
        .map_err(holiday_exception_error_to_app_error)?;

    let response = exceptions
        .into_iter()
        .map(HolidayExceptionResponse::from)
        .collect();

    Ok(Json(response))
}

pub async fn delete_holiday_exception(
    State((_pool, _config)): State<(PgPool, Config)>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<dyn HolidayExceptionServiceTrait>>,
    Path((target_user_id, id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    ensure_admin_or_system(&user)?;

    let exception_id = HolidayExceptionId::from_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid exception ID format".into()))?;
    let user_id = UserId::from_str(&target_user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".into()))?;

    service
        .delete_for_user(exception_id, user_id)
        .await
        .map_err(holiday_exception_error_to_app_error)?;

    Ok(StatusCode::NO_CONTENT)
}

fn holiday_exception_error_to_app_error(error: HolidayExceptionError) -> AppError {
    match error {
        HolidayExceptionError::Conflict => {
            AppError::Conflict("Holiday exception already exists for this date".into())
        }
        HolidayExceptionError::NotFound => AppError::NotFound("Holiday exception not found".into()),
        HolidayExceptionError::UserNotFound => AppError::NotFound("User not found".into()),
        HolidayExceptionError::Database(err) => AppError::InternalServerError(err.into()),
    }
}

fn ensure_admin_or_system(user: &User) -> Result<(), AppError> {
    if user.is_admin() || user.is_system_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden("Forbidden".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_exception_error_handles_user_not_found() {
        let err = holiday_exception_error_to_app_error(HolidayExceptionError::UserNotFound);
        match err {
            AppError::NotFound(msg) => assert_eq!(msg, "User not found"),
            _ => panic!("Expected NotFound"),
        }
    }
}
