use axum::{http::StatusCode, Json};
use serde_json::json;

use crate::{application::dto::ErrorResponse, error::AppError};

pub type HttpError = (StatusCode, Json<ErrorResponse>);

pub fn error(status: StatusCode, message: impl Into<String>) -> HttpError {
    (status, Json(ErrorResponse::new(message)))
}

pub fn bad_request(message: impl Into<String>) -> HttpError {
    error(StatusCode::BAD_REQUEST, message)
}

pub fn forbidden(message: impl Into<String>) -> HttpError {
    error(StatusCode::FORBIDDEN, message)
}

pub fn forbidden_error(message: impl Into<String>) -> AppError {
    AppError::Forbidden(message.into())
}

pub fn unauthorized(message: impl Into<String>) -> HttpError {
    error(StatusCode::UNAUTHORIZED, message)
}

pub fn conflict(message: impl Into<String>) -> HttpError {
    error(StatusCode::CONFLICT, message)
}

pub fn not_found(message: impl Into<String>) -> HttpError {
    error(StatusCode::NOT_FOUND, message)
}

pub fn internal_server_error(err: &anyhow::Error) -> HttpError {
    tracing::error!(error = %err, "internal server error");
    error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
}

pub fn map_app_error(err: AppError) -> HttpError {
    match err {
        AppError::BadRequest(message) => bad_request(message),
        AppError::Forbidden(message) => forbidden(message),
        AppError::Unauthorized(message) => unauthorized(message),
        AppError::Conflict(message) => conflict(message),
        AppError::NotFound(message) => not_found(message),
        AppError::Validation(errors) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_details(
                "Validation failed",
                json!({ "errors": errors }),
            )),
        ),
        AppError::InternalServerError(err) => internal_server_error(&err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_app_error_maps_variants() {
        let bad = map_app_error(AppError::BadRequest("bad".to_string()));
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
        assert_eq!(bad.1 .0.error, "bad");

        let forbidden = map_app_error(AppError::Forbidden("forbidden".to_string()));
        assert_eq!(forbidden.0, StatusCode::FORBIDDEN);
        assert_eq!(forbidden.1 .0.error, "forbidden");

        let internal = map_app_error(AppError::InternalServerError(anyhow::anyhow!("boom")));
        assert_eq!(internal.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(internal.1 .0.error, "Internal server error");
    }

    #[test]
    fn validation_error_keeps_details() {
        let err = map_app_error(AppError::Validation(vec!["missing field".to_string()]));
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err.1 .0.error, "Validation failed");
        assert!(err.1 .0.details.is_some());
    }

    #[test]
    fn forbidden_error_builds_app_error() {
        assert!(matches!(
            forbidden_error("Forbidden"),
            AppError::Forbidden(message) if message == "Forbidden"
        ));
    }
}
