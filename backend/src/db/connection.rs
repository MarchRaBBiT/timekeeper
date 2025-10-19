use sqlx::postgres::PgPool;
use std::sync::Arc;

pub type DbPool = Arc<PgPool>;

pub async fn create_pool(database_url: &str) -> anyhow::Result<DbPool> {
    let pool = PgPool::connect(database_url).await?;
    Ok(Arc::new(pool))
}
