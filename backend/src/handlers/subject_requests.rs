use axum::{
    extract::{Extension, Path, State},
    Json,
};

use crate::{
    application::{
        clock::{Clock, SYSTEM_CLOCK},
        dto::IdStatusResponse,
        http::{map_app_error, HttpError},
    },
    models::{
        subject_request::{CreateDataSubjectRequest, DataSubjectRequestResponse},
        user::User,
    },
    requests::application::user_subject_requests::{
        cancel_subject_request as cancel_subject_request_use_case,
        create_subject_request as create_subject_request_use_case, list_user_subject_requests,
    },
    state::AppState,
};

#[cfg(test)]
use crate::requests::application::user_subject_requests::validate_details;

pub async fn create_subject_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateDataSubjectRequest>,
) -> Result<Json<DataSubjectRequestResponse>, HttpError> {
    let response = create_subject_request_use_case(
        &state.write_pool,
        user.id,
        payload,
        SYSTEM_CLOCK.now_utc(&state.config.time_zone),
    )
    .await
    .map_err(map_app_error)?;

    Ok(Json(response))
}

pub async fn list_my_subject_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<DataSubjectRequestResponse>>, HttpError> {
    let requests = list_user_subject_requests(state.read_pool(), user.id)
        .await
        .map_err(map_app_error)?;

    Ok(Json(requests))
}

pub async fn cancel_subject_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
) -> Result<Json<IdStatusResponse>, HttpError> {
    let result = cancel_subject_request_use_case(
        &state.write_pool,
        user.id,
        &request_id,
        SYSTEM_CLOCK.now_utc(&state.config.time_zone),
    )
    .await
    .map_err(map_app_error)?;

    Ok(Json(result))
}
#[cfg(test)]
mod tests {
    use super::validate_details;
    use crate::error::AppError;
    const MAX_DETAILS_LENGTH: usize = 2000;

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
        assert!(result.is_err());
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn validate_details_accepts_max_length() {
        let max_details = "a".repeat(MAX_DETAILS_LENGTH);
        let result = validate_details(Some(max_details));
        assert!(result.is_ok());
    }
}
