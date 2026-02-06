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

impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let messages: Vec<String> = errors
            .field_errors()
            .into_iter()
            .flat_map(|(field, errs)| {
                errs.iter().map(move |e| {
                    let code = e.code.as_ref();
                    format!("{}: {}", field, code)
                })
            })
            .collect();
        AppError::Validation(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn response_json(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("json")
    }

    #[tokio::test]
    async fn app_error_into_response_maps_status_and_body() {
        let response = AppError::BadRequest("bad".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = response_json(response).await;
        assert_eq!(json["error"], "bad");
        assert_eq!(json["code"], "BAD_REQUEST");

        let response = AppError::Unauthorized("nope".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = response_json(response).await;
        assert_eq!(json["error"], "nope");
        assert_eq!(json["code"], "UNAUTHORIZED");

        let response = AppError::Forbidden("denied".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = response_json(response).await;
        assert_eq!(json["error"], "denied");
        assert_eq!(json["code"], "FORBIDDEN");

        let response = AppError::Conflict("conflict".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let json = response_json(response).await;
        assert_eq!(json["error"], "conflict");
        assert_eq!(json["code"], "CONFLICT");

        let response = AppError::NotFound("missing".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = response_json(response).await;
        assert_eq!(json["error"], "missing");
        assert_eq!(json["code"], "NOT_FOUND");
    }

    #[tokio::test]
    async fn app_error_validation_includes_details() {
        let response = AppError::Validation(vec!["field: invalid".to_string()]).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = response_json(response).await;
        assert_eq!(json["error"], "Validation failed");
        assert_eq!(json["code"], "VALIDATION_ERROR");
        assert_eq!(json["details"]["errors"][0], "field: invalid");
    }

    #[tokio::test]
    async fn app_error_internal_maps_to_generic_message() {
        let response = AppError::InternalServerError(anyhow::anyhow!("boom")).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let json = response_json(response).await;
        assert_eq!(json["error"], "Internal server error");
        assert_eq!(json["code"], "INTERNAL_SERVER_ERROR");
        assert!(json["details"].is_null());
    }
}
