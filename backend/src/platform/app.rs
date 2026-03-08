use axum::{
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Extension, Router,
};
use std::{sync::Arc, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    attendance,
    config::Config,
    docs, handlers, identity,
    middleware::{self, rate_limit::user_rate_limit},
    requests,
    services::{
        audit_log::AuditLogServiceTrait, holiday::HolidayServiceTrait,
        holiday_exception::HolidayExceptionServiceTrait,
    },
    AppState,
};

pub struct AppServices {
    pub audit_log_service: Arc<dyn AuditLogServiceTrait>,
    pub holiday_service: Arc<dyn HolidayServiceTrait>,
    pub holiday_exception_service: Arc<dyn HolidayExceptionServiceTrait>,
}

pub fn build_app(state: AppState, services: AppServices) -> Router {
    let openapi = docs::ApiDoc::openapi();

    Router::new()
        .merge(identity::interface::http::public_routes(state.clone()))
        .merge(identity::interface::http::user_routes(state.clone()))
        .merge(attendance::interface::http::user_routes(state.clone()))
        .merge(attendance::interface::http::admin_routes(state.clone()))
        .merge(attendance::interface::http::system_admin_routes(
            state.clone(),
        ))
        .merge(requests::interface::http::user_routes(state.clone()))
        .merge(requests::interface::http::admin_routes(state.clone()))
        .merge(public_routes(state.clone()))
        .merge(user_routes(state.clone()))
        .merge(admin_routes(state.clone()))
        .merge(system_admin_routes(state.clone()))
        .merge(SwaggerUi::new("/api/docs").url("/api-doc/openapi.json", openapi))
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(build_http_request_span)
                        .on_response(
                            DefaultOnResponse::new()
                                .level(Level::INFO)
                                .latency_unit(tower_http::LatencyUnit::Millis),
                        ),
                )
                .layer(axum_middleware::from_fn(middleware::request_id))
                .layer(axum_middleware::from_fn(middleware::log_error_responses))
                .layer(cors_layer(&state.config)),
        )
        .layer(Extension(services.audit_log_service))
        .layer(Extension(services.holiday_service))
        .layer(Extension(services.holiday_exception_service))
        .with_state(state)
}

pub fn build_http_request_span(request: &axum::http::Request<axum::body::Body>) -> tracing::Span {
    let request_id = request
        .extensions()
        .get::<middleware::RequestId>()
        .map(|id| id.0.as_str())
        .unwrap_or("unknown");

    tracing::info_span!(
        "http_request",
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
        request_id = %request_id,
        user_id = tracing::field::Empty,
        username = tracing::field::Empty,
    )
}

