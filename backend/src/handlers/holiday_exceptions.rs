use std::str::FromStr;
use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;

use crate::{
    error::AppError,
    models::{
        holiday_exception::{CreateHolidayExceptionPayload, HolidayExceptionResponse},
        user::User,
    },
    services::holiday_exception::{HolidayExceptionError, HolidayExceptionServiceTrait},
    state::AppState,
    types::{HolidayExceptionId, UserId},
};

#[derive(Debug, serde::Deserialize)]
pub struct HolidayExceptionQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

pub async fn create_holiday_exception(
    State(_state): State<AppState>,
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
    State(_state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(service): Extension<Arc<dyn HolidayExceptionServiceTrait>>,
    Path(target_user_id): Path<String>,
    Query(query): Query<HolidayExceptionQuery>,
) -> Result<Json<Vec<HolidayExceptionResponse>>, AppError> {
    ensure_admin_or_system(&user)?;

    let user_id = UserId::from_str(&target_user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".into()))?;

    let exceptions = service
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
    State(_state): State<AppState>,
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
    use crate::models::user::UserRole;

    #[test]
    fn map_exception_error_handles_user_not_found() {
        let err = holiday_exception_error_to_app_error(HolidayExceptionError::UserNotFound);
        match err {
            AppError::NotFound(msg) => assert_eq!(msg, "User not found"),
            _ => panic!("Expected NotFound"),
        }
    }

    #[test]
    fn map_exception_error_handles_not_found() {
        let err = holiday_exception_error_to_app_error(HolidayExceptionError::NotFound);
        match err {
            AppError::NotFound(msg) => assert_eq!(msg, "Holiday exception not found"),
            _ => panic!("Expected NotFound"),
        }
    }

    #[test]
    fn map_exception_error_handles_conflict() {
        let err = holiday_exception_error_to_app_error(HolidayExceptionError::Conflict);
        match err {
            AppError::Conflict(msg) => assert_eq!(msg, "Holiday exception already exists for this date"),
            _ => panic!("Expected Conflict"),
        }
    }

    #[test]
    fn map_exception_error_handles_database_error() {
        let db_err = sqlx::Error::PoolTimedOut;
        let err = holiday_exception_error_to_app_error(HolidayExceptionError::Database(db_err));
        match err {
            AppError::InternalServerError(_) => (),
            _ => panic!("Expected InternalServerError"),
        }
    }

    #[test]
    fn ensure_admin_or_system_allows_admin() {
        let now = chrono::Utc::now();
        let admin_user = User {
            id: UserId::new(),
            username: "admin".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Admin".to_string(),
            email: "admin@example.com".to_string(),
            role: UserRole::Admin,
            is_system_admin: false,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            created_at: now,
            updated_at: now,
        };

        let result = ensure_admin_or_system(&admin_user);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_admin_or_system_allows_system_admin() {
        let now = chrono::Utc::now();
        let system_admin = User {
            id: UserId::new(),
            username: "sysadmin".to_string(),
            password_hash: "hash".to_string(),
            full_name: "System Admin".to_string(),
            email: "sysadmin@example.com".to_string(),
            role: UserRole::Employee,
            is_system_admin: true,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            created_at: now,
            updated_at: now,
        };

        let result = ensure_admin_or_system(&system_admin);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_admin_or_system_rejects_regular_user() {
        let now = chrono::Utc::now();
        let user = User {
            id: UserId::new(),
            username: "user".to_string(),
            password_hash: "hash".to_string(),
            full_name: "User".to_string(),
            email: "user@example.com".to_string(),
            role: UserRole::Employee,
            is_system_admin: false,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            created_at: now,
            updated_at: now,
        };

        let result = ensure_admin_or_system(&user);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(msg) => assert_eq!(msg, "Forbidden"),
            _ => panic!("Expected Forbidden error"),
        }
    }
}
