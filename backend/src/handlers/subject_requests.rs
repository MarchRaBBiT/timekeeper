use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

#[cfg(test)]
use crate::requests::application::user_subject_requests::validate_details as validate_details_value;

use crate::{
    models::{
        subject_request::{CreateDataSubjectRequest, DataSubjectRequestResponse},
        user::User,
    },
    requests::application::user_subject_requests::{
        cancel_subject_request as cancel_subject_request_use_case,
        create_subject_request as create_subject_request_use_case, list_user_subject_requests,
    },
    state::AppState,
    utils::time,
};

pub async fn create_subject_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateDataSubjectRequest>,
) -> Result<Json<DataSubjectRequestResponse>, (StatusCode, Json<Value>)> {
    let response = create_subject_request_use_case(
        &state.write_pool,
        user.id,
        payload,
        time::now_utc(&state.config.time_zone),
    )
    .await
    .map_err(map_app_error)?;

    Ok(Json(response))
}

pub async fn list_my_subject_requests(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<DataSubjectRequestResponse>>, (StatusCode, Json<Value>)> {
    let requests = list_user_subject_requests(state.read_pool(), user.id)
        .await
        .map_err(map_app_error)?;

    Ok(Json(requests))
}

pub async fn cancel_subject_request(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(request_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let result = cancel_subject_request_use_case(
        &state.write_pool,
        user.id,
        &request_id,
        time::now_utc(&state.config.time_zone),
    )
    .await
    .map_err(map_app_error)?;

    Ok(Json(json!(result)))
}

#[cfg(test)]
#[allow(dead_code)]
fn validate_details(details: Option<String>) -> Result<Option<String>, (StatusCode, Json<Value>)> {
    validate_details_value(details).map_err(map_app_error)
}

fn map_app_error(err: crate::error::AppError) -> (StatusCode, Json<Value>) {
    match err {
        crate::error::AppError::BadRequest(message) => {
            (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
        }
        crate::error::AppError::Forbidden(message) => {
            (StatusCode::FORBIDDEN, Json(json!({ "error": message })))
        }
        crate::error::AppError::Unauthorized(message) => {
            (StatusCode::UNAUTHORIZED, Json(json!({ "error": message })))
        }
        crate::error::AppError::Conflict(message) => {
            (StatusCode::CONFLICT, Json(json!({ "error": message })))
        }
        crate::error::AppError::NotFound(message) => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": message })))
        }
        crate::error::AppError::Validation(errors) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Validation failed", "details": { "errors": errors } })),
        ),
        crate::error::AppError::InternalServerError(err) => {
            tracing::error!(error = %err, "internal server error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal server error" })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validate_details_accepts_max_length() {
        let max_details = "a".repeat(MAX_DETAILS_LENGTH);
        let result = validate_details(Some(max_details));
        assert!(result.is_ok());
    }
}
