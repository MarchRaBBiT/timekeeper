use axum::{
    body::{to_bytes, Body, Bytes},
    http::{header::CONTENT_LENGTH, Request},
    middleware::Next,
    response::Response,
    Error as AxumError,
};
use std::time::Instant;

const MAX_BUFFERED_BODY_BYTES: usize = 64 * 1024;
const MAX_LOGGED_BODY_BYTES: usize = 2048;

/// Middleware that records detailed diagnostics whenever a handler returns an
/// HTTP status in the 4xx or 5xx range. The response body is buffered so the
/// same payload can still be forwarded to the caller after logging.
pub async fn log_error_responses(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let uri = req.uri().to_string();
    let version = format!("{:?}", req.version());
    let start = Instant::now();

    let response = next.run(req).await;
    let status = response.status();

    if !(status.is_client_error() || status.is_server_error()) {
        return response;
    }

    let latency = start.elapsed();
    let (mut parts, body) = response.into_parts();
    match buffer_body(body).await {
        Ok((bytes, truncated_preview)) => {
            log_error_event(
                status.as_u16(),
                &method,
                &uri,
                &version,
                latency.as_millis() as u64,
                &truncated_preview,
                None,
            );

            Response::from_parts(parts, Body::from(bytes))
        }
        Err(err) => {
            parts.headers.remove(CONTENT_LENGTH);
            log_error_event(
                status.as_u16(),
                &method,
                &uri,
                &version,
                latency.as_millis() as u64,
                "",
                Some(err),
            );
            Response::from_parts(parts, Body::empty())
        }
    }
}

async fn buffer_body(body: Body) -> Result<(Bytes, String), AxumError> {
    let bytes = to_bytes(body, MAX_BUFFERED_BODY_BYTES).await?;
    let preview = if bytes.len() > MAX_LOGGED_BODY_BYTES {
        let slice = bytes.slice(0..MAX_LOGGED_BODY_BYTES);
        format!(
            "{}... (truncated, {} bytes total)",
            String::from_utf8_lossy(&slice),
            bytes.len()
        )
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };
    Ok((bytes, preview))
}

fn log_error_event(
    status: u16,
    method: &str,
    uri: &str,
    version: &str,
    latency_ms: u64,
    body_preview: &str,
    body_error: Option<AxumError>,
) {
    if let Some(err) = body_error {
        if status >= 500 {
            tracing::error!(
                status,
                method,
                uri,
                version,
                latency_ms,
                error = ?err,
                "Failed to read error response body"
            );
        } else {
            tracing::warn!(
                status,
                method,
                uri,
                version,
                latency_ms,
                error = ?err,
                "Failed to read error response body"
            );
        }
        return;
    }

    if status >= 500 {
        tracing::error!(
            status,
            method,
            uri,
            version,
            latency_ms,
            body = body_preview,
            "Request completed with error status"
        );
    } else {
        tracing::warn!(
            status,
            method,
            uri,
            version,
            latency_ms,
            body = body_preview,
            "Request completed with error status"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, middleware::from_fn, routing::get, Router};
    use tower::ServiceExt;

    #[tokio::test]
    async fn buffer_body_returns_bytes_and_preview() {
        let body = Body::from("test body");
        let (bytes, preview) = buffer_body(body).await.unwrap();
        assert_eq!(bytes, b"test body"[..]);
        assert_eq!(preview, "test body");
    }

    #[tokio::test]
    async fn buffer_body_truncates_large_body() {
        let large_body = "x".repeat(10000);
        let body = Body::from(large_body.clone());
        let (bytes, preview) = buffer_body(body).await.unwrap();
        assert_eq!(bytes.len(), 10000);
        assert!(preview.starts_with("x"));
        assert!(preview.contains("... (truncated"));
        assert!(preview.contains("10000 bytes total"));
    }

    #[tokio::test]
    async fn buffer_body_handles_empty_body() {
        let body = Body::empty();
        let (bytes, preview) = buffer_body(body).await.unwrap();
        assert_eq!(bytes.len(), 0);
        assert_eq!(preview, "");
    }

    #[tokio::test]
    async fn buffer_body_handles_invalid_utf8() {
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let body = Body::from(invalid_bytes);
        let (bytes, preview) = buffer_body(body).await.unwrap();
        assert_eq!(bytes.len(), 3);
        assert!(preview.contains("\u{FFFD}"));
    }

    #[tokio::test]
    async fn buffer_body_exceeds_max_size() {
        let huge_body = vec![0u8; 100000];
        let body = Body::from(huge_body);
        let result = buffer_body(body).await;
        assert!(result.is_err());
    }

    #[test]
    fn buffer_body_constants_are_defined() {
        assert_eq!(MAX_BUFFERED_BODY_BYTES, 64 * 1024);
        assert_eq!(MAX_LOGGED_BODY_BYTES, 2048);
    }

    #[test]
    fn log_error_responses_constants_are_defined() {
        assert_eq!(MAX_BUFFERED_BODY_BYTES, 64 * 1024);
        assert_eq!(MAX_LOGGED_BODY_BYTES, 2048);
    }

    #[tokio::test]
    async fn log_error_responses_passthrough_for_success_status() {
        let app = Router::new()
            .route("/ok", get(|| async { (StatusCode::OK, "healthy") }))
            .layer(from_fn(log_error_responses));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/ok")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("call app");

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 1024)
            .await
            .expect("read response body");
        assert_eq!(bytes, b"healthy"[..]);
    }

    #[tokio::test]
    async fn log_error_responses_keeps_error_body_for_client_and_server_errors() {
        let app = Router::new()
            .route(
                "/bad",
                get(|| async { (StatusCode::BAD_REQUEST, "bad request payload") }),
            )
            .route(
                "/err",
                get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "internal error payload") }),
            )
            .layer(from_fn(log_error_responses));

        let bad = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/bad")
                    .body(Body::empty())
                    .expect("build bad request"),
            )
            .await
            .expect("call bad route");
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
        let bad_body = to_bytes(bad.into_body(), 4096)
            .await
            .expect("read bad body");
        assert_eq!(bad_body, b"bad request payload"[..]);

        let err = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/err")
                    .body(Body::empty())
                    .expect("build err request"),
            )
            .await
            .expect("call err route");
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let err_body = to_bytes(err.into_body(), 4096)
            .await
            .expect("read err body");
        assert_eq!(err_body, b"internal error payload"[..]);
    }

    #[tokio::test]
    async fn log_error_responses_returns_empty_body_when_buffering_fails() {
        let app = Router::new()
            .route(
                "/huge-error",
                get(|| async {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "x".repeat(MAX_BUFFERED_BODY_BYTES + 10),
                    )
                }),
            )
            .layer(from_fn(log_error_responses));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/huge-error")
                    .body(Body::empty())
                    .expect("build huge request"),
            )
            .await
            .expect("call huge route");
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("read huge body");
        assert!(body.is_empty());
    }
}
