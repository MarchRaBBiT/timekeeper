use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    error::AppError,
    models::subject_request::{
        CreateDataSubjectRequest, DataSubjectRequest, DataSubjectRequestResponse,
    },
    repositories::subject_request,
    types::UserId,
};

const MAX_DETAILS_LENGTH: usize = 2000;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CancelSubjectRequestResult {
    pub id: String,
    pub status: &'static str,
}

pub async fn create_subject_request(
    pool: &sqlx::PgPool,
    user_id: UserId,
    payload: CreateDataSubjectRequest,
    now: DateTime<Utc>,
) -> Result<DataSubjectRequestResponse, AppError> {
    let details = validate_details(payload.details)?;
    let request = DataSubjectRequest::new(user_id.to_string(), payload.request_type, details, now);

    subject_request::insert_subject_request(pool, &request)
        .await
        .map_err(|err| AppError::InternalServerError(err.into()))?;

    Ok(DataSubjectRequestResponse::from(request))
}

pub async fn list_user_subject_requests(
    pool: &sqlx::PgPool,
    user_id: UserId,
) -> Result<Vec<DataSubjectRequestResponse>, AppError> {
    let requests = subject_request::list_subject_requests_by_user(pool, &user_id.to_string())
        .await
        .map_err(|err| AppError::InternalServerError(err.into()))?;

    Ok(requests
        .into_iter()
        .map(DataSubjectRequestResponse::from)
        .collect())
}

pub async fn cancel_subject_request(
    pool: &sqlx::PgPool,
    user_id: UserId,
    request_id: &str,
    now: DateTime<Utc>,
) -> Result<CancelSubjectRequestResult, AppError> {
    let rows = subject_request::cancel_subject_request(pool, request_id, &user_id.to_string(), now)
        .await
        .map_err(|err| AppError::InternalServerError(err.into()))?;

    if rows == 0 {
        return Err(AppError::NotFound(
            "Request not found or not cancellable".into(),
        ));
    }

    Ok(CancelSubjectRequestResult {
        id: request_id.to_string(),
        status: "cancelled",
    })
}

pub fn validate_details(details: Option<String>) -> Result<Option<String>, AppError> {
    if let Some(details) = details {
        let trimmed = details.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        if trimmed.chars().count() > MAX_DETAILS_LENGTH {
            return Err(AppError::BadRequest("details is too long".into()));
        }
        return Ok(Some(trimmed.to_string()));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::subject_request::DataSubjectRequestType;

    #[test]
    fn validate_details_returns_none_for_none() {
        let result = validate_details(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn validate_details_returns_none_for_empty() {
        let result = validate_details(Some("".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn validate_details_returns_none_for_whitespace_only() {
        let result = validate_details(Some("   ".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn validate_details_accepts_valid_details() {
        let result = validate_details(Some("Valid details".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("Valid details".to_string()));
    }

    #[test]
    fn validate_details_trims_whitespace() {
        let result = validate_details(Some("  test details  ".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("test details".to_string()));
    }

    #[test]
    fn validate_details_rejects_too_long() {
        let long_details = "a".repeat(MAX_DETAILS_LENGTH + 1);
        let result = validate_details(Some(long_details));
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn validate_details_accepts_max_length() {
        let max_details = "a".repeat(MAX_DETAILS_LENGTH);
        let result = validate_details(Some(max_details));
        assert!(result.is_ok());
    }

    #[test]
    fn cancel_subject_request_result_keeps_cancelled_status() {
        let result = CancelSubjectRequestResult {
            id: "req-1".to_string(),
            status: "cancelled",
        };
        assert_eq!(result.id, "req-1");
        assert_eq!(result.status, "cancelled");
    }

    #[test]
    fn create_payload_type_is_compatible() {
        let payload = CreateDataSubjectRequest {
            request_type: DataSubjectRequestType::Access,
            details: Some("details".to_string()),
        };
        assert!(matches!(
            payload.request_type,
            DataSubjectRequestType::Access
        ));
    }
}
