use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

use crate::error::AppError;

pub fn map_app_error(err: AppError) -> (StatusCode, Json<Value>) {
    match err {
        AppError::BadRequest(message) => bad_request(message.as_str()),
        AppError::Forbidden(message) => (StatusCode::FORBIDDEN, Json(json!({ "error": message }))),
        AppError::Unauthorized(message) => {
            (StatusCode::UNAUTHORIZED, Json(json!({ "error": message })))
        }
        AppError::Conflict(message) => (StatusCode::CONFLICT, Json(json!({ "error": message }))),
        AppError::NotFound(message) => (StatusCode::NOT_FOUND, Json(json!({ "error": message }))),
        AppError::Validation(errors) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Validation failed", "details": { "errors": errors } })),
        ),
        AppError::InternalServerError(err) => {
            tracing::error!(error = %err, "internal server error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal server error" })),
            )
        }
    }
}

pub fn bad_request(message: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err_message(err: &(StatusCode, Json<Value>)) -> String {
        err.1
             .0
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn map_app_error_maps_variants() {
        let bad = map_app_error(AppError::BadRequest("bad".to_string()));
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
        assert_eq!(err_message(&bad), "bad");

        let forbidden = map_app_error(AppError::Forbidden("forbidden".to_string()));
        assert_eq!(forbidden.0, StatusCode::FORBIDDEN);

        let internal = map_app_error(AppError::InternalServerError(anyhow::anyhow!("boom")));
        assert_eq!(internal.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err_message(&internal), "Internal server error");
    }

    #[test]
    fn bad_request_builds_payload() {
        let err = bad_request("working day");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err_message(&err), "working day");
    }
}
