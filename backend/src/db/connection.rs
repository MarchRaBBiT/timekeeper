use sqlx::postgres::PgPool;

/// Type alias so downstream code can reference the logical database pool in a single place.
pub type DbPool = PgPool;

pub async fn create_pool(database_url: &str) -> anyhow::Result<DbPool> {
    let pool = PgPool::connect(database_url).await?;
    Ok(pool)
}
