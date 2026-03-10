use anyhow::anyhow;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use sqlx::PgPool;

use crate::{
    config::Config,
    db::redis::RedisPool,
    repositories::auth as auth_repo,
    services::lockout_notification_queue::{
        dequeue_lockout_notification_job, push_lockout_notification_dead_letter,
        requeue_due_lockout_notification_jobs, schedule_lockout_notification_retry,
        LockoutNotificationDeadLetter, LockoutNotificationJob,
    },
    utils::{email::EmailService, encryption::decrypt_pii},
};

#[allow(dead_code)]
const LOCKOUT_NOTIFICATION_PROCESSING_KEY_PREFIX: &str = "auth:lockout-notifications:processing:";
#[allow(dead_code)]
const LOCKOUT_NOTIFICATION_SENT_KEY_PREFIX: &str = "auth:lockout-notifications:sent:";
#[allow(dead_code)]
const LOCKOUT_NOTIFICATION_PROCESSING_TTL_SECONDS: u64 = 300;
#[allow(dead_code)]
const LOCKOUT_NOTIFICATION_SENT_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;
// Five attempts gives 2 + 4 + 8 + 16 seconds of backoff before DLQ.
#[allow(dead_code)]
pub const LOCKOUT_NOTIFICATION_MAX_RETRIES: u32 = 5;
#[allow(dead_code)]
const LOCKOUT_NOTIFICATION_REQUEUE_LIMIT: usize = 32;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerOutcome {
    Sent,
    Retried,
    DeadLettered,
    SkippedDuplicate,
    SkippedMissingUser,
}

#[allow(dead_code)]
pub async fn requeue_due_jobs(pool: &RedisPool, now: DateTime<Utc>) -> anyhow::Result<usize> {
    requeue_due_lockout_notification_jobs(pool, now, LOCKOUT_NOTIFICATION_REQUEUE_LIMIT).await
}

#[allow(dead_code)]
pub async fn work_once(
    pool: &PgPool,
    redis_pool: &RedisPool,
    config: &Config,
) -> anyhow::Result<Option<WorkerOutcome>> {
    requeue_due_jobs(redis_pool, Utc::now()).await?;
    let Some(job) = dequeue_lockout_notification_job(redis_pool, 1).await? else {
        return Ok(None);
    };
    process_lockout_notification_job(pool, redis_pool, config, job)
        .await
        .map(Some)
}

#[allow(dead_code)]
pub async fn process_lockout_notification_job(
    pool: &PgPool,
    redis_pool: &RedisPool,
    config: &Config,
    job: LockoutNotificationJob,
) -> anyhow::Result<WorkerOutcome> {
    let idempotency_key = lockout_notification_idempotency_key(&job);
    if sent_marker_exists(redis_pool, &idempotency_key).await? {
        return Ok(WorkerOutcome::SkippedDuplicate);
    }

    let Some(_processing_guard) = acquire_processing_marker(
        redis_pool,
        &idempotency_key,
        LOCKOUT_NOTIFICATION_PROCESSING_TTL_SECONDS,
    )
    .await?
    else {
        return Ok(WorkerOutcome::SkippedDuplicate);
    };

    let Some(mut user) = auth_repo::find_user_by_id(pool, job.user_id)
        .await
        .map_err(|err| anyhow!("load user for lockout notification: {err}"))?
    else {
        return Ok(WorkerOutcome::SkippedMissingUser);
    };
    user.email =
        decrypt_pii(&user.email, config).map_err(|err| anyhow!("decrypt lockout email: {err}"))?;

    let email_service = EmailService::new().map_err(|err| anyhow!("email service error: {err}"))?;
    match email_service.send_account_lockout_notification(
        &user.email,
        &user.username,
        job.locked_until,
    ) {
        Ok(()) => {
            set_sent_marker(
                redis_pool,
                &idempotency_key,
                LOCKOUT_NOTIFICATION_SENT_TTL_SECONDS,
            )
            .await?;
            Ok(WorkerOutcome::Sent)
        }
        Err(err) => {
            if job.attempt + 1 >= LOCKOUT_NOTIFICATION_MAX_RETRIES {
                push_lockout_notification_dead_letter(
                    redis_pool,
                    &LockoutNotificationDeadLetter {
                        job,
                        failed_at: Utc::now(),
                        error: err.to_string(),
                    },
                )
                .await?;
                Ok(WorkerOutcome::DeadLettered)
            } else {
                let retry_job = job.retrying();
                let retry_at = Utc::now()
                    + ChronoDuration::seconds(next_retry_delay_seconds(retry_job.attempt));
                schedule_lockout_notification_retry(redis_pool, &retry_job, retry_at).await?;
                Ok(WorkerOutcome::Retried)
            }
        }
    }
}

