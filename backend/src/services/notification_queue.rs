//! Generic notification queue primitives (T-16: 汎用通知サービス基盤).
//!
//! This module generalizes the Redis-backed queue that previously existed only for account
//! lockout notifications (`lockout_notification_queue.rs`). The goal is to give later work
//! (T-17: request submitted / approved / rejected / missing clock-out notifications) an
//! extension point that does not require a parallel queue implementation per event type.
//!
//! ## Message shape: `notification_kind` + payload
//!
//! [`NotificationJob`] is an internally tagged enum (`#[serde(tag = "notification_kind")]`).
//! Each variant wraps a kind-specific payload struct (e.g. [`LockoutNotificationJob`] for
//! [`NotificationKind::AccountLockout`]). Because the tag is *internal*, serde serializes the
//! discriminant as a `notification_kind` field placed *alongside* the payload struct's own
//! fields, not nested under a separate `payload` key. Concretely, `serde_json` renders:
//!
//! ```json
//! {"notification_kind":"account_lockout","job_id":"...","user_id":"...","locked_until":"...","enqueued_at":"...","attempt":0}
//! ```
//!
//! ## Why this preserves existing wire/test compatibility
//!
//! The pre-existing lockout integration tests (`backend/tests/auth_lockout_redis_integration.rs`)
//! read a raw Redis list entry and deserialize it *directly* into [`LockoutNotificationJob`]
//! (a plain struct, not the tagged enum). Serde ignores unknown fields by default (no
//! `#[serde(deny_unknown_fields)]` on `LockoutNotificationJob`), so the extra `notification_kind`
//! field introduced by this generalization does not break that direct deserialization. This
//! compatibility guarantee is pinned by the unit tests in this module (see `tests` below) and by
//! `notification_queue_wire_compat` in `lockout_notification_queue.rs`.
//!
//! ## Legacy (untagged) payload fallback: dequeue must tolerate pre-T-16 jobs
//!
//! The compatibility guarantee above is one-directional (new tagged wire format -> old plain
//! struct). The *other* direction also has to hold across a deploy: at the moment this
//! generalization ships, the Redis queue / retry ZSET may still contain jobs that were enqueued
//! by pre-T-16 code as a bare, untagged `LockoutNotificationJob` (no `notification_kind` field
//! at all). Because [`NotificationJob`] is an *internally tagged* enum, serde requires the tag
//! field to be present to pick a variant, so [`dequeue_notification_job`] (strict tagged parse)
//! fails on such a legacy payload. Since the queue is consumed via `BRPOP` (a destructive pop),
//! a dequeue that errors out on this legacy shape would lose the job permanently — it has
//! already left the Redis list and would not land in the DLQ either, since only jobs that reach
//! `process_lockout_notification_job` can be dead-lettered.
//!
//! To avoid that, this module exposes [`dequeue_notification_payload`], which returns the raw
//! JSON string without attempting to parse it. Kind-specific adapters (currently only
//! `lockout_notification_queue::dequeue_lockout_notification_job`) use it to implement a
//! fallback: try the strict tagged [`NotificationJob`] parse first, and if that fails, retry as
//! the legacy, untagged payload shape. This module deliberately does not know about any
//! kind-specific legacy struct itself — that fallback knowledge lives in the adapter module that
//! introduced the tag requirement.
//!
//! ## Extension point for future notification kinds (T-17)
//!
//! Adding a new event type is expected to look like:
//! 1. Add a payload struct (mirrors [`LockoutNotificationJob`]'s shape: at minimum a `job_id`,
//!    `enqueued_at`, and `attempt` for retry bookkeeping, plus kind-specific fields).
//! 2. Add a variant to [`NotificationKind`] and [`NotificationJob`].
//! 3. Reuse [`enqueue_notification_job`] / [`dequeue_notification_job`] /
//!    [`schedule_notification_retry`] / [`requeue_due_notification_jobs`] /
//!    [`push_notification_dead_letter`] against either the existing lockout queue key (if the
//!    new kind should share the same Redis list and worker loop) or a new dedicated key (if it
//!    should be operated independently). Neither choice requires touching this module again.
//! 4. Any code that currently pattern-matches on [`NotificationJob`] with a single variant
//!    (e.g. `lockout_notification_queue::dequeue_lockout_notification_job`) will fail to compile
//!    until it explicitly decides how to handle the new variant — this is intentional, it is the
//!    safety net that stops a new kind from being silently mishandled by old dequeue code.
//!
//! T-17 itself (wiring handlers to enqueue these new kinds) is out of scope for this change.
//!
//! ## Application notification queue has no consumer yet: `LTRIM` cap
//!
//! `docs/exec-plans/active/EP-20260709-request-notification-events.md` scoped worker
//! delivery for the application notification variants (`RequestSubmitted` /
//! `RequestApproved` / `RequestRejected` / `MissingClockOutReminder`) out of that change —
//! unlike [`crate::services::lockout_notification_queue`], which is drained by
//! `lockout_notification_worker` via `BRPOP`, nothing currently reads
//! [`APPLICATION_NOTIFICATION_QUEUE_KEY`]. Every leave/overtime submit, approval, and
//! rejection enqueues onto it, so an unbounded `RPUSH` would grow the Redis list forever and
//! risk exhausting Redis memory in production. [`enqueue_application_notification_job`]
//! guards against that by `LTRIM`-capping the list to
//! [`APPLICATION_NOTIFICATION_QUEUE_MAX_LEN`] entries after every push, silently dropping the
//! oldest queued jobs once the cap is exceeded. This is a stopgap: once a worker exists for
//! this queue, revisit (likely remove) this cap — see the module-level doc comment on
//! [`enqueue_application_notification_job`] for details. The shared
//! [`enqueue_notification_job`] helper (also used by the lockout queue, which does not need
//! this cap because it has a consumer) is deliberately left untouched.