pub fn public_routes(state: AppState) -> Router<AppState> {
    Router::new()
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

pub fn user_routes(state: AppState) -> Router<AppState> {
    Router::new()
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
            "/api/admin/users/{id}/sessions",
            get(handlers::admin::list_user_sessions),
        )
        .route(
            "/api/admin/sessions/{id}",
            delete(handlers::admin::revoke_session),
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

pub fn admin_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/admin/users", post(handlers::admin::create_user))
        .route(
            "/api/admin/users/{id}/reset-mfa",
            post(handlers::admin::reset_user_mfa),
        )
        .route("/api/admin/users/{id}", put(handlers::admin::update_user))
        .route(
            "/api/admin/users/{id}/unlock",
            post(handlers::admin::unlock_user_account),
        )
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

pub fn system_admin_routes(_state: AppState) -> Router<AppState> {
    Router::new()
}

pub fn cors_layer(config: &Config) -> CorsLayer {
    let mut layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([ACCEPT, AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true)
        .max_age(Duration::from_secs(24 * 60 * 60));

    if config.cors_allow_origins.iter().any(|origin| origin == "*") {
        layer = layer.allow_origin(AllowOrigin::predicate(|_, _| true));
    } else {
        let origins: Vec<HeaderValue> = config
            .cors_allow_origins
            .iter()
            .map(|origin| origin.parse().expect("Invalid CORS origin"))
            .collect();
        layer = layer.allow_origin(origins);
    }

    layer
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, extract::ConnectInfo, http::Request};
    use chrono_tz::UTC;
    use sqlx::postgres::PgPoolOptions;
    use std::{
        collections::HashMap,
        io,
        net::SocketAddr,
        sync::{Arc, Mutex},
    };
    use tower::Service;
    use tracing::{
        field::{Field, Visit},
        span::{Attributes, Id, Record},
    };
    use tracing_subscriber::{
        fmt::MakeWriter,
        layer::SubscriberExt,
        layer::{Context, Layer},
        registry::LookupSpan,
    };

    use crate::{
        config::Config,
        middleware,
        services::{
            audit_log::AuditLogService, holiday::HolidayService,
            holiday_exception::HolidayExceptionService,
        },
    };

    fn test_config(cors_allow_origins: Vec<String>) -> Config {
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
            cors_allow_origins,
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

    fn test_state_with_config(config: Config) -> AppState {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&config.database_url)
            .expect("create lazy pool");
        AppState::new(pool, None, None, None, config)
    }

    fn test_services(state: &AppState) -> AppServices {
        AppServices {
            audit_log_service: Arc::new(AuditLogService::new(state.write_pool.clone())),
            holiday_service: Arc::new(HolidayService::new(state.write_pool.clone())),
            holiday_exception_service: Arc::new(HolidayExceptionService::new(
                state.write_pool.clone(),
            )),
        }
    }

    #[tokio::test]
    async fn test_app_router_builds() {
        let config = test_config(vec!["*".to_string()]);
        let state = test_state_with_config(config);
        let services = test_services(&state);
        let mut app = build_app(state, services);

        let mut request = Request::builder()
            .uri("/api/config/timezone")
            .body(Body::empty())
            .expect("build request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.call(request).await.expect("call app");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_domain_route_groups_require_auth() {
        let state = test_state_with_config(test_config(vec!["*".to_string()]));

        let mut user_app = Router::new()
            .merge(user_routes(state.clone()))
            .with_state(state.clone());
        let request = Request::builder()
            .method("GET")
            .uri("/api/admin/users")
            .body(Body::empty())
            .expect("build user route request");
        let response = user_app.call(request).await.expect("call user route");
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);

        let mut admin_app = Router::new()
            .merge(admin_routes(state.clone()))
            .with_state(state.clone());
        let request = Request::builder()
            .method("GET")
            .uri("/api/admin/users")
            .body(Body::empty())
            .expect("build admin route request");
        let response = admin_app.call(request).await.expect("call admin route");
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);

        let mut system_admin_app = Router::new()
            .merge(system_admin_routes(state.clone()))
            .with_state(state);
        let request = Request::builder()
            .method("POST")
            .uri("/api/admin/users/test-id/reset-mfa")
            .body(Body::empty())
            .expect("build system admin route request");
        let response = system_admin_app
            .call(request)
            .await
            .expect("call system admin route");
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_cors_layer_accepts_specific_origins() {
        let config = test_config(vec![
            "https://example.com".to_string(),
            "http://localhost:3000".to_string(),
        ]);
        let _layer = cors_layer(&config);
    }

    #[derive(Default, Clone)]
    struct SpanStore {
        data: Arc<Mutex<HashMap<String, HashMap<String, String>>>>,
    }

    #[derive(Default, Clone)]
    struct CaptureWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    struct CaptureWriterGuard {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl<'a> MakeWriter<'a> for CaptureWriter {
        type Writer = CaptureWriterGuard;

        fn make_writer(&'a self) -> Self::Writer {
            CaptureWriterGuard {
                buffer: Arc::clone(&self.buffer),
            }
        }
    }

    impl io::Write for CaptureWriterGuard {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut buffer = self.buffer.lock().expect("lock log buffer");
            buffer.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FieldCapture {
        fields: HashMap<String, String>,
    }

    impl FieldCapture {
        fn record_value(&mut self, field: &Field, value: String) {
            self.fields.insert(field.name().to_string(), value);
        }
    }

    impl Visit for FieldCapture {
        fn record_i64(&mut self, field: &Field, value: i64) {
            self.record_value(field, value.to_string());
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
            self.record_value(field, value.to_string());
        }

        fn record_bool(&mut self, field: &Field, value: bool) {
            self.record_value(field, value.to_string());
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.record_value(field, value.to_string());
        }

        fn record_f64(&mut self, field: &Field, value: f64) {
            self.record_value(field, value.to_string());
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.record_value(field, format!("{value:?}"));
        }
    }

    struct SpanName(String);

    impl<S> Layer<S> for SpanStore
    where
        S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
            let mut visitor = FieldCapture::default();
            attrs.record(&mut visitor);
            let name = attrs.metadata().name().to_string();

            {
                let mut data = self.data.lock().expect("lock span data");
                data.insert(name.clone(), visitor.fields);
            }

            if let Some(span) = ctx.span(id) {
                span.extensions_mut().insert(SpanName(name));
            }
        }

        fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
            let mut visitor = FieldCapture::default();
            values.record(&mut visitor);
            if visitor.fields.is_empty() {
                return;
            }

            if let Some(span) = ctx.span(id) {
                if let Some(name) = span.extensions().get::<SpanName>() {
                    let mut data = self.data.lock().expect("lock span data");
                    let entry = data.entry(name.0.clone()).or_default();
                    entry.extend(visitor.fields);
                }
            }
        }
    }

    #[test]
    fn test_http_request_span_fields_are_recorded() {
        let store = SpanStore::default();
        let subscriber = tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_test_writer()
                    .with_ansi(false),
            )
            .with(store.clone());

        tracing::subscriber::with_default(subscriber, || {
            let mut request = Request::builder()
                .method("GET")
                .uri("/api/attendance/status")
                .version(axum::http::Version::HTTP_11)
                .body(Body::empty())
                .expect("build request");
            request
                .extensions_mut()
                .insert(middleware::RequestId("req-123".to_string()));

            let span = build_http_request_span(&request);
            span.record("user_id", "42");
            span.record("username", "alice");

            let _guard = span.enter();
            tracing::info!("span capture test");
        });

        let data = store.data.lock().expect("lock span data");
        let span_fields = data
            .get("http_request")
            .expect("http_request span recorded");

        assert_eq!(span_fields.get("method").map(String::as_str), Some("GET"));
        assert_eq!(
            span_fields.get("uri").map(String::as_str),
            Some("/api/attendance/status")
        );
        assert_eq!(
            span_fields.get("version").map(String::as_str),
            Some("HTTP/1.1")
        );
        assert_eq!(
            span_fields.get("request_id").map(String::as_str),
            Some("req-123")
        );
        assert_eq!(span_fields.get("user_id").map(String::as_str), Some("42"));
        assert_eq!(
            span_fields.get("username").map(String::as_str),
            Some("alice")
        );
    }

    #[test]
    fn test_http_request_span_does_not_log_sensitive_values() {
        let store = SpanStore::default();
        let writer = CaptureWriter::default();
        let subscriber = tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(writer.clone())
                    .with_ansi(false),
            )
            .with(store.clone());

        tracing::subscriber::with_default(subscriber, || {
            let mut request = Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Authorization", "Bearer super-secret-token")
                .body(Body::from(
                    r#"{"password":"P@ssw0rd","token":"secret-token"}"#,
                ))
                .expect("build request");
            request
                .extensions_mut()
                .insert(middleware::RequestId("req-123".to_string()));

            let span = build_http_request_span(&request);
            let _guard = span.enter();
            tracing::info!("login attempt");
        });

        let data = store.data.lock().expect("lock span data");
        let span_fields = data
            .get("http_request")
            .expect("http_request span recorded");
        let sensitive_values = ["super-secret-token", "P@ssw0rd", "secret-token"];

        for value in sensitive_values {
            let leaked_field = span_fields.values().any(|field| field.contains(value));
            assert!(
                !leaked_field,
                "span fields should not contain sensitive value: {value}"
            );
        }
    }
}
