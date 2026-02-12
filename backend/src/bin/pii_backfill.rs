use sqlx::{FromRow, PgPool};
use timekeeper_backend::{
    config::Config,
    db::connection::create_pool,
    utils::{
        encryption::{decrypt_pii, encrypt_pii, hash_email},
        kms::active_key_version,
        mfa::{protect_totp_secret, recover_totp_secret},
    },
};

#[derive(Debug, FromRow)]
struct UserPiiRow {
    id: String,
    full_name_enc: Option<String>,
    email_enc: Option<String>,
    mfa_secret_enc: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let pool = create_pool(&config.database_url).await?;

    backfill_users(&pool, &config).await?;
    backfill_archived_users(&pool, &config).await?;

    println!("PII backfill completed");
    Ok(())
}

async fn backfill_users(pool: &PgPool, config: &Config) -> anyhow::Result<()> {
    let key_version = i16::try_from(active_key_version()).unwrap_or(1);
    let rows = sqlx::query_as::<_, UserPiiRow>(
        "SELECT id, full_name_enc, email_enc, mfa_secret_enc FROM users ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        let full_name_plain = row
            .full_name_enc
            .as_deref()
            .map(|value| decrypt_pii(value, config))
            .transpose()?
            .unwrap_or_default();
        let email_plain = row
            .email_enc
            .as_deref()
            .map(|value| decrypt_pii(value, config))
            .transpose()?
            .unwrap_or_default();
        let full_name_enc = encrypt_pii(&full_name_plain, config)?;
        let email_enc = encrypt_pii(&email_plain, config)?;
        let email_hash = hash_email(&email_plain, config);
        let mfa_secret_enc = row
            .mfa_secret_enc
            .as_deref()
            .map(|raw| {
                let plain = recover_totp_secret(raw, config)?;
                protect_totp_secret(&plain, config)
            })
            .transpose()?;

        sqlx::query(
            "UPDATE users SET \
             full_name_enc = $1, \
             email_enc = $2, \
             email_hash = $3, \
             mfa_secret_enc = $4, \
             pii_key_version = $5 \
             WHERE id = $6",
        )
        .bind(full_name_enc)
        .bind(email_enc)
        .bind(email_hash)
        .bind(mfa_secret_enc)
        .bind(key_version)
        .bind(row.id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn backfill_archived_users(pool: &PgPool, config: &Config) -> anyhow::Result<()> {
    let key_version = i16::try_from(active_key_version()).unwrap_or(1);
    let rows = sqlx::query_as::<_, UserPiiRow>(
        "SELECT id, full_name_enc, email_enc, mfa_secret_enc FROM archived_users ORDER BY archived_at ASC",
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        let full_name_plain = row
            .full_name_enc
            .as_deref()
            .map(|value| decrypt_pii(value, config))
            .transpose()?
            .unwrap_or_default();
        let email_plain = row
            .email_enc
            .as_deref()
            .map(|value| decrypt_pii(value, config))
            .transpose()?
            .unwrap_or_default();
        let full_name_enc = encrypt_pii(&full_name_plain, config)?;
        let email_enc = encrypt_pii(&email_plain, config)?;
        let email_hash = hash_email(&email_plain, config);
        let mfa_secret_enc = row
            .mfa_secret_enc
            .as_deref()
            .map(|raw| {
                let plain = recover_totp_secret(raw, config)?;
                protect_totp_secret(&plain, config)
            })
            .transpose()?;

        sqlx::query(
            "UPDATE archived_users SET \
             full_name_enc = $1, \
             email_enc = $2, \
             email_hash = $3, \
             mfa_secret_enc = $4, \
             pii_key_version = $5 \
             WHERE id = $6",
        )
        .bind(full_name_enc)
        .bind(email_enc)
        .bind(email_hash)
        .bind(mfa_secret_enc)
        .bind(key_version)
        .bind(row.id)
        .execute(pool)
        .await?;
    }

    Ok(())
}
