use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Router,
};

use crate::{
    handlers::admin,
    middleware::{self, rate_limit::user_rate_limit},
    AppState,
};

pub fn admin_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/admin/audit-logs", get(admin::list_audit_logs))
        .route(
            "/api/admin/audit-logs/export",
            get(admin::export_audit_logs),
        )
        .route(
            "/api/admin/audit-logs/{id}",
            get(admin::get_audit_log_detail),
        )
        .route("/api/admin/users", get(admin::get_users))
        .route(
            "/api/admin/users/{id}/sessions",
            get(admin::list_user_sessions),
        )
        .route("/api/admin/sessions/{id}", delete(admin::revoke_session))
        .route("/api/admin/export", get(admin::export_data))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            user_rate_limit,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_admin,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::audit_log,
        ))
}

pub fn system_admin_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/admin/users", post(admin::create_user))
        .route(
            "/api/admin/users/{id}/reset-mfa",
            post(admin::reset_user_mfa),
        )
        .route("/api/admin/users/{id}", put(admin::update_user))
        .route(
            "/api/admin/users/{id}/unlock",
            post(admin::unlock_user_account),
        )
        .route("/api/admin/users/{id}", delete(admin::delete_user))
        .route("/api/admin/archived-users", get(admin::get_archived_users))
        .route(
            "/api/admin/archived-users/{id}",
            delete(admin::delete_archived_user),
        )
        .route(
            "/api/admin/archived-users/{id}/restore",
            post(admin::restore_archived_user),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            user_rate_limit,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_system_admin,
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
    async fn admin_console_routes_require_admin_auth() {
        let state = test_state();
        let mut app = Router::new()
            .merge(admin_routes(state.clone()))
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/admin/users")
            .body(Body::empty())
            .expect("build admin route request");
        let response = app.call(request).await.expect("call admin route");

        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_system_routes_require_system_admin_auth() {
        let state = test_state();
        let mut app = Router::new()
            .merge(system_admin_routes(state.clone()))
            .with_state(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/admin/users")
            .body(Body::empty())
            .expect("build system admin route request");
        let response = app.call(request).await.expect("call system admin route");

        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }
}
