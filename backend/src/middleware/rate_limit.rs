use axum::body::Body;
use axum::http::{header::CONTENT_TYPE, HeaderValue, Response, StatusCode};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response as AxumResponse,
};
use governor::middleware::StateInformationMiddleware;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor, GovernorError,
    GovernorLayer,
};

use crate::config::Config;
use crate::models::user::User;
use crate::state::AppState;
use crate::utils::jwt::Claims;

#[derive(Debug, Clone)]
struct UserRateLimitWindow {
    requests: VecDeque<Instant>,
}

const USER_RATE_LIMIT_PERIODIC_CLEANUP_INTERVAL: Duration = Duration::from_secs(300);

fn user_rate_limit_store() -> &'static Mutex<HashMap<String, UserRateLimitWindow>> {
    static USER_RATE_LIMIT_STORE: OnceLock<Mutex<HashMap<String, UserRateLimitWindow>>> =
        OnceLock::new();
    USER_RATE_LIMIT_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn user_rate_limit_cleanup_threshold() -> usize {
    static THRESHOLD: OnceLock<usize> = OnceLock::new();
    cached_cleanup_threshold(
        &THRESHOLD,
        std::env::var("RATE_LIMIT_USER_STORE_CLEANUP_THRESHOLD").ok(),
    )
}

fn cached_cleanup_threshold(cache: &OnceLock<usize>, raw: Option<String>) -> usize {
    *cache.get_or_init(|| parse_cleanup_threshold(raw))
}

fn user_rate_limit_last_cleanup() -> &'static Mutex<Instant> {
    static LAST_CLEANUP: OnceLock<Mutex<Instant>> = OnceLock::new();
    // Start at "now" so periodic cleanup does not fire immediately during startup.
    LAST_CLEANUP.get_or_init(|| Mutex::new(Instant::now()))
}

fn parse_cleanup_threshold(raw: Option<String>) -> usize {
    const DEFAULT_THRESHOLD: usize = 10_000;
    raw.and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_THRESHOLD)
}

fn should_cleanup_user_rate_limit_store(
    store_len: usize,
    threshold: usize,
    now: Instant,
    last_cleanup_at: Instant,
    interval: Duration,
) -> bool {
    store_len > threshold || now.duration_since(last_cleanup_at) >= interval
}

pub async fn user_rate_limit(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> AxumResponse {
    let key = request
        .extensions()
        .get::<User>()
        .map(|user| user.id.to_string())
        .or_else(|| {
            request
                .extensions()
                .get::<Claims>()
                .map(|claims| claims.sub.clone())
        });

    let Some(key) = key else {
        return json_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "rate_limit_key_error",
            "Unable to determine request identity.",
            None,
        );
    };

    let max_requests = state.config.rate_limit_user_max_requests.max(1);
    let window = Duration::from_secs(state.config.rate_limit_user_window_seconds.max(1));
    let now = Instant::now();
    let threshold = user_rate_limit_cleanup_threshold();
    let last_cleanup_at = *user_rate_limit_last_cleanup()
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let (retry_after, did_cleanup) = {
        let mut store = user_rate_limit_store()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut did_cleanup = false;
        if should_cleanup_user_rate_limit_store(
            store.len(),
            threshold,
            now,
            last_cleanup_at,
            USER_RATE_LIMIT_PERIODIC_CLEANUP_INTERVAL,
        ) {
            store.retain(|_, entry| {
                prune_expired_requests(entry, now, window);
                !entry.requests.is_empty()
            });
            did_cleanup = true;
        }

        (
            evaluate_user_rate_limit(&mut store, key, max_requests, window, now).err(),
            did_cleanup,
        )
    };

    if did_cleanup {
        let mut last_cleanup = user_rate_limit_last_cleanup()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *last_cleanup = now;
    }

    if let Some(retry_after) = retry_after {
        return json_error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limit_exceeded",
            "Too many requests. Please try again later.",
            Some(retry_after),
        );
    }

    next.run(request).await
}

