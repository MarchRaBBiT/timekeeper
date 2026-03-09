use std::str::FromStr;

use chrono::NaiveDate;

use crate::{
    application::http::forbidden_error,
    error::AppError,
    models::{
        holiday_exception::{CreateHolidayExceptionPayload, HolidayExceptionResponse},
        user::User,
    },
    services::holiday_exception::{HolidayExceptionError, HolidayExceptionServiceTrait},
    types::{HolidayExceptionId, UserId},
};

pub async fn create_holiday_exception(
    service: &dyn HolidayExceptionServiceTrait,
    user: &User,
    target_user_id: &str,
    payload: CreateHolidayExceptionPayload,
) -> Result<HolidayExceptionResponse, AppError> {
    ensure_admin_or_system(user)?;

    let user_id = parse_user_id(target_user_id)?;
    let created = service
        .create_workday_override(user_id, payload, user.id)
        .await
        .map_err(holiday_exception_error_to_app_error)?;

    Ok(HolidayExceptionResponse::from(created))
}

pub async fn list_holiday_exceptions(
    service: &dyn HolidayExceptionServiceTrait,
    user: &User,
    target_user_id: &str,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<Vec<HolidayExceptionResponse>, AppError> {
    ensure_admin_or_system(user)?;

    let user_id = parse_user_id(target_user_id)?;
    let exceptions = service
        .list_for_user(user_id, from, to)
        .await
        .map_err(holiday_exception_error_to_app_error)?;

    Ok(exceptions
        .into_iter()
        .map(HolidayExceptionResponse::from)
        .collect())
}

pub async fn delete_holiday_exception(
    service: &dyn HolidayExceptionServiceTrait,
    user: &User,
    target_user_id: &str,
    id: &str,
) -> Result<(), AppError> {
    ensure_admin_or_system(user)?;

    let exception_id = parse_exception_id(id)?;
    let user_id = parse_user_id(target_user_id)?;

    service
        .delete_for_user(exception_id, user_id)
        .await
        .map_err(holiday_exception_error_to_app_error)
}

pub fn holiday_exception_error_to_app_error(error: HolidayExceptionError) -> AppError {
    match error {
        HolidayExceptionError::Conflict => {
            AppError::Conflict("Holiday exception already exists for this date".into())
        }
        HolidayExceptionError::NotFound => AppError::NotFound("Holiday exception not found".into()),
        HolidayExceptionError::UserNotFound => AppError::NotFound("User not found".into()),
        HolidayExceptionError::Database(err) => AppError::InternalServerError(err.into()),
    }
}

pub fn ensure_admin_or_system(user: &User) -> Result<(), AppError> {
    if user.is_admin() || user.is_system_admin() {
        Ok(())
    } else {
        Err(forbidden_error("Forbidden"))
    }
}

fn parse_user_id(target_user_id: &str) -> Result<UserId, AppError> {
    UserId::from_str(target_user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".into()))
}

fn parse_exception_id(id: &str) -> Result<HolidayExceptionId, AppError> {
    HolidayExceptionId::from_str(id)
        .map_err(|_| AppError::BadRequest("Invalid exception ID format".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            holiday_exception::{CreateHolidayExceptionPayload, HolidayException},
            user::UserRole,
        },
        services::holiday_exception::HolidayExceptionServiceTrait,
    };
    use chrono::Utc;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockHolidayExceptionService {
        list_result: Mutex<Option<Result<Vec<HolidayException>, HolidayExceptionError>>>,
        create_result: Mutex<Option<Result<HolidayException, HolidayExceptionError>>>,
        delete_result: Mutex<Option<Result<(), HolidayExceptionError>>>,
    }

    #[async_trait::async_trait]
    impl HolidayExceptionServiceTrait for MockHolidayExceptionService {
        async fn list_for_user(
            &self,
            _user_id: UserId,
            _from: Option<NaiveDate>,
            _to: Option<NaiveDate>,
        ) -> Result<Vec<HolidayException>, HolidayExceptionError> {
            self.list_result
                .lock()
                .expect("lock list result")
                .take()
                .expect("list result configured")
        }

        async fn create_workday_override(
            &self,
            _user_id: UserId,
            _payload: CreateHolidayExceptionPayload,
            _created_by: UserId,
        ) -> Result<HolidayException, HolidayExceptionError> {
            self.create_result
                .lock()
                .expect("lock create result")
                .take()
                .expect("create result configured")
        }

        async fn delete_for_user(
            &self,
            _id: HolidayExceptionId,
            _user_id: UserId,
        ) -> Result<(), HolidayExceptionError> {
            self.delete_result
                .lock()
                .expect("lock delete result")
                .take()
                .expect("delete result configured")
        }
    }

    fn sample_user(role: UserRole, is_system_admin: bool) -> User {
        let now = Utc::now();
        User {
            id: UserId::new(),
            username: "user".to_string(),
            password_hash: "hash".to_string(),
            full_name: "User".to_string(),
            email: "user@example.com".to_string(),
            role,
            is_system_admin,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            failed_login_attempts: 0,
            locked_until: None,
            lock_reason: None,
            lockout_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

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
            AppError::Conflict(msg) => {
                assert_eq!(msg, "Holiday exception already exists for this date")
            }
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
        let result = ensure_admin_or_system(&sample_user(UserRole::Admin, false));
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_admin_or_system_allows_system_admin() {
        let result = ensure_admin_or_system(&sample_user(UserRole::Employee, true));
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_admin_or_system_rejects_regular_user() {
        let result = ensure_admin_or_system(&sample_user(UserRole::Employee, false));
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(msg) => assert_eq!(msg, "Forbidden"),
            _ => panic!("Expected Forbidden error"),
        }
    }

    #[tokio::test]
    async fn create_holiday_exception_rejects_invalid_user_id() {
        let service = MockHolidayExceptionService::default();
        let err = create_holiday_exception(
            &service,
            &sample_user(UserRole::Admin, false),
            "not-a-user-id",
            CreateHolidayExceptionPayload {
                exception_date: NaiveDate::from_ymd_opt(2026, 3, 9).expect("valid date"),
                reason: Some("reason".to_string()),
            },
        )
        .await
        .expect_err("invalid user id should fail");

        match err {
            AppError::BadRequest(msg) => assert_eq!(msg, "Invalid user ID format"),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[tokio::test]
    async fn delete_holiday_exception_rejects_invalid_exception_id() {
        let service = MockHolidayExceptionService::default();
        let user = sample_user(UserRole::Admin, false);
        let target_user_id = UserId::new().to_string();

        let err = delete_holiday_exception(&service, &user, &target_user_id, "not-an-id")
            .await
            .expect_err("invalid exception id should fail");

        match err {
            AppError::BadRequest(msg) => assert_eq!(msg, "Invalid exception ID format"),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[tokio::test]
    async fn list_holiday_exceptions_maps_service_response() {
        let user_id = UserId::new();
        let exception = HolidayException::new(
            user_id,
            NaiveDate::from_ymd_opt(2026, 3, 9).expect("valid date"),
            Some("Make-up day".to_string()),
            UserId::new(),
        );
        let service = MockHolidayExceptionService {
            list_result: Mutex::new(Some(Ok(vec![exception.clone()]))),
            create_result: Mutex::new(None),
            delete_result: Mutex::new(None),
        };

        let result = list_holiday_exceptions(
            &service,
            &sample_user(UserRole::Admin, false),
            &user_id.to_string(),
            None,
            None,
        )
        .await
        .expect("list should succeed");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, exception.id);
        assert_eq!(result[0].exception_date, exception.exception_date);
        assert_eq!(result[0].is_workday, exception.is_workday());
        assert_eq!(result[0].reason, exception.reason);
    }
}
