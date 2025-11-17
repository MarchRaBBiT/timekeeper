use timekeeper_backend::{config::Config, db::connection::create_pool, handlers::auth_repo};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let pool = create_pool(&config.database_url).await?;

    auth_repo::cleanup_expired_access_tokens(&pool)
        .await
        .expect("cleanup active tokens");

    sqlx::query("VACUUM (ANALYZE) active_access_tokens")
        .execute(&pool)
        .await
        .expect("vacuum tokens table");

    Ok(())
}
