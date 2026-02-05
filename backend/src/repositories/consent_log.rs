use sqlx::PgPool;

use crate::models::consent_log::ConsentLog;

pub async fn insert_consent_log(pool: &PgPool, log: &ConsentLog) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO consent_logs \
         (id, user_id, purpose, policy_version, consented_at, ip, user_agent, request_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&log.id)
    .bind(&log.user_id)
    .bind(&log.purpose)
    .bind(&log.policy_version)
    .bind(log.consented_at)
    .bind(&log.ip)
    .bind(&log.user_agent)
    .bind(&log.request_id)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn list_consent_logs_for_user(
    pool: &PgPool,
    user_id: &str,
) -> Result<Vec<ConsentLog>, sqlx::Error> {
    sqlx::query_as::<_, ConsentLog>(
        "SELECT id, user_id, purpose, policy_version, consented_at, ip, user_agent, request_id, \
         created_at FROM consent_logs WHERE user_id = $1 ORDER BY consented_at DESC, id DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_consent_logs_before(
    pool: &PgPool,
    cutoff: chrono::DateTime<chrono::Utc>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM consent_logs WHERE consented_at < $1")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consent_log_functions_exist() {
        let _insert_consent_log = insert_consent_log;
        let _list_consent_logs_for_user = list_consent_logs_for_user;
        let _delete_consent_logs_before = delete_consent_logs_before;
    }
}
