use axum::extract::State;
use sqlx::PgPool;
use timekeeper_backend::handlers::config;

mod support;
use support::test_config;

#[sqlx::test(migrations = "./migrations")]
async fn timezone_endpoint_returns_configured_value(pool: PgPool) {
    let cfg = test_config();

    let response = config::get_time_zone(State((pool, cfg.clone()))).await;
    assert_eq!(response.0.time_zone, cfg.time_zone.to_string());
}
