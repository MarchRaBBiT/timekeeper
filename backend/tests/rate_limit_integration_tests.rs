use axum::{http::StatusCode, routing::post, Router};
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpListener;

use timekeeper_backend::{config::Config, middleware::rate_limit::create_auth_rate_limiter};

fn test_config(rate_limit_ip_max_requests: u32, rate_limit_ip_window_seconds: u64) -> Config {
    Config {
        database_url: "test://".to_string(),
        read_database_url: None,
        jwt_secret: "test-jwt-secret-32-chars-minimum!".to_string(),
        jwt_expiration_hours: 1,
        refresh_token_expiration_days: 7,
        max_concurrent_sessions: 3,
        audit_log_retention_days: 30,
        audit_log_retention_forever: false,
        consent_log_retention_days: 30,
        consent_log_retention_forever: false,
        aws_region: "us-east-1".to_string(),
        aws_kms_key_id: "test-key".to_string(),
        aws_audit_log_bucket: "test-bucket".to_string(),
        aws_cloudtrail_enabled: false,
        cookie_secure: false,
        cookie_same_site: timekeeper_backend::utils::cookies::SameSite::Lax,
        cors_allow_origins: vec!["http://localhost:3000".to_string()],
        time_zone: chrono_tz::UTC,
        mfa_issuer: "Timekeeper".to_string(),
        rate_limit_ip_max_requests,
        rate_limit_ip_window_seconds,
        rate_limit_user_max_requests: 5,
        rate_limit_user_window_seconds: 300,
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
        production_mode: false,
    }
}

async fn spawn_rate_limited_app(config: Config) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let limiter = create_auth_rate_limiter(&config);

    let app = Router::new()
        .route("/login", post(|| async { StatusCode::OK }))
        .route_layer(limiter);

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );
    let handle = tokio::spawn(async move {
        server.await.expect("server should run");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (addr, handle)
}

#[tokio::test]
async fn rate_limit_blocks_after_burst() {
    let config = test_config(2, 2);
    let (addr, handle) = spawn_rate_limited_app(config).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/login", addr);

    for _ in 0..2 {
        let resp = client.post(&url).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    let resp = client.post(&url).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    handle.abort();
}

#[tokio::test]
async fn rate_limit_includes_headers() {
    let config = test_config(1, 2);
    let (addr, handle) = spawn_rate_limited_app(config).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/login", addr);

    let _ = client.post(&url).send().await.unwrap();
    let resp = client.post(&url).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
    assert!(resp.headers().contains_key("x-ratelimit-remaining"));
    assert!(resp.headers().contains_key("x-ratelimit-after"));

    handle.abort();
}

#[tokio::test]
async fn rate_limit_blocks_same_peer_ip() {
    let config = test_config(1, 2);
    let (addr, handle) = spawn_rate_limited_app(config).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/login", addr);

    let resp = client.post(&url).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = client.post(&url).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    handle.abort();
}
