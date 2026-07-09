//! Account lockout notification queue.
//!
//! This is the reference (and, so far, only) consumer of the generic notification envelope
//! defined in [`crate::services::notification_queue`] (T-16: 汎用通知サービス基盤). All Redis key
//! names, struct shapes, and function signatures below are unchanged from before that
//! generalization — only the internal wire representation changed (jobs are now enqueued via
//! [`crate::services::notification_queue::NotificationJob::AccountLockout`], which adds a
//! `notification_kind` tag alongside this struct's own fields). See the module docs on
//! `notification_queue` for why that tag does not break direct deserialization into
//! [`LockoutNotificationJob`].
//!
//! `dequeue_lockout_notification_job` also has to accept the *legacy*, untagged wire format
//! (a bare `LockoutNotificationJob` with no `notification_kind` field), since a job enqueued by
//! pre-T-16 code may still be sitting in the queue/retry ZSET when this generalization deploys.
//! See `decode_lockout_notification_payload` below and the "Legacy (untagged) payload fallback"
//! section of the `notification_queue` module docs.

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::redis::RedisPool,
    services::notification_queue::{self, NotificationDeadLetter, NotificationJob},
    types::UserId,
};

pub const LOCKOUT_NOTIFICATION_QUEUE_KEY: &str = "auth:lockout-notifications";
#[allow(dead_code)]
pub const LOCKOUT_NOTIFICATION_RETRY_KEY: &str = "auth:lockout-notifications:retry";
#[allow(dead_code)]
pub const LOCKOUT_NOTIFICATION_DLQ_KEY: &str = "auth:lockout-notifications:dlq";

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
    notification_queue::enqueue_notification_job(
        pool,
        LOCKOUT_NOTIFICATION_QUEUE_KEY,
        &NotificationJob::AccountLockout(job.clone()),
    )
    .await
}

#[allow(dead_code)]
pub async fn dequeue_lockout_notification_job(
    pool: &RedisPool,
    timeout_seconds: usize,
) -> anyhow::Result<Option<LockoutNotificationJob>> {
    let Some(raw) = notification_queue::dequeue_notification_payload(
        pool,
        LOCKOUT_NOTIFICATION_QUEUE_KEY,
        timeout_seconds,
    )
    .await?
    else {
        return Ok(None);
    };
    decode_lockout_notification_payload(&raw).map(Some)
}

/// Decodes a raw queue/retry-ZSET payload into a [`LockoutNotificationJob`], tolerating both the
/// current and the legacy (pre-T-16) wire format.
///
/// Two shapes must be accepted:
/// 1. The current, internally tagged [`NotificationJob::AccountLockout`] envelope (carries a
///    `notification_kind` field), produced by [`enqueue_lockout_notification_job`] since T-16.
/// 2. The legacy, untagged `LockoutNotificationJob` shape (no `notification_kind` field at
///    all), which pre-T-16 code enqueued directly. A job in this shape can still be sitting in
///    `LOCKOUT_NOTIFICATION_QUEUE_KEY` or `LOCKOUT_NOTIFICATION_RETRY_KEY` at the moment this
///    generalization deploys, if it was enqueued before the deploy and not yet consumed.
///
/// [`NotificationJob`]'s internally tagged representation requires the tag field to select a
/// variant, so parsing a legacy payload directly as [`NotificationJob`] fails; this function
/// falls back to decoding it as a bare [`LockoutNotificationJob`] instead. This matters because
/// the queue is consumed via a destructive `BRPOP` (see
/// [`notification_queue::dequeue_notification_payload`]): if both parses failed here, the
/// calling job would already be popped off the list with no path back onto the queue or into
/// the DLQ, i.e. it would be silently lost rather than degraded gracefully.
///
/// Returns an error only if *both* interpretations fail, with both failure reasons included so
/// an operator can tell whether the payload is legacy-but-malformed or genuinely corrupt.
#[allow(dead_code)]
fn decode_lockout_notification_payload(raw: &str) -> anyhow::Result<LockoutNotificationJob> {
    match serde_json::from_str::<NotificationJob>(raw) {
        Ok(NotificationJob::AccountLockout(job)) => Ok(job),
        Ok(other) => Err(anyhow!(
            "expected account_lockout notification job, got {:?}",
            other.kind()
        )),
        Err(tagged_err) => {
            serde_json::from_str::<LockoutNotificationJob>(raw).map_err(|legacy_err| {
                anyhow!(
                    "decode lockout notification job failed for both tagged and legacy formats: \
                     tagged_parse_error={tagged_err}; legacy_parse_error={legacy_err}"
                )
            })
        }
    }
}

#[allow(dead_code)]
pub async fn schedule_lockout_notification_retry(
    pool: &RedisPool,
    job: &LockoutNotificationJob,
    retry_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    notification_queue::schedule_notification_retry(
        pool,
        LOCKOUT_NOTIFICATION_RETRY_KEY,
        &NotificationJob::AccountLockout(job.clone()),
        retry_at,
    )
    .await
}

