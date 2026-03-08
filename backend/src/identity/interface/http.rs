use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Router,
};

use crate::{
    handlers::{auth, config, sessions},
    middleware::{self, rate_limit::user_rate_limit},
    AppState,
};

pub fn public_routes(state: AppState) -> Router<AppState> {
    let rate_limiter = middleware::rate_limit::create_auth_rate_limiter(&state.config);

    Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/refresh", post(auth::refresh))
        .route(
            "/api/auth/request-password-reset",
            post(auth::request_password_reset),
        )
        .route("/api/auth/reset-password", post(auth::reset_password))
        .route("/api/config/timezone", get(config::get_time_zone))
        .route_layer(rate_limiter)
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::audit_log,
        ))
}

pub fn user_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/auth/mfa", get(auth::mfa_status))
        .route("/api/auth/mfa", delete(auth::mfa_disable))
        .route("/api/auth/mfa/register", post(auth::mfa_register))
        .route("/api/auth/mfa/setup", post(auth::mfa_setup))
        .route("/api/auth/mfa/activate", post(auth::mfa_activate))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/me", put(auth::update_profile))
        .route("/api/auth/sessions", get(sessions::list_sessions))
        .route("/api/auth/sessions/{id}", delete(sessions::revoke_session))
        .route("/api/auth/change-password", put(auth::change_password))
        .route("/api/auth/logout", post(auth::logout))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            user_rate_limit,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::audit_log,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use chrono_tz::UTC;
    use sqlx::postgres::PgPoolOptions;
    use tower::Service;

    use crate::config::Config;

    fn test_config() -> Config {
        Config {
            database_url: "postgres://test".to_string(),
            read_database_url: None,
            jwt_secret: "test-jwt-secret-32-chars-minimum!".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            max_concurrent_sessions: 3,
            audit_log_retention_days: 1825,
            audit_log_retention_forever: false,
            consent_log_retention_days: 1825,
            consent_log_retention_forever: false,
            aws_region: "ap-northeast-1".to_string(),
            aws_kms_key_id: "alias/timekeeper-test".to_string(),
            aws_audit_log_bucket: "timekeeper-audit-logs".to_string(),
            aws_cloudtrail_enabled: true,
            cookie_secure: false,
            cookie_same_site: crate::utils::cookies::SameSite::Lax,
            cors_allow_origins: vec!["*".to_string()],
            time_zone: UTC,
            mfa_issuer: "Timekeeper".to_string(),
            rate_limit_ip_max_requests: 15,
            rate_limit_ip_window_seconds: 900,
            rate_limit_user_max_requests: 20,
            rate_limit_user_window_seconds: 3600,
            redis_url: None,
            redis_pool_size: 10,
            redis_connect_timeout: 5,
            feature_redis_cache_enabled: true,
            feature_read_replica_enabled: true,
            password_min_length: 12,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_numbers: true,
            password_require_symbols: true,
            password_expiration_days: 90,
            password_history_count: 5,
            account_lockout_threshold: 5,
            account_lockout_duration_minutes: 15,
            account_lockout_backoff_enabled: true,
            account_lockout_max_duration_hours: 24,
            production_mode: false,
        }
    }

    fn test_state() -> AppState {
        let config = test_config();
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&config.database_url)
            .expect("create lazy pool");
        AppState::new(pool, None, None, None, config)
    }

    #[tokio::test]
    async fn user_identity_routes_require_auth() {
        let state = test_state();
        let mut app = Router::new()
            .merge(user_routes(state.clone()))
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/auth/me")
            .body(Body::empty())
            .expect("build user route request");
        let response = app.call(request).await.expect("call user route");

        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }
}
