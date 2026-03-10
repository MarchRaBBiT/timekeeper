use anyhow::anyhow;
use bb8_redis::redis;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{db::redis::RedisPool, types::UserId};

pub const LOCKOUT_NOTIFICATION_QUEUE_KEY: &str = "auth:lockout-notifications";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutNotificationJob {
    pub user_id: UserId,
    pub locked_until: DateTime<Utc>,
    pub enqueued_at: DateTime<Utc>,
}

impl LockoutNotificationJob {
    pub fn new(user_id: UserId, locked_until: DateTime<Utc>) -> Self {
        Self {
            user_id,
            locked_until,
            enqueued_at: Utc::now(),
        }
    }
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
