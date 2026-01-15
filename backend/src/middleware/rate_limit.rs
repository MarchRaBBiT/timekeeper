use axum::body::Body;
use axum::http::{header::CONTENT_TYPE, HeaderValue, Response, StatusCode};
use governor::middleware::StateInformationMiddleware;
use std::sync::Arc;
use std::time::Duration;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor, GovernorError,
    GovernorLayer,
};

use crate::config::Config;

pub fn create_auth_rate_limiter(
    config: &Config,
) -> GovernorLayer<PeerIpKeyExtractor, StateInformationMiddleware, Body> {
    let burst_size = config.rate_limit_ip_max_requests.max(1);
    let window_seconds = config.rate_limit_ip_window_seconds.max(1);
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(window_seconds))
            .burst_size(burst_size)
            .key_extractor(PeerIpKeyExtractor)
            .use_headers()
            .finish()
            .expect("rate limiter config should be valid"),
    );

    GovernorLayer::new(governor_conf).error_handler(rate_limit_error_handler)
}

fn rate_limit_error_handler(error: GovernorError) -> Response<Body> {
    match error {
        GovernorError::TooManyRequests { wait_time, headers } => {
            tracing::warn!(wait_time, "Rate limit exceeded");
            let body = serde_json::json!({
                "error": "rate_limit_exceeded",
                "message": "Too many requests. Please try again later.",
                "retry_after": wait_time,
            })
            .to_string();
            let mut response = Response::new(Body::from(body));
            *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            if let Some(headers) = headers {
                response.headers_mut().extend(headers);
            }
            response
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            response
        }
        GovernorError::UnableToExtractKey => {
            let body = serde_json::json!({
                "error": "rate_limit_key_error",
                "message": "Unable to determine request identity.",
            })
            .to_string();
            let mut response = Response::new(Body::from(body));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            response
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            response
        }
        GovernorError::Other { code, msg, headers } => {
            let body = serde_json::json!({
                "error": "rate_limit_error",
                "message": msg.unwrap_or_else(|| "Rate limit error".to_string()),
            })
            .to_string();
            let mut response = Response::new(Body::from(body));
            *response.status_mut() = code;
            if let Some(headers) = headers {
                response.headers_mut().extend(headers);
            }
            response
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            response
        }
    }
}