#[allow(dead_code)]
pub async fn requeue_due_lockout_notification_jobs(
    pool: &RedisPool,
    now: DateTime<Utc>,
    limit: usize,
) -> anyhow::Result<usize> {
    notification_queue::requeue_due_notification_jobs(
        pool,
        LOCKOUT_NOTIFICATION_RETRY_KEY,
        LOCKOUT_NOTIFICATION_QUEUE_KEY,
        now,
        limit,
    )
    .await
}

#[allow(dead_code)]
pub async fn push_lockout_notification_dead_letter(
    pool: &RedisPool,
    entry: &LockoutNotificationDeadLetter,
) -> anyhow::Result<()> {
    notification_queue::push_notification_dead_letter(
        pool,
        LOCKOUT_NOTIFICATION_DLQ_KEY,
        &NotificationDeadLetter {
            job: NotificationJob::AccountLockout(entry.job.clone()),
            failed_at: entry.failed_at,
            error: entry.error.clone(),
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_job() -> LockoutNotificationJob {
        LockoutNotificationJob::new(UserId::new(), Utc::now() + Duration::minutes(15))
    }

    /// Pins the exact compatibility guarantee that lets
    /// `backend/tests/auth_lockout_redis_integration.rs::queued_lockout_notifications` keep
    /// deserializing raw queue entries directly into `LockoutNotificationJob`, even though this
    /// module now enqueues through the generic, kind-tagged `NotificationJob` envelope.
    #[test]
    fn notification_queue_wire_compat_deserializes_into_lockout_notification_job() {
        let job = sample_job();
        let enqueued_shape = NotificationJob::AccountLockout(job.clone());
        let serialized = serde_json::to_string(&enqueued_shape).expect("serialize");

        let decoded: LockoutNotificationJob =
            serde_json::from_str(&serialized).expect("decode directly as LockoutNotificationJob");
        assert_eq!(decoded.job_id, job.job_id);
        assert_eq!(decoded.user_id, job.user_id);
        assert_eq!(decoded.locked_until, job.locked_until);
        assert_eq!(decoded.attempt, job.attempt);
    }

    #[test]
    fn retrying_increments_attempt_without_mutating_original() {
        let job = sample_job();
        let retried = job.retrying();
        assert_eq!(job.attempt, 0);
        assert_eq!(retried.attempt, 1);
        assert_eq!(retried.job_id, job.job_id);
    }

    /// Pins the backward-compat direction that the forward-compat test above does not cover:
    /// a job enqueued by pre-T-16 code (a bare `LockoutNotificationJob`, no `notification_kind`
    /// tag at all) must still decode correctly through the current dequeue path. Without this
    /// fallback, such an in-flight job would be lost silently on deploy (see module docs).
    #[test]
    fn decode_lockout_notification_payload_falls_back_to_legacy_untagged_shape() {
        let job = sample_job();
        // Legacy producers serialized `LockoutNotificationJob` directly — no `NotificationJob`
        // envelope, hence no `notification_kind` field.
        let legacy_payload = serde_json::to_string(&job).expect("serialize legacy shape");
        assert!(
            !legacy_payload.contains("notification_kind"),
            "legacy payload must not carry the tag field: {legacy_payload}"
        );

        let decoded = decode_lockout_notification_payload(&legacy_payload)
            .expect("legacy payload should decode via fallback");
        assert_eq!(decoded.job_id, job.job_id);
        assert_eq!(decoded.user_id, job.user_id);
        assert_eq!(decoded.locked_until, job.locked_until);
        assert_eq!(decoded.attempt, job.attempt);
    }

    /// Current (tagged) payloads must still take the primary parse path, not the fallback.
    #[test]
    fn decode_lockout_notification_payload_decodes_current_tagged_shape() {
        let job = sample_job();
        let tagged_payload = serde_json::to_string(&NotificationJob::AccountLockout(job.clone()))
            .expect("serialize tagged shape");

        let decoded = decode_lockout_notification_payload(&tagged_payload)
            .expect("tagged payload should decode");
        assert_eq!(decoded.job_id, job.job_id);
        assert_eq!(decoded.user_id, job.user_id);
    }

    /// Payloads that are neither a valid tagged envelope nor a valid legacy struct must fail
    /// with a message that surfaces both parse errors, not just the first one tried.
    #[test]
    fn decode_lockout_notification_payload_reports_both_errors_when_genuinely_corrupt() {
        let err = decode_lockout_notification_payload("{\"not\":\"a job\"}")
            .expect_err("garbage payload must not decode");
        let message = err.to_string();
        assert!(message.contains("tagged_parse_error"), "message: {message}");
        assert!(message.contains("legacy_parse_error"), "message: {message}");
    }
}