fn evaluate_user_rate_limit(
    store: &mut HashMap<String, UserRateLimitWindow>,
    key: String,
    max_requests: u32,
    window: Duration,
    now: Instant,
) -> Result<(), u64> {
    let entry = store.entry(key).or_insert(UserRateLimitWindow {
        requests: VecDeque::new(),
    });
    prune_expired_requests(entry, now, window);

    if entry.requests.len() >= max_requests as usize {
        let retry_after = entry
            .requests
            .front()
            .map(|oldest| {
                window
                    .saturating_sub(now.duration_since(*oldest))
                    .as_secs()
                    .max(1)
            })
            .unwrap_or(1);
        return Err(retry_after);
    }

    entry.requests.push_back(now);
    Ok(())
}

fn prune_expired_requests(entry: &mut UserRateLimitWindow, now: Instant, window: Duration) {
    while let Some(oldest) = entry.requests.front() {
        if now.duration_since(*oldest) >= window {
            entry.requests.pop_front();
        } else {
            break;
        }
    }
}

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
            let mut response = json_error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit_exceeded",
                "Too many requests. Please try again later.",
                Some(wait_time),
            );
            if let Some(headers) = headers {
                response.headers_mut().extend(headers);
            }
            response
        }
        GovernorError::UnableToExtractKey => json_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "rate_limit_key_error",
            "Unable to determine request identity.",
            None,
        ),
        GovernorError::Other { code, msg, headers } => {
            let mut response = json_error_response(
                code,
                "rate_limit_error",
                &msg.unwrap_or_else(|| "Rate limit error".to_string()),
                None,
            );
            if let Some(headers) = headers {
                response.headers_mut().extend(headers);
            }
            response
        }
    }
}

