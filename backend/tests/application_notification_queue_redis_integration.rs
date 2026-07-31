//! Regression test for the MEDIUM operational-risk fix on `app:notifications`.
//!
//! Context: `enqueue_application_notification_job` (leave/overtime submit + approval/rejection)
//! has no consumer yet — `docs/exec-plans/completed/EP-20260709-request-notification-events.md`
//! explicitly scoped worker delivery for these events out of that change. Unlike the lockout
//! queue (`auth:lockout-notifications`, drained by `lockout_notification_worker`), nothing
//! `BRPOP`s `app:notifications`, so an unbounded `RPUSH` there would let the Redis list grow
//! without limit in production. This test pins the fix: `enqueue_application_notification_job`
//! must cap that list at `APPLICATION_NOTIFICATION_QUEUE_MAX_LEN` via `LTRIM`, dropping the
//! oldest entries first, while leaving the shared lockout queue behavior untouched.
//!
//! Pushing `APPLICATION_NOTIFICATION_QUEUE_MAX_LEN` (10,000) real jobs one at a time through
//! `enqueue_application_notification_job` to observe trimming would work but is needlessly slow
//! for a test. Instead this test bulk-seeds the queue to `MAX_LEN - 2` entries with one raw
//! `RPUSH` (bypassing the function under test, since seeding is not what's being verified), then
//! calls `enqueue_application_notification_job` three times through the real function. That
//! pushes the list to `MAX_LEN + 1`, which must trigger exactly one entry to be trimmed from the
//! head.

use bb8_redis::redis;
use chrono::Utc;
use timekeeper_backend::{
    config::Config,
    db::redis::create_redis_pool,
    services::notification_queue::{
        enqueue_application_notification_job, ApplicationNotificationJob, NotificationJob,
        APPLICATION_NOTIFICATION_QUEUE_KEY, APPLICATION_NOTIFICATION_QUEUE_MAX_LEN,
    },
};

mod support;
use support::integration_guard;

fn test_config(redis_url: String) -> Config {
    let mut config = support::test_config();
    config.redis_url = Some(redis_url);
    config.feature_redis_cache_enabled = true;
    config
}

