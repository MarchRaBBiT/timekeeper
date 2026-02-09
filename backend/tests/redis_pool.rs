use timekeeper_backend::db::redis::create_redis_pool;

mod support;

#[tokio::test]
async fn create_redis_pool_returns_none_when_disabled() {
    let config = support::test_config();
    let pool = create_redis_pool(&config).await.expect("create redis pool");
    assert!(pool.is_none());
}

#[tokio::test]
async fn create_redis_pool_fails_when_unreachable() {
    let mut config = support::test_config();
    config.redis_url = Some("redis://127.0.0.1:1".to_string());
    config.redis_pool_size = 1;
    config.redis_connect_timeout = 1;

    let pool = create_redis_pool(&config)
        .await
        .expect("pool builder should succeed");
    let pool = pool.expect("pool should be returned");
    let result = pool.get().await;
    assert!(result.is_err());
}
