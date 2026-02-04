use sqlx::PgPool;
use timekeeper_backend::db::connection::{
    create_pool_with_config, create_pools, create_pools_with_config, PoolConfig,
};

mod support;

async fn ensure_test_db() -> String {
    let _pool = support::test_pool().await;
    std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL set")
}

async fn ping(pool: &PgPool) {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .expect("ping database");
}

#[tokio::test]
async fn create_pool_with_config_connects() {
    let url = ensure_test_db().await;
    let config = PoolConfig::default();
    let pool = create_pool_with_config(&url, config)
        .await
        .expect("create pool");
    ping(&pool).await;
}

#[tokio::test]
async fn create_pools_returns_none_when_read_url_missing() {
    let url = ensure_test_db().await;
    let (write_pool, read_pool) = create_pools(&url, None)
        .await
        .expect("create pools");
    assert!(read_pool.is_none());
    ping(&write_pool).await;
}

#[tokio::test]
async fn create_pools_with_read_replica_connects() {
    let url = ensure_test_db().await;
    let config = PoolConfig::default();
    let (write_pool, read_pool) = create_pools_with_config(&url, Some(&url), config)
        .await
        .expect("create pools with read replica");
    let read_pool = read_pool.expect("read pool");
    ping(&write_pool).await;
    ping(&read_pool).await;
}
