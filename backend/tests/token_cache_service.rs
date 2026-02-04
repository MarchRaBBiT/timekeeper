use std::net::TcpListener;

use testcontainers::{clients::Cli, core::WaitFor, GenericImage, RunnableImage};
use timekeeper_backend::db::redis::create_redis_pool;
use timekeeper_backend::services::token_cache::{TokenCacheService, TokenCacheServiceTrait};
use timekeeper_backend::types::UserId;

mod support;

fn allocate_ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("read socket addr")
        .port()
}

#[tokio::test]
async fn token_cache_service_roundtrip() {
    let docker = Cli::default();
    let host_port = allocate_ephemeral_port();
    let image = GenericImage::new("redis", "7-alpine")
        .with_wait_for(WaitFor::message_on_stdout(
            "Ready to accept connections",
        ));
    let image = RunnableImage::from(image).with_mapped_port((host_port, 6379));
    let _container = docker.run(image);

    let mut config = support::test_config();
    config.redis_url = Some(format!("redis://127.0.0.1:{host_port}"));
    config.redis_pool_size = 2;
    config.redis_connect_timeout = 5;

    let pool = create_redis_pool(&config)
        .await
        .expect("create redis pool")
        .expect("redis pool available");
    let service = TokenCacheService::new(pool);

    let user_id = UserId::new();
    service
        .cache_token("jti-1", user_id, 60)
        .await
        .expect("cache token");
    let active = service
        .is_token_active("jti-1")
        .await
        .expect("check token");
    assert_eq!(active, Some(true));

    service
        .invalidate_token("jti-1")
        .await
        .expect("invalidate token");
    let active = service
        .is_token_active("jti-1")
        .await
        .expect("check token");
    assert_eq!(active, Some(false));

    service
        .cache_token("jti-2", user_id, 60)
        .await
        .expect("cache token 2");
    service
        .cache_token("jti-3", user_id, 60)
        .await
        .expect("cache token 3");
    service
        .invalidate_user_tokens(user_id)
        .await
        .expect("invalidate user tokens");

    let active = service
        .is_token_active("jti-2")
        .await
        .expect("check token 2");
    assert_eq!(active, Some(false));
    let active = service
        .is_token_active("jti-3")
        .await
        .expect("check token 3");
    assert_eq!(active, Some(false));
}