use anyhow::anyhow;
use bb8_redis::redis;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::{db::redis::RedisPool, services::lockout_notification_queue::LockoutNotificationJob};

pub const APPLICATION_NOTIFICATION_QUEUE_KEY: &str = "app:notifications";

/// Maximum number of jobs retained on [`APPLICATION_NOTIFICATION_QUEUE_KEY`].
///
/// This queue has no consumer yet (see the module docs' "Application notification queue has
/// no consumer yet" section), so every enqueue past this length causes
/// [`enqueue_application_notification_job`] to `LTRIM` the oldest entries off. 10_000 is a
/// generous multiple of realistic daily leave/overtime submit+decision volume for this system
/// (each request submission/approval/rejection enqueues at most one job); it exists purely to
/// bound worst-case Redis memory while no worker drains the list, not as a tuned capacity
/// figure. Revisit (most likely: remove the cap and rely on `BRPOP` draining it) once a worker
/// is implemented for this queue.
pub const APPLICATION_NOTIFICATION_QUEUE_MAX_LEN: isize = 10_000;

/// Lua script shared by every notification queue: moves due jobs from a retry ZSET (scored by
/// millisecond timestamp) back onto the head of the ready-to-process LIST.
const REQUEUE_DUE_NOTIFICATION_JOBS_SCRIPT: &str = r#"
local jobs = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1], 'LIMIT', 0, ARGV[2])
for _, job in ipairs(jobs) do
  redis.call('ZREM', KEYS[1], job)
  redis.call('RPUSH', KEYS[2], job)
end
return #jobs
"#;

/// Discriminates the payload carried by a [`NotificationJob`]. New event types (T-17: request
/// submitted / approved / rejected, missing clock-out reminder) extend this enum with an
/// additional variant instead of introducing a parallel notification type.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    AccountLockout,
    RequestSubmitted,
    RequestApproved,
    RequestRejected,
    MissingClockOutReminder,
}

/// Generic notification job envelope. See the module docs for the wire format rationale.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "notification_kind", rename_all = "snake_case")]
pub enum NotificationJob {
    AccountLockout(LockoutNotificationJob),
    RequestSubmitted(ApplicationNotificationJob),
    RequestApproved(ApplicationNotificationJob),
    RequestRejected(ApplicationNotificationJob),
    MissingClockOutReminder(ApplicationNotificationJob),
}