#[allow(dead_code)]
pub fn next_retry_delay_seconds(attempt: u32) -> i64 {
    let exponent = attempt.min(6);
    2_i64.saturating_pow(exponent).max(1)
}

fn lockout_notification_idempotency_key(job: &LockoutNotificationJob) -> String {
    format!(
        "{}{}:{}",
        LOCKOUT_NOTIFICATION_SENT_KEY_PREFIX,
        job.user_id,
        job.locked_until.timestamp_millis()
    )
}

#[allow(dead_code)]
fn lockout_notification_processing_key(idempotency_key: &str) -> String {
    let suffix = idempotency_key
        .strip_prefix(LOCKOUT_NOTIFICATION_SENT_KEY_PREFIX)
        .unwrap_or(idempotency_key);
    format!("{LOCKOUT_NOTIFICATION_PROCESSING_KEY_PREFIX}{suffix}")
}

#[allow(dead_code)]
async fn sent_marker_exists(redis_pool: &RedisPool, idempotency_key: &str) -> anyhow::Result<bool> {
    let mut conn = redis_pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let exists: i32 = bb8_redis::redis::cmd("EXISTS")
        .arg(idempotency_key)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("read sent marker: {err}"))?;
    Ok(exists != 0)
}

#[allow(dead_code)]
async fn set_sent_marker(
    redis_pool: &RedisPool,
    idempotency_key: &str,
    ttl_seconds: u64,
) -> anyhow::Result<()> {
    let mut conn = redis_pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let response: Option<String> = bb8_redis::redis::cmd("SET")
        .arg(idempotency_key)
        .arg("1")
        .arg("EX")
        .arg(ttl_seconds)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("set sent marker: {err}"))?;
    if response.as_deref() != Some("OK") {
        return Err(anyhow!("set sent marker returned unexpected response"));
    }
    Ok(())
}

#[allow(dead_code)]
async fn acquire_processing_marker(
    redis_pool: &RedisPool,
    idempotency_key: &str,
    ttl_seconds: u64,
) -> anyhow::Result<Option<ProcessingMarkerGuard>> {
    let key = lockout_notification_processing_key(idempotency_key);
    let mut conn = redis_pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let response: Option<String> = bb8_redis::redis::cmd("SET")
        .arg(&key)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(ttl_seconds)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("set processing marker: {err}"))?;
    if response.as_deref() == Some("OK") {
        Ok(Some(ProcessingMarkerGuard {
            redis_pool: redis_pool.clone(),
            key,
        }))
    } else {
        Ok(None)
    }
}

#[allow(dead_code)]
struct ProcessingMarkerGuard {
    redis_pool: RedisPool,
    key: String,
}

impl Drop for ProcessingMarkerGuard {
    fn drop(&mut self) {
        let redis_pool = self.redis_pool.clone();
        let key = self.key.clone();
        tokio::spawn(async move {
            if let Ok(mut conn) = redis_pool.get().await {
                let _: Result<i32, _> = bb8_redis::redis::cmd("DEL")
                    .arg(key)
                    .query_async(&mut *conn)
                    .await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_retry_delay_seconds_grows_exponentially_and_caps() {
        assert_eq!(next_retry_delay_seconds(1), 2);
        assert_eq!(next_retry_delay_seconds(2), 4);
        assert_eq!(next_retry_delay_seconds(3), 8);
        assert_eq!(next_retry_delay_seconds(8), 64);
    }
}
