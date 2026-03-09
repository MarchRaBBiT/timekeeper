pub use crate::application::http::{bad_request, map_app_error, HttpError};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use axum::http::StatusCode;

    fn err_message(err: &HttpError) -> String {
        err.1 .0.error.clone()
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
