use timekeeper_backend::{
    config::Config, db::connection::create_pool, handlers::auth_repo,
    repositories::password_reset as password_reset_repo,
};

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

    let deleted_count = password_reset_repo::delete_expired_tokens(&pool)
        .await
        .expect("cleanup expired password reset tokens");

    if deleted_count > 0 {
        tracing::info!("Deleted {} expired password reset tokens", deleted_count);
    }

    sqlx::query("VACUUM (ANALYZE) password_resets")
        .execute(&pool)
        .await
        .expect("vacuum password_resets table");

    Ok(())
}
