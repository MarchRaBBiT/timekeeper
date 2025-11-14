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