impl NotificationJob {
    #[allow(dead_code)]
    pub fn kind(&self) -> NotificationKind {
        match self {
            NotificationJob::AccountLockout(_) => NotificationKind::AccountLockout,
            NotificationJob::RequestSubmitted(_) => NotificationKind::RequestSubmitted,
            NotificationJob::RequestApproved(_) => NotificationKind::RequestApproved,
            NotificationJob::RequestRejected(_) => NotificationKind::RequestRejected,
            NotificationJob::MissingClockOutReminder(_) => {
                NotificationKind::MissingClockOutReminder
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationNotificationJob {
    pub job_id: String,
    pub recipient_user_id: String,
    pub actor_user_id: Option<String>,
    pub subject_user_id: String,
    pub event_id: String,
    pub event_kind: String,
    pub locale: String,
    pub enqueued_at: DateTime<Utc>,
    pub attempt: u32,
}

impl ApplicationNotificationJob {
    pub fn new(
        recipient_user_id: String,
        actor_user_id: Option<String>,
        subject_user_id: String,
        event_id: String,
        event_kind: String,
        locale: String,
    ) -> Self {
        Self {
            job_id: Uuid::new_v4().to_string(),
            recipient_user_id,
            actor_user_id,
            subject_user_id,
            event_id,
            event_kind,
            locale,
            enqueued_at: Utc::now(),
            attempt: 0,
        }
    }
}

/// Enqueues `job` onto [`APPLICATION_NOTIFICATION_QUEUE_KEY`] (`RPUSH`), then `LTRIM`s that
/// list down to the newest [`APPLICATION_NOTIFICATION_QUEUE_MAX_LEN`] entries.
///
/// **Truncation notice:** because this queue currently has no consumer (see the module docs'
/// "Application notification queue has no consumer yet" section), once the list holds more
/// than [`APPLICATION_NOTIFICATION_QUEUE_MAX_LEN`] jobs, this function silently drops the
/// oldest excess jobs (the ones nearest the head, i.e. least recently `RPUSH`ed) rather than
/// letting the list grow without bound. Callers must not assume every enqueued job survives to
/// be delivered. This is a deliberate, temporary tradeoff (unbounded Redis memory growth vs.
/// best-effort delivery) that should be revisited once a worker drains this queue.
///
/// The `RPUSH` and `LTRIM` are issued as two sequential commands rather than a single atomic
/// Lua script (unlike [`requeue_due_notification_jobs`]'s move script) — a job can transiently
/// exceed the cap by one entry between the two calls under concurrent enqueues, but that is
/// acceptable here since this is a soft memory-bound, not a hard correctness requirement.
///
/// This cap is intentionally scoped to the application notification queue only: the shared
/// [`enqueue_notification_job`] helper (also used to enqueue onto the lockout queue, which has
/// a consumer and must not be trimmed) is left unmodified.
pub async fn enqueue_application_notification_job(
    pool: &RedisPool,
    job: &NotificationJob,
) -> anyhow::Result<()> {
    enqueue_notification_job(pool, APPLICATION_NOTIFICATION_QUEUE_KEY, job).await?;
    trim_notification_queue(
        pool,
        APPLICATION_NOTIFICATION_QUEUE_KEY,
        APPLICATION_NOTIFICATION_QUEUE_MAX_LEN,
    )
    .await
}

/// `LTRIM`s `queue_key` down to its newest `max_len` entries.
///
/// Redis `LTRIM start stop` keeps the inclusive `[start, stop]` slice of the list and drops
/// everything outside it. `RPUSH` appends to the tail, so the newest entries live at the
/// highest (least negative) indices; [`ltrim_keep_newest_range`] computes the `(start, stop)`
/// pair that keeps exactly the newest `max_len` entries and drops the rest from the head.
async fn trim_notification_queue(
    pool: &RedisPool,
    queue_key: &str,
    max_len: isize,
) -> anyhow::Result<()> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let (start, stop) = ltrim_keep_newest_range(max_len);
    let _: () = redis::cmd("LTRIM")
        .arg(queue_key)
        .arg(start)
        .arg(stop)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("trim notification queue: {err}"))?;
    Ok(())
}

/// Computes the `LTRIM start stop` range that keeps only the newest `max_len` entries of a
/// `RPUSH`-appended Redis list, e.g. `ltrim_keep_newest_range(10_000)` returns `(-10_000, -1)`
/// (`LTRIM key -10000 -1`), which keeps the last 10,000 elements and drops everything before
/// them. Negative Redis list indices count from the tail (`-1` is the newest/last element), so
/// this is independent of the list's current length — Redis clamps `start` to `0` itself when
/// the list is shorter than `max_len`, so this never errors or over-trims a short list.
///
/// Extracted as a pure function (no Redis I/O) so the range arithmetic is unit-testable without
/// a live Redis connection.
fn ltrim_keep_newest_range(max_len: isize) -> (isize, isize) {
    (-max_len, -1)
}

/// Generic dead-letter envelope, mirroring [`NotificationJob`]'s tagging strategy for the
/// original job it wraps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDeadLetter {
    pub job: NotificationJob,
    pub failed_at: DateTime<Utc>,
    pub error: String,
}

/// `RPUSH job` onto `queue_key`.
pub async fn enqueue_notification_job(
    pool: &RedisPool,
    queue_key: &str,
    job: &NotificationJob,
) -> anyhow::Result<()> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let payload =
        serde_json::to_string(job).map_err(|err| anyhow!("serialize notification job: {err}"))?;
    let _: i32 = redis::cmd("RPUSH")
        .arg(queue_key)
        .arg(payload)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("enqueue notification job: {err}"))?;
    Ok(())
}

