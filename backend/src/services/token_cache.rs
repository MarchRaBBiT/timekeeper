use crate::db::redis::RedisPool;
use crate::types::UserId;
use async_trait::async_trait;
use bb8_redis::redis::{self, AsyncCommands};

#[async_trait]
pub trait TokenCacheServiceTrait: Send + Sync {
    async fn cache_token(&self, jti: &str, user_id: UserId, ttl_seconds: u64)
        -> anyhow::Result<()>;
    async fn is_token_active(&self, jti: &str) -> anyhow::Result<Option<bool>>;
    async fn invalidate_token(&self, jti: &str) -> anyhow::Result<()>;
    async fn invalidate_user_tokens(&self, user_id: UserId) -> anyhow::Result<()>;
}

pub struct TokenCacheService {
    pool: RedisPool,
}

impl TokenCacheService {
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    fn token_key(jti: &str) -> String {
        format!("token:{}", jti)
    }

    fn user_tokens_key(user_id: UserId) -> String {
        format!("user_tokens:{}", user_id)
    }
}

#[async_trait]
impl TokenCacheServiceTrait for TokenCacheService {
    async fn cache_token(
        &self,
        jti: &str,
        user_id: UserId,
        ttl_seconds: u64,
    ) -> anyhow::Result<()> {
        let span = tracing::debug_span!("redis_cache_token", jti, %user_id);
        let _enter = span.enter();

        let mut conn = self.pool.get().await?;
        let key = Self::token_key(jti);
        let user_key = Self::user_tokens_key(user_id);

        redis::pipe()
            .atomic()
            .set_ex(&key, user_id.to_string(), ttl_seconds)
            .sadd(&user_key, jti)
            .expire(&user_key, ttl_seconds as i64)
            .query_async::<_, ()>(&mut *conn)
            .await?;

        Ok(())
    }

    async fn is_token_active(&self, jti: &str) -> anyhow::Result<Option<bool>> {
        let span = tracing::debug_span!("redis_is_token_active", jti);
        let _enter = span.enter();

        let mut conn = self.pool.get().await?;
        let key = Self::token_key(jti);
        let exists: bool = conn.exists(key).await?;
        Ok(Some(exists))
    }

    async fn invalidate_token(&self, jti: &str) -> anyhow::Result<()> {
        let span = tracing::debug_span!("redis_invalidate_token", jti);
        let _enter = span.enter();

        let mut conn = self.pool.get().await?;
        let key = Self::token_key(jti);

        // We don't easily know user_id here to remove from user_tokens set
        // but it will expire eventually. For a full logout/invalidation,
        // we usually have the JTI.
        conn.del::<_, ()>(key).await?;
        Ok(())
    }

    async fn invalidate_user_tokens(&self, user_id: UserId) -> anyhow::Result<()> {
        let span = tracing::debug_span!("redis_invalidate_user_tokens", %user_id);
        let _enter = span.enter();

        let mut conn = self.pool.get().await?;
        let user_key = Self::user_tokens_key(user_id);

        let jtis: Vec<String> = conn.smembers(&user_key).await?;
        if jtis.is_empty() {
            return Ok(());
        }

        let mut pipe = redis::pipe();
        pipe.atomic();
        for jti in jtis {
            pipe.del(Self::token_key(&jti));
        }
        pipe.del(user_key);

        pipe.query_async::<_, ()>(&mut *conn).await?;
        Ok(())
    }
}
