use axum::{
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Extension, Router,
};
use chrono::Utc;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod config;
mod db;
mod docs;
mod handlers;
mod middleware;
mod models;
mod repositories;
mod services;
mod utils;

use config::{AuditLogRetentionPolicy, Config};
use db::connection::{create_pool, DbPool};
use services::{
    audit_log::AuditLogService, holiday::HolidayService, holiday_exception::HolidayExceptionService,
};

type AuthState = (DbPool, Config);

fn mask_secret(s: &str) -> String {
    if s.is_empty() {
        return "<empty>".into();
    }
    let prefix = s.chars().take(4).collect::<String>();
    format!("{}*** (len={})", prefix, s.len())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = Config::load()?;
    log_config(&config);

    let pool: DbPool = create_pool(&config.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let audit_log_service = Arc::new(AuditLogService::new(pool.clone()));
    let holiday_service = Arc::new(HolidayService::new(pool.clone()));
    let holiday_exception_service = Arc::new(HolidayExceptionService::new(pool.clone()));
    let shared_state: AuthState = (pool.clone(), config.clone());

    spawn_audit_log_cleanup(
        audit_log_service.clone(),
        config.audit_log_retention_policy(),
    );

    let openapi = docs::ApiDoc::openapi();

    let app = Router::new()
        .merge(public_routes(shared_state.clone()))
        .merge(user_routes(shared_state.clone()))
        .merge(admin_routes(shared_state.clone()))
        .merge(system_admin_routes(shared_state.clone()))
        .merge(SwaggerUi::new("/api/docs").url("/api-doc/openapi.json", openapi.clone()))
        .layer(
            ServiceBuilder::new()
                .layer(axum_middleware::from_fn(middleware::request_id))
                .layer(axum_middleware::from_fn(middleware::log_error_responses))
                .layer(TraceLayer::new_for_http())
                .layer(cors_layer(&config)),
        )
        .layer(Extension(audit_log_service))
        .layer(Extension(holiday_service))
        .layer(Extension(holiday_exception_service))
        .with_state(shared_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "timekeeper_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn log_config(config: &Config) {
    tracing::info!(
        database_url = %config.database_url,
        jwt_secret = %mask_secret(&config.jwt_secret),
        jwt_expiration_hours = config.jwt_expiration_hours,
        refresh_token_expiration_days = config.refresh_token_expiration_days,
        audit_log_retention_days = config.audit_log_retention_days,
        audit_log_retention_forever = config.audit_log_retention_forever,
        cookie_secure = config.cookie_secure,
        cookie_same_site = ?config.cookie_same_site,
        cors_allow_origins = ?config.cors_allow_origins,
        time_zone = %config.time_zone,
        mfa_issuer = %config.mfa_issuer,
        "Loaded configuration from environment/.env"
    );
}

fn public_routes(state: AuthState) -> Router<AuthState> {
    Router::new()
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/refresh", post(handlers::auth::refresh))
        .route("/api/config/timezone", get(handlers::config::get_time_zone))
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::audit_log,
        ))
}

fn user_routes(state: AuthState) -> Router<AuthState> {
    let audit_state = state.clone();
    Router::new()
        .route(
            "/api/attendance/clock-in",
            post(handlers::attendance::clock_in),
        )
        .route(
            "/api/attendance/clock-out",
            post(handlers::attendance::clock_out),
        )
        .route(
            "/api/attendance/break-start",
            post(handlers::attendance::break_start),
        )
        .route(
            "/api/attendance/break-end",
            post(handlers::attendance::break_end),
        )
        .route(
            "/api/attendance/status",
            get(handlers::attendance::get_attendance_status),
        )
        .route(
            "/api/attendance/me",
            get(handlers::attendance::get_my_attendance),
        )
        .route(
            "/api/attendance/me/summary",
            get(handlers::attendance::get_my_summary),
        )
        .route(
            "/api/attendance/:id/breaks",
            get(handlers::attendance::get_breaks_by_attendance),
        )
        .route(
            "/api/attendance/export",
            get(handlers::attendance::export_my_attendance),
        )
        .route(
            "/api/requests/leave",
            post(handlers::requests::create_leave_request),
        )
        .route(
            "/api/requests/overtime",
            post(handlers::requests::create_overtime_request),
        )
        .route("/api/requests/me", get(handlers::requests::get_my_requests))
        .route(
            "/api/requests/:id",
            put(handlers::requests::update_request).delete(handlers::requests::cancel_request),
        )
        .route("/api/auth/mfa", get(handlers::auth::mfa_status))
        .route("/api/auth/mfa", delete(handlers::auth::mfa_disable))
        .route("/api/auth/mfa/register", post(handlers::auth::mfa_register))
        .route("/api/auth/mfa/setup", post(handlers::auth::mfa_setup))
        .route("/api/auth/mfa/activate", post(handlers::auth::mfa_activate))
        .route("/api/auth/me", get(handlers::auth::me))
        .route(
            "/api/auth/change-password",
            put(handlers::auth::change_password),
        )
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route(
            "/api/holidays",
            get(handlers::holidays::list_public_holidays),
        )
        .route(
            "/api/holidays/check",
            get(handlers::holidays::check_holiday),
        )
        .route(
            "/api/holidays/month",
            get(handlers::holidays::list_month_holidays),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, middleware::auth))
        .route_layer(axum_middleware::from_fn_with_state(
            audit_state,
            middleware::audit_log,
        ))
}

