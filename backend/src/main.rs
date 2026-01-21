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
pub mod error;
mod handlers;
mod middleware;
mod models;
mod repositories;
mod services;
mod state;
mod types;
mod utils;
mod validation;

use config::{AuditLogRetentionPolicy, Config};
use db::connection::create_pools;
use middleware::rate_limit::create_auth_rate_limiter;
use services::{
    audit_log::{AuditLogService, AuditLogServiceTrait},
    consent_log::ConsentLogService,
    holiday::{HolidayService, HolidayServiceTrait},
    holiday_exception::{HolidayExceptionService, HolidayExceptionServiceTrait},
};

pub use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = Config::load()?;
    log_config(&config);

    let (write_pool, read_pool) =
        create_pools(&config.database_url, config.read_database_url.as_deref()).await?;

    sqlx::migrate!("./migrations").run(&write_pool).await?;

    let audit_log_service: Arc<dyn AuditLogServiceTrait> =
        Arc::new(AuditLogService::new(write_pool.clone()));
    let consent_log_service = Arc::new(ConsentLogService::new(write_pool.clone()));
    let holiday_service: Arc<dyn HolidayServiceTrait> =
        Arc::new(HolidayService::new(write_pool.clone()));
    let holiday_exception_service: Arc<dyn HolidayExceptionServiceTrait> =
        Arc::new(HolidayExceptionService::new(write_pool.clone()));

    let shared_state = AppState::new(write_pool, read_pool, config.clone());

    spawn_audit_log_cleanup(
        audit_log_service.clone(),
        config.audit_log_retention_policy(),
    );
    spawn_consent_log_cleanup(
        consent_log_service.clone(),
        config.consent_log_retention_policy(),
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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

fn public_routes(state: AppState) -> Router<AppState> {
    let rate_limiter = create_auth_rate_limiter(&state.config);
    Router::new()
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/refresh", post(handlers::auth::refresh))
        .route(
            "/api/auth/request-password-reset",
            post(handlers::auth::request_password_reset),
        )
        .route(
            "/api/auth/reset-password",
            post(handlers::auth::reset_password),
        )
        .route("/api/config/timezone", get(handlers::config::get_time_zone))
        .route_layer(rate_limiter)
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::audit_log,
        ))
}

fn user_routes(state: AppState) -> Router<AppState> {
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
            "/api/attendance/{id}/breaks",
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
            "/api/requests/{id}",
            put(handlers::requests::update_request),
        )
        .route(
            "/api/requests/{id}",
            delete(handlers::requests::cancel_request),
        )
        .route("/api/consents", post(handlers::consents::record_consent))
        .route(
            "/api/consents/me",
            get(handlers::consents::list_my_consents),
        )
        .route(
            "/api/subject-requests",
            post(handlers::subject_requests::create_subject_request),
        )
        .route(
            "/api/subject-requests/me",
            get(handlers::subject_requests::list_my_subject_requests),
        )
        .route(
            "/api/subject-requests/{id}",
            delete(handlers::subject_requests::cancel_subject_request),
        )
        .route("/api/auth/mfa", get(handlers::auth::mfa_status))
        .route("/api/auth/mfa", delete(handlers::auth::mfa_disable))
        .route("/api/auth/mfa/register", post(handlers::auth::mfa_register))
        .route("/api/auth/mfa/setup", post(handlers::auth::mfa_setup))
        .route("/api/auth/mfa/activate", post(handlers::auth::mfa_activate))
        .route("/api/auth/me", get(handlers::auth::me))
        .route("/api/auth/me", put(handlers::auth::update_profile))
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
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::audit_log,
        ))
}

fn admin_routes(state: AppState) -> Router<AppState> {
    let audit_state = state.clone();
    Router::new()
        .route("/api/admin/requests", get(handlers::admin::list_requests))
        .route(
            "/api/admin/requests/{id}",
            get(handlers::admin::get_request_detail),
        )
        .route(
            "/api/admin/requests/{id}/approve",
            put(handlers::admin::approve_request),
        )
        .route(
            "/api/admin/requests/{id}/reject",
            put(handlers::admin::reject_request),
        )
        .route(
            "/api/admin/subject-requests",
            get(handlers::admin::list_subject_requests),
        )
        .route(
            "/api/admin/subject-requests/{id}/approve",
            put(handlers::admin::approve_subject_request),
        )
        .route(
            "/api/admin/subject-requests/{id}/reject",
            put(handlers::admin::reject_subject_request),
        )
        .route(
            "/api/admin/audit-logs",
            get(handlers::admin::list_audit_logs),
        )
        .route(
            "/api/admin/audit-logs/export",
            get(handlers::admin::export_audit_logs),
        )
        .route(
            "/api/admin/audit-logs/{id}",
            get(handlers::admin::get_audit_log_detail),
        )
        .route(
            "/api/admin/holidays",
            get(handlers::admin::list_holidays).post(handlers::admin::create_holiday),
        )
        .route(
            "/api/admin/holidays/weekly",
            get(handlers::admin::list_weekly_holidays).post(handlers::admin::create_weekly_holiday),
        )
        .route(
            "/api/admin/holidays/weekly/{id}",
            delete(handlers::admin::delete_weekly_holiday),
        )
        .route("/api/admin/users", get(handlers::admin::get_users))
        .route(
            "/api/admin/attendance",
            get(handlers::admin::get_all_attendance),
        )
        .route(
            "/api/admin/holidays/{id}",
            delete(handlers::admin::delete_holiday),
        )
        .route(
            "/api/admin/holidays/google",
            get(handlers::holidays::fetch_google_holidays),
        )
        .route(
            "/api/admin/users/{user_id}/holiday-exceptions",
            post(handlers::holiday_exceptions::create_holiday_exception)
                .get(handlers::holiday_exceptions::list_holiday_exceptions),
        )
        .route(
            "/api/admin/users/{user_id}/holiday-exceptions/{id}",
            delete(handlers::holiday_exceptions::delete_holiday_exception),
        )
        .route("/api/admin/export", get(handlers::admin::export_data))
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::auth_admin,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            audit_state,
            middleware::audit_log,
        ))
}