fn json_error_response(
    status: StatusCode,
    error: &str,
    message: &str,
    retry_after: Option<u64>,
) -> Response<Body> {
    let mut body = serde_json::json!({
        "error": error,
        "message": message,
    });
    if let Some(retry_after) = retry_after {
        body["retry_after"] = retry_after.into();
    }

    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if let Some(retry_after) = retry_after {
        if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
            response.headers_mut().insert("retry-after", value);
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{middleware, routing::get, Router};
    use http_body_util::BodyExt;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    #[test]
    fn create_auth_rate_limiter_uses_config_values() {
        let config = test_config(10, 60, 20, 3600);
        let _limiter = create_auth_rate_limiter(&config);
    }

    #[test]
    fn create_auth_rate_limiter_handles_zero_values() {
        let config = test_config(0, 0, 20, 3600);
        let _limiter = create_auth_rate_limiter(&config);
    }

    #[test]
    fn parse_cleanup_threshold_uses_default_for_invalid_values() {
        assert_eq!(parse_cleanup_threshold(None), 10_000);
        assert_eq!(parse_cleanup_threshold(Some("".to_string())), 10_000);
        assert_eq!(parse_cleanup_threshold(Some("abc".to_string())), 10_000);
        assert_eq!(parse_cleanup_threshold(Some("0".to_string())), 10_000);
    }

    #[test]
    fn parse_cleanup_threshold_accepts_positive_values() {
        assert_eq!(parse_cleanup_threshold(Some("500".to_string())), 500);
    }

    #[test]
    fn cached_cleanup_threshold_reads_once() {
        let cache = OnceLock::new();
        assert_eq!(
            cached_cleanup_threshold(&cache, Some("500".to_string())),
            500
        );
        assert_eq!(
            cached_cleanup_threshold(&cache, Some("1000".to_string())),
            500
        );
    }

    #[test]
    fn should_cleanup_periodically_even_below_threshold() {
        let now = Instant::now();
        let threshold = 10_000;
        let last_cleanup = now - USER_RATE_LIMIT_PERIODIC_CLEANUP_INTERVAL - Duration::from_secs(1);

        assert!(should_cleanup_user_rate_limit_store(
            1,
            threshold,
            now,
            last_cleanup,
            USER_RATE_LIMIT_PERIODIC_CLEANUP_INTERVAL,
        ));
    }

    #[test]
    fn should_not_cleanup_when_below_threshold_and_interval_not_elapsed() {
        let now = Instant::now();

        assert!(!should_cleanup_user_rate_limit_store(
            5,
            10_000,
            now,
            now - Duration::from_secs(60),
            USER_RATE_LIMIT_PERIODIC_CLEANUP_INTERVAL,
        ));
    }

    #[test]
    fn user_rate_limit_rejects_burst_at_window_boundary() {
        clear_user_rate_limit_store();
        let mut store = HashMap::new();
        let key = "boundary-user".to_string();
        let max_requests = 5u32;
        let window = Duration::from_secs(60);
        let base = Instant::now();

        assert!(
            evaluate_user_rate_limit(&mut store, key.clone(), max_requests, window, base).is_ok()
        );

        for _ in 0..(max_requests - 1) {
            assert!(evaluate_user_rate_limit(
                &mut store,
                key.clone(),
                max_requests,
                window,
                base + Duration::from_millis(59_900)
            )
            .is_ok());
        }

        assert!(evaluate_user_rate_limit(
            &mut store,
            key.clone(),
            max_requests,
            window,
            base + Duration::from_millis(60_100)
        )
        .is_ok());

        let rejected = evaluate_user_rate_limit(
            &mut store,
            key,
            max_requests,
            window,
            base + Duration::from_millis(60_100),
        );
        assert!(rejected.is_err());
    }

    #[test]
    fn rate_limit_error_handler_too_many_requests() {
        let error = GovernorError::TooManyRequests {
            wait_time: Duration::from_secs(5).as_secs(),
            headers: None,
        };

        let response = rate_limit_error_handler(error);
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().get(CONTENT_TYPE).is_some());
        assert!(response.headers().get("retry-after").is_some());
    }

    #[test]
    fn rate_limit_error_handler_unable_to_extract_key() {
        let error = GovernorError::UnableToExtractKey;

        let response = rate_limit_error_handler(error);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
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

    #[tokio::test]
    async fn user_rate_limit_rejects_excess_requests_for_same_user() {
        clear_user_rate_limit_store();
        let state = test_state(1, 60);
        let app = Router::new()
            .route("/limited", get(|| async { "ok" }))
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                user_rate_limit,
            ))
            .route_layer(middleware::from_fn(inject_claims))
            .with_state(state);

        let response_1 = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/limited")
                    .body(Body::empty())
                    .expect("build request 1"),
            )
            .await
            .expect("call request 1");
        assert_eq!(response_1.status(), StatusCode::OK);

        let response_2 = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/limited")
                    .body(Body::empty())
                    .expect("build request 2"),
            )
            .await
            .expect("call request 2");
        assert_eq!(response_2.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response_2.headers().get("retry-after").is_some());

        let body = response_2
            .into_body()
            .collect()
            .await
            .expect("read body")
            .to_bytes();
        let body_json: serde_json::Value =
            serde_json::from_slice(&body).expect("parse rate limit body");
        assert_eq!(body_json["error"], "rate_limit_exceeded");
    }

    async fn inject_claims(mut request: Request, next: Next) -> Response<Body> {
        request.extensions_mut().insert(Claims {
            sub: "test-user-1".to_string(),
            username: "tester".to_string(),
            role: "employee".to_string(),
            exp: chrono::Utc::now().timestamp() + 3600,
            iat: chrono::Utc::now().timestamp(),
            jti: "test-jti".to_string(),
        });
        next.run(request).await
    }

    fn clear_user_rate_limit_store() {
        let mut store = user_rate_limit_store()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        store.clear();
        let mut last_cleanup = user_rate_limit_last_cleanup()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *last_cleanup = Instant::now();
    }

    fn test_state(user_max_requests: u32, user_window_seconds: u64) -> AppState {
        let config = test_config(10, 60, user_max_requests, user_window_seconds);
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&config.database_url)
            .expect("create lazy pool");
        AppState::new(pool, None, None, None, config)
    }

    fn test_config(
        ip_max_requests: u32,
        ip_window_seconds: u64,
        user_max_requests: u32,
        user_window_seconds: u64,
    ) -> crate::config::Config {
        crate::config::Config {
            database_url: "postgres://localhost/test".to_string(),
            read_database_url: None,
            jwt_secret: "test-secret-key".to_string(),
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
            rate_limit_ip_max_requests: ip_max_requests,
            rate_limit_ip_window_seconds: ip_window_seconds,
            rate_limit_user_max_requests: user_max_requests,
            rate_limit_user_window_seconds: user_window_seconds,
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
            account_lockout_threshold: 5,
            account_lockout_duration_minutes: 15,
            account_lockout_backoff_enabled: true,
            account_lockout_max_duration_hours: 24,
            production_mode: false,
        }
    }
}
