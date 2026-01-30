use timekeeper_backend::{
    config::Config,
    db::connection::create_pool,
    repositories::{active_session, auth as auth_repo, password_reset as password_reset_repo},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let pool = create_pool(&config.database_url).await?;

    auth_repo::cleanup_expired_access_tokens(&pool)
        .await
        .expect("cleanup active tokens");

    let deleted_sessions = active_session::cleanup_expired_sessions(&pool)
        .await
        .expect("cleanup expired sessions");
    if deleted_sessions > 0 {
        tracing::info!("Deleted {} expired sessions", deleted_sessions);
    }

    sqlx::query("VACUUM (ANALYZE) active_access_tokens")
        .execute(&pool)
        .await
        .expect("vacuum tokens table");

    sqlx::query("VACUUM (ANALYZE) active_sessions")
        .execute(&pool)
        .await
        .expect("vacuum active_sessions table");

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