fn admin_routes(state: AuthState) -> Router<AuthState> {
    let audit_state = state.clone();
    Router::new()
        // Request routes (from requests.rs module)
        .route("/api/admin/requests", get(handlers::admin::requests::list_requests))
        .route(
            "/api/admin/requests/:id",
            get(handlers::admin::requests::get_request_detail),
        )
        .route(
            "/api/admin/requests/:id/approve",
            put(handlers::admin::requests::approve_request),
        )
        .route(
            "/api/admin/requests/:id/reject",
            put(handlers::admin::requests::reject_request),
        )
        // Holiday routes (from holidays.rs module)
        .route(
            "/api/admin/holidays",
            get(handlers::admin::holidays::list_holidays).post(handlers::admin::holidays::create_holiday),
        )
        .route(
            "/api/admin/holidays/weekly",
            get(handlers::admin::holidays::list_weekly_holidays).post(handlers::admin::holidays::create_weekly_holiday),
        )
        .route(
            "/api/admin/holidays/weekly/:id",
            delete(handlers::admin::holidays::delete_weekly_holiday),
        )
        .route(
            "/api/admin/holidays/:id",
            delete(handlers::admin::holidays::delete_holiday),
        )
        // User routes (from users.rs module)
        .route("/api/admin/users", get(handlers::admin::users::get_users))
        // Attendance routes (from attendance.rs module)
        .route(
            "/api/admin/attendance",
            get(handlers::admin::attendance::get_all_attendance),
        )
        // Export routes (from export.rs module)
        .route("/api/admin/export", get(handlers::admin::export::export_data))
        // Google holidays (from holidays module)
        .route(
            "/api/admin/holidays/google",
            get(handlers::holidays::fetch_google_holidays),
        )
        // Holiday exceptions (from holiday_exceptions module)
        .route(
            "/api/admin/users/:user_id/holiday-exceptions",
            post(handlers::holiday_exceptions::create_holiday_exception)
                .get(handlers::holiday_exceptions::list_holiday_exceptions),
        )
        .route(
            "/api/admin/users/:user_id/holiday-exceptions/:id",
            delete(handlers::holiday_exceptions::delete_holiday_exception),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::auth_admin,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            audit_state,
            middleware::audit_log,
        ))
}

fn system_admin_routes(state: AuthState) -> Router<AuthState> {
    let audit_state = state.clone();
    Router::new()
        // Audit log routes (from audit_logs.rs module)
        .route(
            "/api/admin/audit-logs",
            get(handlers::admin::audit_logs::list_audit_logs),
        )
        .route(
            "/api/admin/audit-logs/export",
            get(handlers::admin::audit_logs::export_audit_logs),
        )
        .route(
            "/api/admin/audit-logs/:id",
            get(handlers::admin::audit_logs::get_audit_log_detail),
        )
        // User routes (from users.rs module)
        .route("/api/admin/users", post(handlers::admin::users::create_user))
        .route(
            "/api/admin/users/:id",
            delete(handlers::admin::users::delete_user),
        )
        .route(
            "/api/admin/archived-users",
            get(handlers::admin::users::get_archived_users),
        )
        .route(
            "/api/admin/archived-users/:id",
            delete(handlers::admin::users::delete_archived_user),
        )
        .route(
            "/api/admin/archived-users/:id/restore",
            post(handlers::admin::users::restore_archived_user),
        )
        // Attendance routes (from attendance.rs module)
        .route(
            "/api/admin/attendance",
            put(handlers::admin::attendance::upsert_attendance),
        )
        .route(
            "/api/admin/breaks/:id/force-end",
            put(handlers::admin::attendance::force_end_break),
        )
        // MFA routes (from users.rs module)
        .route(
            "/api/admin/mfa/reset",
            post(handlers::admin::users::reset_user_mfa),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::auth_system_admin,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            audit_state,
            middleware::audit_log,
        ))
}