/// `BRPOP queue_key timeout_seconds`, returning the raw JSON payload without attempting to parse
/// it as a [`NotificationJob`].
///
/// This exists so that kind-specific adapters (e.g.
/// `lockout_notification_queue::dequeue_lockout_notification_job`) can implement their own
/// legacy-format fallback when the strict, internally tagged [`NotificationJob`] parse fails —
/// see the module docs' "Legacy (untagged) payload fallback" section for why that fallback is
/// necessary. This module intentionally has no knowledge of any legacy, kind-specific wire
/// format; that knowledge belongs in the adapter that introduced the breaking tag requirement.
#[allow(dead_code)]
pub async fn dequeue_notification_payload(
    pool: &RedisPool,
    queue_key: &str,
    timeout_seconds: usize,
) -> anyhow::Result<Option<String>> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let payload: Option<(String, String)> = redis::cmd("BRPOP")
        .arg(queue_key)
        .arg(timeout_seconds)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("dequeue notification job: {err}"))?;
    Ok(payload.map(|(_, job)| job))
}

/// `BRPOP queue_key timeout_seconds`, deserializing the payload back into a [`NotificationJob`].
///
/// Note: this performs a *strict* tagged parse (the payload must carry a `notification_kind`
/// field). A payload enqueued before this envelope existed (no tag) will fail to parse here —
/// callers that must tolerate that legacy shape should use [`dequeue_notification_payload`]
/// directly and add their own fallback, as `lockout_notification_queue` does.
#[allow(dead_code)]
pub async fn dequeue_notification_job(
    pool: &RedisPool,
    queue_key: &str,
    timeout_seconds: usize,
) -> anyhow::Result<Option<NotificationJob>> {
    let Some(payload) = dequeue_notification_payload(pool, queue_key, timeout_seconds).await?
    else {
        return Ok(None);
    };
    serde_json::from_str(&payload)
        .map(Some)
        .map_err(|err| anyhow!("deserialize notification job: {err}"))
}

/// Schedules `job` for a later retry by adding it to `retry_key` (a ZSET), scored by
/// `retry_at`'s millisecond timestamp.
#[allow(dead_code)]
pub async fn schedule_notification_retry(
    pool: &RedisPool,
    retry_key: &str,
    job: &NotificationJob,
    retry_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let payload =
        serde_json::to_string(job).map_err(|err| anyhow!("serialize notification job: {err}"))?;
    let _: i32 = redis::cmd("ZADD")
        .arg(retry_key)
        .arg(retry_at.timestamp_millis())
        .arg(payload)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("schedule notification retry: {err}"))?;
    Ok(())
}

/// Moves due jobs (score <= `now`) from `retry_key` back onto `queue_key`, capped at `limit`
/// jobs per call. Returns the number of jobs moved.
#[allow(dead_code)]
pub async fn requeue_due_notification_jobs(
    pool: &RedisPool,
    retry_key: &str,
    queue_key: &str,
    now: DateTime<Utc>,
    limit: usize,
) -> anyhow::Result<usize> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let moved: i32 = redis::cmd("EVAL")
        .arg(REQUEUE_DUE_NOTIFICATION_JOBS_SCRIPT)
        .arg(2)
        .arg(retry_key)
        .arg(queue_key)
        .arg(now.timestamp_millis())
        .arg(limit)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("requeue due notification jobs: {err}"))?;
    Ok(moved.max(0) as usize)
}

