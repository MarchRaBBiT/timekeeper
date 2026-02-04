use axum::{
    extract::Request,
    http::{header::HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

const REQUEST_ID_HEADER: &str = "x-request-id";
const CORRELATION_ID_HEADER: &str = "x-correlation-id";

#[derive(Clone, Debug)]
pub struct RequestId(pub String);

pub async fn request_id(mut req: Request, next: Next) -> Response {
    let header_name = HeaderName::from_static(REQUEST_ID_HEADER);

    let id = req
        .headers()
        .get(&header_name)
        .or_else(|| {
            req.headers()
                .get(HeaderName::from_static(CORRELATION_ID_HEADER))
        })
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let request_id = RequestId(id.clone());
    req.extensions_mut().insert(request_id.clone());

    let mut response = next.run(req).await;

    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(header_name, value);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn request_id_generates_new_id_when_not_provided() {
        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(request_id));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("x-request-id").is_some());
    }

    #[tokio::test]
    async fn request_id_uses_provided_x_request_id() {
        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(request_id));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .header("x-request-id", "test-id-123")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("x-request-id").unwrap(),
            "test-id-123"
        );
    }

    #[tokio::test]
    async fn request_id_falls_back_to_correlation_id() {
        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(request_id));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .header("x-correlation-id", "corr-id-456")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("x-request-id").unwrap(),
            "corr-id-456"
        );
    }

    #[tokio::test]
    async fn request_id_prefers_x_request_id_over_correlation_id() {
        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(request_id));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .header("x-request-id", "req-id")
                    .header("x-correlation-id", "corr-id")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("x-request-id").unwrap(), "req-id");
    }

    #[test]
    fn request_id_struct_has_inner_string() {
        let req_id = RequestId("test-id".to_string());
        assert_eq!(req_id.0, "test-id");
    }
}