fn cors_layer(config: &Config) -> CorsLayer {
    let origins = config
        .cors_allow_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();
    let allow_origin = if config.cors_allow_origins.iter().any(|o| o == "*") || origins.is_empty() {
        AllowOrigin::predicate(|_, _| true)
    } else {
        AllowOrigin::list(origins)
    };
    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT])
        .max_age(Duration::from_secs(24 * 60 * 60))
}

fn spawn_audit_log_cleanup(
    audit_log_service: Arc<AuditLogService>,
    retention_policy: AuditLogRetentionPolicy,
) {
    let Some(retention_days) = retention_policy.retention_days() else {
        tracing::info!(
            retention_policy = ?retention_policy,
            "Audit log cleanup disabled"
        );
        return;
    };

    tracing::info!(retention_days, "Starting daily audit log cleanup task");

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            let Some(cutoff) = retention_policy.cleanup_cutoff(Utc::now()) else {
                continue;
            };
            match audit_log_service.delete_logs_before(cutoff).await {
                Ok(deleted) => {
                    tracing::info!(deleted, "Audit log cleanup completed");
                }
                Err(err) => {
                    tracing::warn!(error = ?err, "Audit log cleanup failed");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::get};
    use chrono_tz::UTC;
    use tower::Service;

    fn test_config(cors_allow_origins: Vec<String>) -> Config {
        Config {
            database_url: "postgres://test".to_string(),
            jwt_secret: "test-jwt-secret-32-chars-minimum!".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
            audit_log_retention_days: 365,
            audit_log_retention_forever: false,
            cookie_secure: false,
            cookie_same_site: crate::utils::cookies::SameSite::Lax,
            cors_allow_origins,
            time_zone: UTC,
            mfa_issuer: "Timekeeper".to_string(),
        }
    }

    #[tokio::test]
    async fn cors_allows_request_origin_when_wildcard_configured() {
        let config = test_config(vec!["*".to_string()]);
        let app = Router::new()
            .route("/ping", get(|| async { "ok" }))
            .layer(cors_layer(&config));

        let origin = "http://example.com";
        let mut app = app;
        let response = app
            .call(
                Request::builder()
                    .method(Method::GET)
                    .uri("/ping")
                    .header("Origin", origin)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.headers().get("access-control-allow-origin"),
            Some(&HeaderValue::from_static(origin))
        );
        assert_eq!(
            response.headers().get("access-control-allow-credentials"),
            Some(&HeaderValue::from_static("true"))
        );
    }

    #[tokio::test]
    async fn cors_allows_request_origin_when_origin_list_empty() {
        let config = test_config(Vec::new());
        let app = Router::new()
            .route("/ping", get(|| async { "ok" }))
            .layer(cors_layer(&config));

        let origin = "http://localhost:8000";
        let mut app = app;
        let response = app
            .call(
                Request::builder()
                    .method(Method::GET)
                    .uri("/ping")
                    .header("Origin", origin)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.headers().get("access-control-allow-origin"),
            Some(&HeaderValue::from_static(origin))
        );
    }

    #[tokio::test]
    async fn cors_preflight_allows_configured_headers() {
        let config = test_config(vec!["http://localhost:8000".to_string()]);
        let app = Router::new()
            .route("/ping", get(|| async { "ok" }))
            .layer(cors_layer(&config));

        let origin = "http://localhost:8000";
        let mut app = app;
        let response = app
            .call(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/ping")
                    .header("Origin", origin)
                    .header("Access-Control-Request-Method", "POST")
                    .header(
                        "Access-Control-Request-Headers",
                        "content-type,authorization",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let allow_headers = response
            .headers()
            .get("access-control-allow-headers")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();

        assert!(allow_headers.contains("content-type"));
        assert!(allow_headers.contains("authorization"));
    }
}
