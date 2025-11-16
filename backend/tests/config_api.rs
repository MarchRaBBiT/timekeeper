use axum::extract::State;
use timekeeper_backend::handlers::config;

mod support;
use support::{setup_test_pool, test_config};

#[tokio::test]
async fn timezone_endpoint_returns_configured_value() {
    let Some(pool) = setup_test_pool().await else {
        eprintln!("Skipping timezone_endpoint_returns_configured_value: database unavailable");
        return;
    };
    let cfg = test_config();

    let response = config::get_time_zone(State((pool, cfg.clone()))).await;
    assert_eq!(response.0.time_zone, cfg.time_zone.to_string());
}
