use axum::{
    http::Method,
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod handlers;
mod middleware;
use crate::middleware as auth_middleware;
mod models;
mod utils;

use config::Config;
use db::connection::{create_pool, DbPool};

fn mask_secret(s: &str) -> String {
    if s.is_empty() {
        return "<empty>".into();
    }
    let prefix = s.chars().take(4).collect::<String>();
    format!("{}*** (len={})", prefix, s.len())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "timekeeper_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::load()?;
    tracing::info!(
        database_url = %config.database_url,
        jwt_secret = %mask_secret(&config.jwt_secret),
        jwt_expiration_hours = config.jwt_expiration_hours,
        refresh_token_expiration_days = config.refresh_token_expiration_days,
        time_zone = %config.time_zone,
        mfa_issuer = %config.mfa_issuer,
        "Loaded configuration from environment/.env"
    );

    // Initialize database
    let pool: DbPool = create_pool(&config.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Build public routes (no auth)
    let public_routes = Router::new()
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/refresh", post(handlers::auth::refresh));

    // Build user-protected routes (auth required)
    let user_routes = Router::new()
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
        .route(
            "/api/auth/change-password",
            put(handlers::auth::change_password),
        )
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route(
            "/api/holidays",
            get(handlers::holidays::list_public_holidays),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            (pool.clone(), config.clone()),
            auth_middleware::auth,
        ));

    // Build admin-protected routes (auth + admin role)
    let admin_routes = Router::new()
        .route(
            "/api/admin/users",
            get(handlers::admin::get_users).post(handlers::admin::create_user),
        )
        .route(
            "/api/admin/attendance",
            get(handlers::admin::get_all_attendance).put(handlers::admin::upsert_attendance),
        )
        .route(
            "/api/admin/breaks/:id/force-end",
            put(handlers::admin::force_end_break),
        )
        .route("/api/admin/requests", get(handlers::admin::list_requests))
        .route(
            "/api/admin/requests/:id",
            get(handlers::admin::get_request_detail),
        )
        .route(
            "/api/admin/requests/:id/approve",
            put(handlers::admin::approve_request),
        )
        .route(
            "/api/admin/requests/:id/reject",
            put(handlers::admin::reject_request),
        )
        .route(
            "/api/admin/mfa/reset",
            post(handlers::admin::reset_user_mfa),
        )
        .route(
            "/api/admin/holidays",
            get(handlers::admin::list_holidays).post(handlers::admin::create_holiday),
        )
        .route(
            "/api/admin/holidays/:id",
            delete(handlers::admin::delete_holiday),
        )
        .route(
            "/api/admin/holidays/google",
            get(handlers::holidays::fetch_google_holidays),
        )
        .route("/api/admin/export", get(handlers::admin::export_data))
        .route_layer(axum_middleware::from_fn_with_state(
            (pool.clone(), config.clone()),
            auth_middleware::auth_admin,
        ));

    // Compose app with shared layers (CORS/Trace) and shared state
    let app = Router::new()
        .merge(public_routes)
        .merge(user_routes)
        .merge(admin_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([
                            Method::GET,
                            Method::POST,
                            Method::PUT,
                            Method::DELETE,
                            Method::OPTIONS,
                        ])
                        .allow_headers(Any)
                        .max_age(std::time::Duration::from_secs(24 * 60 * 60)),
                ),
        )
        .with_state((pool, config));

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
