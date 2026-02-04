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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_auth_rate_limiter_uses_config_values() {
        let database_url = "postgres://localhost/test".to_string();
        let jwt_secret = "test-secret-key".to_string();
        let config = crate::config::Config {
            database_url,
            read_database_url: None,
            jwt_secret,
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            max_concurrent_sessions: 3,
            audit_log_retention_days: 1825,
            audit_log_retention_forever: false,
            consent_log_retention_days: 1825,
            consent_log_retention_forever: false,
            aws_region: "us-east-1".into(),
            aws_kms_key_id: "key-id".into(),
            aws_audit_log_bucket: "bucket".into(),
            aws_cloudtrail_enabled: false,
            cookie_secure: true,
            cookie_same_site: crate::utils::cookies::SameSite::Lax,
            cors_allow_origins: vec!["http://localhost:8000".into()],
            time_zone: chrono_tz::UTC,
            mfa_issuer: "Timekeeper".into(),
            rate_limit_ip_max_requests: 10,
            rate_limit_ip_window_seconds: 60,
            rate_limit_user_max_requests: 20,
            rate_limit_user_window_seconds: 3600,
            redis_url: None,
            redis_pool_size: 5,
            redis_connect_timeout: 5,
            feature_redis_cache_enabled: true,
            feature_read_replica_enabled: true,
            password_min_length: 12,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_numbers: true,
            password_require_symbols: true,
            password_expiration_days: 30,
            password_history_count: 5,
            production_mode: false,
        };

        let _limiter = create_auth_rate_limiter(&config);
    }

    #[test]
    fn create_auth_rate_limiter_handles_zero_values() {
        let database_url = "postgres://localhost/test".to_string();
        let jwt_secret = "test-secret-key".to_string();
        let config = crate::config::Config {
            database_url,
            read_database_url: None,
            jwt_secret,
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            max_concurrent_sessions: 3,
            audit_log_retention_days: 1825,
            audit_log_retention_forever: false,
            consent_log_retention_days: 1825,
            consent_log_retention_forever: false,
            aws_region: "us-east-1".into(),
            aws_kms_key_id: "key-id".into(),
            aws_audit_log_bucket: "bucket".into(),
            aws_cloudtrail_enabled: false,
            cookie_secure: true,
            cookie_same_site: crate::utils::cookies::SameSite::Lax,
            cors_allow_origins: vec!["http://localhost:8000".into()],
            time_zone: chrono_tz::UTC,
            mfa_issuer: "Timekeeper".into(),
            rate_limit_ip_max_requests: 0,
            rate_limit_ip_window_seconds: 0,
            rate_limit_user_max_requests: 20,
            rate_limit_user_window_seconds: 3600,
            redis_url: None,
            redis_pool_size: 5,
            redis_connect_timeout: 5,
            feature_redis_cache_enabled: true,
            feature_read_replica_enabled: true,
            password_min_length: 12,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_numbers: true,
            password_require_symbols: true,
            password_expiration_days: 30,
            password_history_count: 5,
            production_mode: false,
        };

        let _limiter = create_auth_rate_limiter(&config);
    }

    #[test]
    fn rate_limit_error_handler_too_many_requests() {
        use std::time::Duration;

        let error = GovernorError::TooManyRequests {
            wait_time: Duration::from_secs(5).as_secs(),
            headers: None,
        };

        let response = rate_limit_error_handler(error);
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().get(CONTENT_TYPE).is_some());
    }

    #[test]
    fn rate_limit_error_handler_unable_to_extract_key() {
        let error = GovernorError::UnableToExtractKey;

        let response = rate_limit_error_handler(error);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.headers().get(CONTENT_TYPE).is_some());
    }

    #[test]
    fn rate_limit_error_handler_other_error() {
        let error = GovernorError::Other {
            code: StatusCode::BAD_REQUEST,
            msg: Some("custom error".to_string()),
            headers: None,
        };

        let response = rate_limit_error_handler(error);
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response.headers().get(CONTENT_TYPE).is_some());
    }

    #[test]
    fn rate_limit_error_handler_other_error_with_headers() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-custom", "value".parse().unwrap());

        let error = GovernorError::Other {
            code: StatusCode::BAD_REQUEST,
            msg: Some("error with headers".to_string()),
            headers: Some(headers),
        };

        let response = rate_limit_error_handler(error);
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response.headers().get("x-custom").is_some());
    }
}
