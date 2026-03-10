use anyhow::anyhow;
use bb8_redis::redis;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{db::redis::RedisPool, types::UserId};

pub const LOCKOUT_NOTIFICATION_QUEUE_KEY: &str = "auth:lockout-notifications";
#[allow(dead_code)]
pub const LOCKOUT_NOTIFICATION_RETRY_KEY: &str = "auth:lockout-notifications:retry";
#[allow(dead_code)]
pub const LOCKOUT_NOTIFICATION_DLQ_KEY: &str = "auth:lockout-notifications:dlq";
const REQUEUE_DUE_LOCKOUT_NOTIFICATION_JOBS_SCRIPT: &str = r#"
local jobs = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1], 'LIMIT', 0, ARGV[2])
for _, job in ipairs(jobs) do
  redis.call('ZREM', KEYS[1], job)
  redis.call('RPUSH', KEYS[2], job)
end
return #jobs
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutNotificationJob {
    pub job_id: String,
    pub user_id: UserId,
    pub locked_until: DateTime<Utc>,
    pub enqueued_at: DateTime<Utc>,
    pub attempt: u32,
}

impl LockoutNotificationJob {
    pub fn new(user_id: UserId, locked_until: DateTime<Utc>) -> Self {
        Self {
            job_id: Uuid::new_v4().to_string(),
            user_id,
            locked_until,
            enqueued_at: Utc::now(),
            attempt: 0,
        }
    }

    #[allow(dead_code)]
    pub fn retrying(&self) -> Self {
        let mut next = self.clone();
        next.attempt = next.attempt.saturating_add(1);
        next
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutNotificationDeadLetter {
    pub job: LockoutNotificationJob,
    pub failed_at: DateTime<Utc>,
    pub error: String,
}

pub async fn enqueue_lockout_notification_job(
    pool: &RedisPool,
    job: &LockoutNotificationJob,
) -> anyhow::Result<()> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let payload =
        serde_json::to_string(job).map_err(|err| anyhow!("serialize lockout job: {err}"))?;
    let _: i32 = redis::cmd("RPUSH")
        .arg(LOCKOUT_NOTIFICATION_QUEUE_KEY)
        .arg(payload)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("enqueue lockout notification job: {err}"))?;
    Ok(())
}

#[allow(dead_code)]
pub async fn dequeue_lockout_notification_job(
    pool: &RedisPool,
    timeout_seconds: usize,
) -> anyhow::Result<Option<LockoutNotificationJob>> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let payload: Option<(String, String)> = redis::cmd("BRPOP")
        .arg(LOCKOUT_NOTIFICATION_QUEUE_KEY)
        .arg(timeout_seconds)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("dequeue lockout notification job: {err}"))?;
    payload
        .map(|(_, job)| {
            serde_json::from_str(&job).map_err(|err| anyhow!("deserialize lockout job: {err}"))
        })
        .transpose()
}

#[allow(dead_code)]
pub async fn schedule_lockout_notification_retry(
    pool: &RedisPool,
    job: &LockoutNotificationJob,
    retry_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let payload =
        serde_json::to_string(job).map_err(|err| anyhow!("serialize lockout job: {err}"))?;
    let _: i32 = redis::cmd("ZADD")
        .arg(LOCKOUT_NOTIFICATION_RETRY_KEY)
        .arg(retry_at.timestamp_millis())
        .arg(payload)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("schedule lockout notification retry: {err}"))?;
    Ok(())
}

#[allow(dead_code)]
pub async fn requeue_due_lockout_notification_jobs(
    pool: &RedisPool,
    now: DateTime<Utc>,
    limit: usize,
) -> anyhow::Result<usize> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let moved: i32 = redis::cmd("EVAL")
        .arg(REQUEUE_DUE_LOCKOUT_NOTIFICATION_JOBS_SCRIPT)
        .arg(2)
        .arg(LOCKOUT_NOTIFICATION_RETRY_KEY)
        .arg(LOCKOUT_NOTIFICATION_QUEUE_KEY)
        .arg(now.timestamp_millis())
        .arg(limit)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("requeue due retry jobs: {err}"))?;
    Ok(moved.max(0) as usize)
}

#[allow(dead_code)]
pub async fn push_lockout_notification_dead_letter(
    pool: &RedisPool,
    entry: &LockoutNotificationDeadLetter,
) -> anyhow::Result<()> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let payload =
        serde_json::to_string(entry).map_err(|err| anyhow!("serialize dead letter: {err}"))?;
    let _: i32 = redis::cmd("RPUSH")
        .arg(LOCKOUT_NOTIFICATION_DLQ_KEY)
        .arg(payload)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("push lockout notification dead letter: {err}"))?;
    Ok(())
}