fn system_admin_routes(state: AppState) -> Router<AppState> {
    let audit_state = state.clone();
    Router::new()
        .route("/api/admin/users", post(handlers::admin::create_user))
        .route(
            "/api/admin/attendance",
            put(handlers::admin::upsert_attendance),
        )
        .route(
            "/api/admin/breaks/{id}/force-end",
            put(handlers::admin::force_end_break),
        )
        .route(
            "/api/admin/mfa/reset",
            post(handlers::admin::reset_user_mfa),
        )
        .route("/api/admin/users/{id}", put(handlers::admin::update_user))
        .route(
            "/api/admin/users/{id}",
            delete(handlers::admin::delete_user),
        )
        .route(
            "/api/admin/archived-users",
            get(handlers::admin::get_archived_users),
        )
        .route(
            "/api/admin/archived-users/{id}",
            delete(handlers::admin::delete_archived_user),
        )
        .route(
            "/api/admin/archived-users/{id}/restore",
            post(handlers::admin::restore_archived_user),
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
    tracing::info!("Database URL: {}", config.database_url);
    tracing::info!("Read Database URL: {:?}", config.read_database_url);
    tracing::info!("JWT Expiration: {} hours", config.jwt_expiration_hours);
    tracing::info!("Time Zone: {}", config.time_zone);
    tracing::info!("CORS Allowed Origins: {:?}", config.cors_allow_origins);
}

fn cors_layer(config: &Config) -> CorsLayer {
    let mut layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([ACCEPT, AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true)
        .max_age(Duration::from_secs(24 * 60 * 60));

    if config.cors_allow_origins.contains(&"*".to_string()) {
        layer = layer.allow_origin(AllowOrigin::predicate(|_, _| true));
    } else {
        let origins: Vec<HeaderValue> = config
            .cors_allow_origins
            .iter()
            .map(|s| s.parse().expect("Invalid CORS origin"))
            .collect();
        layer = layer.allow_origin(origins);
    }

    layer
}

fn spawn_audit_log_cleanup(
    audit_log_service: Arc<dyn AuditLogServiceTrait>,
    retention_policy: AuditLogRetentionPolicy,
) {
    if !retention_policy.is_recording_enabled() {
        return;
    }

    tracing::info!(
        retention_days = retention_policy.retention_days(),
        "Starting daily audit log cleanup task"
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
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

fn spawn_consent_log_cleanup(
    consent_log_service: Arc<ConsentLogService>,
    retention_policy: AuditLogRetentionPolicy,
) {
    if !retention_policy.is_recording_enabled() {
        return;
    }

    tracing::info!(
        retention_days = retention_policy.retention_days(),
        "Starting daily consent log cleanup task"
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            let Some(cutoff) = retention_policy.cleanup_cutoff(Utc::now()) else {
                continue;
            };
            match consent_log_service.delete_logs_before(cutoff).await {
                Ok(deleted) => {
                    tracing::info!(deleted, "Consent log cleanup completed");
                }
                Err(err) => {
                    tracing::warn!(error = ?err, "Consent log cleanup failed");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use chrono_tz::UTC;
    use tower::Service;

    fn test_config(cors_allow_origins: Vec<String>) -> Config {
        Config {
            database_url: "postgres://test".to_string(),
            read_database_url: None,
            jwt_secret: "test-jwt-secret-32-chars-minimum!".to_string(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 7,
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
            cors_allow_origins,
            time_zone: UTC,
            mfa_issuer: "Timekeeper".to_string(),
            rate_limit_ip_max_requests: 15,
            rate_limit_ip_window_seconds: 900,
            rate_limit_user_max_requests: 20,
            rate_limit_user_window_seconds: 3600,
            feature_read_replica_enabled: true,
        }
    }

    #[tokio::test]
    async fn test_app_router_builds() {
        let config = test_config(vec!["*".to_string()]);
        let (pool, _) = create_pools("sqlite::memory:", None).await.unwrap();
        let state = AppState::new(pool, None, config);

        let mut app = Router::new()
            .merge(public_routes(state.clone()))
            .with_state(state);

        let response = app
            .call(
                Request::builder()
                    .uri("/api/config/timezone")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
