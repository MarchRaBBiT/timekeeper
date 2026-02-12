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

#[derive(Debug, Clone, Copy)]
struct RotateOptions {
    dry_run: bool,
}

#[derive(Debug, FromRow)]
struct UserPiiRow {
    id: String,
    full_name_enc: Option<String>,
    email_enc: Option<String>,
    mfa_secret_enc: Option<String>,
    pii_key_version: i16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = parse_options(std::env::args().skip(1));
    let config = Config::load()?;
    let pool = create_pool(&config.database_url).await?;

    let target_version = active_key_version();
    if target_version == 0 {
        anyhow::bail!("KMS_ACTIVE_KEY_VERSION must be >= 1");
    }

    let users_updated = rotate_users(&pool, &config, target_version, options).await?;
    let archived_updated = rotate_archived_users(&pool, &config, target_version, options).await?;

    if options.dry_run {
        println!(
            "[dry-run] rotation target key_version={} users={} archived_users={}",
            target_version, users_updated, archived_updated
        );
    } else {
        println!(
            "rotation completed: key_version={} users={} archived_users={}",
            target_version, users_updated, archived_updated
        );
    }

    Ok(())
}

fn parse_options<I>(args: I) -> RotateOptions
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let dry_run = args.into_iter().any(|arg| arg.as_ref() == "--dry-run");
    RotateOptions { dry_run }
}

async fn rotate_users(
    pool: &PgPool,
    config: &Config,
    target_version: u16,
    options: RotateOptions,
) -> anyhow::Result<u64> {
    let rows = sqlx::query_as::<_, UserPiiRow>(
        "SELECT id, full_name_enc, email_enc, mfa_secret_enc, pii_key_version \
         FROM users ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    rotate_rows(
        pool,
        config,
        target_version,
        options,
        rows,
        "users",
        "updated_at = NOW(),",
    )
    .await
}

async fn rotate_archived_users(
    pool: &PgPool,
    config: &Config,
    target_version: u16,
    options: RotateOptions,
) -> anyhow::Result<u64> {
    let rows = sqlx::query_as::<_, UserPiiRow>(
        "SELECT id, full_name_enc, email_enc, mfa_secret_enc, pii_key_version \
         FROM archived_users ORDER BY archived_at ASC",
    )
    .fetch_all(pool)
    .await?;

    rotate_rows(
        pool,
        config,
        target_version,
        options,
        rows,
        "archived_users",
        "",
    )
    .await
}

async fn rotate_rows(
    pool: &PgPool,
    config: &Config,
    target_version: u16,
    options: RotateOptions,
    rows: Vec<UserPiiRow>,
    table_name: &str,
    update_prefix: &str,
) -> anyhow::Result<u64> {
    let mut updated = 0u64;
    let target_version_db = i16::try_from(target_version).unwrap_or(1);

    for row in rows {
        if row.pii_key_version == target_version_db {
            continue;
        }

        let full_name_source = row.full_name_enc.as_deref().unwrap_or_default();
        let email_source = row.email_enc.as_deref().unwrap_or_default();

        let full_name_plain = decrypt_pii(full_name_source, config)?;
        let email_plain = decrypt_pii(email_source, config)?;

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

        if options.dry_run {
            updated += 1;
            continue;
        }

        let sql = format!(
            "UPDATE {table_name} SET \
             {update_prefix} \
             full_name_enc = $1, \
             email_enc = $2, \
             email_hash = $3, \
             mfa_secret_enc = $4, \
             pii_key_version = $5 \
             WHERE id = $6"
        );

        sqlx::query(&sql)
            .bind(full_name_enc)
            .bind(email_enc)
            .bind(email_hash)
            .bind(mfa_secret_enc)
            .bind(target_version_db)
            .bind(row.id)
            .execute(pool)
            .await?;

        updated += 1;
    }

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_options_detects_dry_run() {
        let options = parse_options(["--dry-run"]);
        assert!(options.dry_run);

        let options = parse_options(["--unknown"]);
        assert!(!options.dry_run);
    }
}