/// `RPUSH entry` onto `dlq_key`.
#[allow(dead_code)]
pub async fn push_notification_dead_letter(
    pool: &RedisPool,
    dlq_key: &str,
    entry: &NotificationDeadLetter,
) -> anyhow::Result<()> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("acquire redis connection: {err}"))?;
    let payload =
        serde_json::to_string(entry).map_err(|err| anyhow!("serialize dead letter: {err}"))?;
    let _: i32 = redis::cmd("RPUSH")
        .arg(dlq_key)
        .arg(payload)
        .query_async(&mut *conn)
        .await
        .map_err(|err| anyhow!("push notification dead letter: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UserId;
    use chrono::Duration;

    fn sample_lockout_job() -> LockoutNotificationJob {
        LockoutNotificationJob::new(UserId::new(), Utc::now() + Duration::minutes(15))
    }

    #[test]
    fn notification_job_serializes_with_notification_kind_tag() {
        let job = NotificationJob::AccountLockout(sample_lockout_job());
        let value: serde_json::Value =
            serde_json::to_value(&job).expect("serialize notification job");
        assert_eq!(value["notification_kind"], "account_lockout");
        // Kind-specific fields must sit at the top level (flattened), not nested under a
        // separate "payload"/"AccountLockout" key — this is what keeps direct deserialization
        // into `LockoutNotificationJob` working.
        assert!(value.get("user_id").is_some());
        assert!(value.get("locked_until").is_some());
        assert!(value.get("payload").is_none());
        assert!(value.get("AccountLockout").is_none());
    }

    #[test]
    fn application_notification_jobs_carry_kind_tag() {
        let job = ApplicationNotificationJob::new(
            "manager-1".to_string(),
            Some("employee-1".to_string()),
            "employee-1".to_string(),
            "request-1".to_string(),
            "leave".to_string(),
            "ja".to_string(),
        );
        let serialized = serde_json::to_value(NotificationJob::RequestSubmitted(job))
            .expect("serialize request notification");

        assert_eq!(serialized["notification_kind"], "request_submitted");
        assert_eq!(serialized["recipient_user_id"], "manager-1");
        assert_eq!(serialized["locale"], "ja");
    }

    #[test]
    fn notification_job_wire_format_deserializes_directly_into_lockout_notification_job() {
        let inner = sample_lockout_job();
        let job = NotificationJob::AccountLockout(inner.clone());
        let serialized = serde_json::to_string(&job).expect("serialize notification job");

        let decoded: LockoutNotificationJob =
            serde_json::from_str(&serialized).expect("deserialize as LockoutNotificationJob");
        assert_eq!(decoded.job_id, inner.job_id);
        assert_eq!(decoded.user_id, inner.user_id);
        assert_eq!(decoded.locked_until, inner.locked_until);
        assert_eq!(decoded.enqueued_at, inner.enqueued_at);
        assert_eq!(decoded.attempt, inner.attempt);
    }

    #[test]
    fn notification_job_round_trips_through_generic_envelope() {
        let inner = sample_lockout_job();
        let job = NotificationJob::AccountLockout(inner.clone());
        let serialized = serde_json::to_string(&job).expect("serialize notification job");

        let decoded: NotificationJob =
            serde_json::from_str(&serialized).expect("deserialize as NotificationJob");
        assert_eq!(decoded.kind(), NotificationKind::AccountLockout);
        match decoded {
            NotificationJob::AccountLockout(decoded_inner) => {
                assert_eq!(decoded_inner.job_id, inner.job_id);
                assert_eq!(decoded_inner.user_id, inner.user_id);
            }
            _ => panic!("expected account_lockout notification"),
        }
    }

    #[test]
    fn ltrim_keep_newest_range_keeps_exactly_max_len_from_the_tail() {
        // `LTRIM key -10000 -1` keeps the newest 10,000 entries (RPUSH appends to the tail, so
        // negative indices closest to -1 are the most recently pushed).
        assert_eq!(ltrim_keep_newest_range(10_000), (-10_000, -1));
    }

    #[test]
    fn ltrim_keep_newest_range_matches_configured_application_queue_cap() {
        assert_eq!(
            ltrim_keep_newest_range(APPLICATION_NOTIFICATION_QUEUE_MAX_LEN),
            (-APPLICATION_NOTIFICATION_QUEUE_MAX_LEN, -1)
        );
    }

    #[test]
    fn ltrim_keep_newest_range_handles_small_and_edge_lengths() {
        assert_eq!(ltrim_keep_newest_range(1), (-1, -1));
        // max_len = 0 is not a value this module ever configures, but the arithmetic should
        // still be well-defined (Redis treats LTRIM key 0 -1 as "keep everything", i.e. this
        // would be a no-op cap, not a panic/overflow) rather than behaving unpredictably.
        assert_eq!(ltrim_keep_newest_range(0), (0, -1));
    }
}