async fn flush_redis(redis_url: &str) {
    let client = redis::Client::open(redis_url).expect("open redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("connect redis");
    let _: () = redis::cmd("FLUSHDB")
        .query_async(&mut conn)
        .await
        .expect("flush redis");
}

/// Seeds `app:notifications` with `count` placeholder entries via a single raw `RPUSH`,
/// bypassing `enqueue_application_notification_job` entirely — this test is about the trim
/// behavior of that function once the cap is reached, not about how the queue got close to the
/// cap.
async fn seed_placeholder_jobs(redis_url: &str, count: usize) {
    let client = redis::Client::open(redis_url).expect("open redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("connect redis");
    let placeholders: Vec<String> = (0..count).map(|i| format!("placeholder-{i}")).collect();
    let mut cmd = redis::cmd("RPUSH");
    cmd.arg(APPLICATION_NOTIFICATION_QUEUE_KEY);
    for placeholder in &placeholders {
        cmd.arg(placeholder);
    }
    let _: i64 = cmd.query_async(&mut conn).await.expect("seed placeholders");
}

async fn queue_len(redis_url: &str) -> i64 {
    let client = redis::Client::open(redis_url).expect("open redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("connect redis");
    redis::cmd("LLEN")
        .arg(APPLICATION_NOTIFICATION_QUEUE_KEY)
        .query_async(&mut conn)
        .await
        .expect("read app notification queue len")
}

async fn queue_entries(redis_url: &str) -> Vec<String> {
    let client = redis::Client::open(redis_url).expect("open redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("connect redis");
    redis::cmd("LRANGE")
        .arg(APPLICATION_NOTIFICATION_QUEUE_KEY)
        .arg(0)
        .arg(-1)
        .query_async(&mut conn)
        .await
        .expect("read app notification queue entries")
}

fn sample_notification(event_id: &str) -> NotificationJob {
    NotificationJob::RequestSubmitted(ApplicationNotificationJob::new(
        "manager-1".to_string(),
        Some("employee-1".to_string()),
        "employee-1".to_string(),
        event_id.to_string(),
        "leave".to_string(),
        "ja".to_string(),
    ))
}

#[tokio::test]
async fn enqueue_application_notification_job_trims_oldest_entries_past_cap() {
    let _guard = integration_guard().await;
    let fixture = support::profile::db_and_redis().await;
    let redis_url = fixture.redis_url.clone();
    flush_redis(&redis_url).await;

    let max_len =
        usize::try_from(APPLICATION_NOTIFICATION_QUEUE_MAX_LEN).expect("max len fits in usize");

    // Seed the queue to two below the cap so the three real enqueues below cross it by exactly
    // one entry (max_len - 2 + 3 = max_len + 1).
    seed_placeholder_jobs(&redis_url, max_len - 2).await;
    assert_eq!(queue_len(&redis_url).await, (max_len - 2) as i64);

    let redis_pool = create_redis_pool(&test_config(redis_url.clone()))
        .await
        .expect("create redis pool")
        .expect("redis pool available");

    for event_id in ["event-a", "event-b", "event-c"] {
        enqueue_application_notification_job(&redis_pool, &sample_notification(event_id))
            .await
            .expect("enqueue application notification job");
    }

    // The list must never exceed the cap, even though 3 real jobs were pushed on top of
    // max_len - 2 seeded entries (which would otherwise leave max_len + 1 entries).
    assert_eq!(
        queue_len(&redis_url).await,
        APPLICATION_NOTIFICATION_QUEUE_MAX_LEN as i64,
        "queue length must be trimmed down to the configured cap"
    );

    let entries = queue_entries(&redis_url).await;
    assert_eq!(entries.len(), max_len);

    // The single oldest placeholder ("placeholder-0") must have been dropped by the trim; the
    // rest of the placeholders plus all three real jobs (newest-pushed) must survive.
    assert!(
        !entries.contains(&"placeholder-0".to_string()),
        "oldest placeholder must be trimmed, got head entries: {:?}",
        &entries[..3.min(entries.len())]
    );
    assert!(entries.contains(&"placeholder-1".to_string()));

    let tail = &entries[entries.len() - 3..];
    for (entry, event_id) in tail.iter().zip(["event-a", "event-b", "event-c"]) {
        assert!(
            entry.contains(event_id),
            "expected tail entry to carry event_id {event_id}, got {entry}"
        );
    }
}

#[tokio::test]
async fn enqueue_application_notification_job_does_not_trim_below_cap() {
    let _guard = integration_guard().await;
    let fixture = support::profile::db_and_redis().await;
    let redis_url = fixture.redis_url.clone();
    flush_redis(&redis_url).await;

    let redis_pool = create_redis_pool(&test_config(redis_url.clone()))
        .await
        .expect("create redis pool")
        .expect("redis pool available");

    for event_id in ["event-a", "event-b", "event-c"] {
        enqueue_application_notification_job(&redis_pool, &sample_notification(event_id))
            .await
            .expect("enqueue application notification job");
    }

    assert_eq!(
        queue_len(&redis_url).await,
        3,
        "well under the cap: no entries should be trimmed"
    );

    let entries = queue_entries(&redis_url).await;
    assert_eq!(entries.len(), 3);
    let now = Utc::now();
    for entry in &entries {
        let value: serde_json::Value = serde_json::from_str(entry).expect("valid json job");
        let enqueued_at: chrono::DateTime<Utc> = value["enqueued_at"]
            .as_str()
            .expect("enqueued_at present")
            .parse()
            .expect("valid timestamp");
        assert!(enqueued_at <= now);
    }
}
