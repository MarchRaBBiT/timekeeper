use axum::{
    body::Body, http::StatusCode, middleware, middleware::Next, response::Response, routing::get,
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::{
    env, fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};
use testcontainers::{clients::Cli, core::WaitFor, GenericImage, RunnableImage};
use timekeeper_backend::{
    config::Config,
    db::redis::create_redis_pool,
    middleware::rate_limit::user_rate_limit,
    state::AppState,
    utils::{cookies::SameSite, jwt::Claims},
};
use tower::ServiceExt;

static DOCKER_WRAPPER_DIR: OnceLock<PathBuf> = OnceLock::new();

fn allocate_ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("read socket addr")
        .port()
}

fn ensure_docker_cli() {
    if env::var("DOCKER_HOST").is_err() {
        let podman_socket = Path::new("/run/podman/podman.sock");
        if podman_socket.exists() {
            env::set_var("DOCKER_HOST", "unix:///run/podman/podman.sock");
        } else if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
            let path = Path::new(&runtime_dir).join("podman/podman.sock");
            if path.exists() {
                if let Some(path_str) = path.to_str() {
                    env::set_var("DOCKER_HOST", format!("unix://{}", path_str));
                }
            }
        }
    }

    if Command::new("docker").arg("--version").output().is_ok() {
        return;
    }
    if Command::new("podman").arg("--version").output().is_err() {
        return;
    }

    let dir = DOCKER_WRAPPER_DIR.get_or_init(|| {
        let dir = env::temp_dir().join("timekeeper-testcontainers-docker");
        let _ = fs::create_dir_all(&dir);
        dir
    });
    let docker_path = dir.join("docker");
    if !docker_path.exists() {
        let script = "#!/usr/bin/env sh\nexec podman \"$@\"\n";
        let _ = fs::write(&docker_path, script);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&docker_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&docker_path, perms);
            }
        }
    }

    let path = env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.display(), path);
    env::set_var("PATH", new_path);
}

fn test_config(redis_url: Option<String>) -> Config {
    Config {
        database_url: "postgres://localhost/test".to_string(),
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
        cookie_same_site: SameSite::Lax,
        cors_allow_origins: vec!["http://localhost:8000".to_string()],
        time_zone: chrono_tz::UTC,
        mfa_issuer: "Timekeeper".to_string(),
        rate_limit_ip_max_requests: 10,
        rate_limit_ip_window_seconds: 60,
        rate_limit_user_max_requests: 1,
        rate_limit_user_window_seconds: 60,
        redis_url,
        redis_pool_size: 2,
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

fn build_test_app(state: AppState) -> Router {
    Router::new()
        .route("/limited", get(|| async { "ok" }))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            user_rate_limit,
        ))
        .route_layer(middleware::from_fn(inject_claims))
        .with_state(state)
}

async fn inject_claims(mut request: axum::extract::Request, next: Next) -> Response {
    request.extensions_mut().insert(Claims {
        sub: "redis-rate-limit-user".to_string(),
        username: "tester".to_string(),
        role: "employee".to_string(),
        exp: chrono::Utc::now().timestamp() + 3600,
        iat: chrono::Utc::now().timestamp(),
        jti: "test-jti".to_string(),
    });
    next.run(request).await
}

#[tokio::test]
async fn distributed_user_rate_limit_is_shared_across_app_instances_with_redis() {
    ensure_docker_cli();
    let docker = Cli::default();
    let host_port = allocate_ephemeral_port();
    let image = GenericImage::new("redis", "7-alpine")
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));
    let image = RunnableImage::from(image).with_mapped_port((host_port, 6379));
    let _container = docker.run(image);

    let redis_url = format!("redis://127.0.0.1:{host_port}");
    let config = test_config(Some(redis_url));
    let redis_pool = create_redis_pool(&config)
        .await
        .expect("create redis pool")
        .expect("redis pool available");

    let db_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&config.database_url)
        .expect("create lazy pool");

    let state_a = AppState::new(
        db_pool.clone(),
        None,
        Some(redis_pool.clone()),
        None,
        config.clone(),
    );
    let state_b = AppState::new(db_pool, None, Some(redis_pool), None, config);

    let response_a = build_test_app(state_a)
        .oneshot(
            axum::http::Request::builder()
                .uri("/limited")
                .body(Body::empty())
                .expect("build request for instance A"),
        )
        .await
        .expect("call instance A");
    assert_eq!(response_a.status(), StatusCode::OK);

    let response_b = build_test_app(state_b)
        .oneshot(
            axum::http::Request::builder()
                .uri("/limited")
                .body(Body::empty())
                .expect("build request for instance B"),
        )
        .await
        .expect("call instance B");
    assert_eq!(response_b.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response_b.headers().get("retry-after").is_some());
}
