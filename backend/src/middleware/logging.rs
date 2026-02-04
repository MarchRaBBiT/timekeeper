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
}
