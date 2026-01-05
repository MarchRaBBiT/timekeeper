use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    BadRequest(String),
    InternalServerError(anyhow::Error),
    Validation(Vec<String>),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, code, details) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg, "NOT_FOUND".to_string(), None),
            AppError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                msg,
                "UNAUTHORIZED".to_string(),
                None,
            ),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg, "FORBIDDEN".to_string(), None),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg, "CONFLICT".to_string(), None),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                msg,
                "BAD_REQUEST".to_string(),
                None,
            ),
            AppError::InternalServerError(err) => {
                tracing::error!("Internal server error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                    "INTERNAL_SERVER_ERROR".to_string(),
                    None,
                )
            }
            AppError::Validation(errors) => (
                StatusCode::BAD_REQUEST,
                "Validation failed".to_string(),
                "VALIDATION_ERROR".to_string(),
                Some(serde_json::json!({ "errors": errors })),
            ),
        };

        let body = Json(ErrorResponse {
            error: error_message,
            code,
            details,
        });

        (status, body).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::InternalServerError(err)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".to_string()),
            _ => AppError::InternalServerError(err.into()),
        }
    }
}
