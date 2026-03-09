use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Method, Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, sync::Arc};
use timekeeper_backend::{
    platform::app::{build_app, AppServices},
    services::{
        audit_log::{AuditLogService, AuditLogServiceTrait},
        holiday::{HolidayService, HolidayServiceTrait},
        holiday_exception::{HolidayExceptionService, HolidayExceptionServiceTrait},
    },
    AppState,
};
use tower::ServiceExt;

mod support;

fn smoke_app() -> axum::Router {
    let config = support::test_config();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&config.database_url)
        .expect("create lazy pool");
    let state = AppState::new(pool.clone(), None, None, None, config);
    let services = AppServices {
        audit_log_service: Arc::new(AuditLogService::new(pool.clone()))
            as Arc<dyn AuditLogServiceTrait>,
        holiday_service: Arc::new(HolidayService::new(pool.clone()))
            as Arc<dyn HolidayServiceTrait>,
        holiday_exception_service: Arc::new(HolidayExceptionService::new(pool))
            as Arc<dyn HolidayExceptionServiceTrait>,
    };

    build_app(state, services)
}

fn smoke_request(method: Method, uri: &str) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .expect("build smoke request");
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
    request
}

#[tokio::test]
async fn public_platform_routes_are_wired() {
    let cases = [
        (Method::GET, "/api/config/timezone", StatusCode::OK),
        (Method::GET, "/api-doc/openapi.json", StatusCode::OK),
    ];

    for (method, uri, expected_status) in cases {
        let response = smoke_app()
            .oneshot(smoke_request(method.clone(), uri))
            .await
            .expect("call public smoke route");

        assert_eq!(response.status(), expected_status, "{uri} should be wired");
    }
}

#[tokio::test]
async fn protected_platform_routes_require_auth_instead_of_404() {
    let cases = [
        (Method::GET, "/api/auth/me"),
        (Method::GET, "/api/auth/sessions"),
        (Method::GET, "/api/attendance/me"),
        (Method::GET, "/api/requests/me"),
        (Method::GET, "/api/admin/users"),
        (Method::GET, "/api/admin/audit-logs"),
        (Method::POST, "/api/admin/users"),
        (
            Method::GET,
            "/api/admin/users/00000000-0000-0000-0000-000000000000/holiday-exceptions",
        ),
    ];

    for (method, uri) in cases {
        let response = smoke_app()
            .oneshot(smoke_request(method.clone(), uri))
            .await
            .expect("call protected smoke route");

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{uri} should be protected instead of missing"
        );
    }
}
