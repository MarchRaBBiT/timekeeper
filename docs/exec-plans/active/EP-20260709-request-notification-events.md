# EP-20260709-request-notification-events

## Goal

- T-17: route request workflow events through the generalized notification queue.

## Scope

- In: notification job variants for request submitted/approved/rejected and missing clock-out reminders; request submit/decision enqueue hooks.
- Out: SMTP rendering/worker delivery for the new application notification variants.

## Done Criteria

- [x] Generic notification envelope supports application notification variants.
- [x] Leave/overtime submission enqueues `request_submitted` jobs when Redis is enabled.
- [x] Approval/rejection enqueues applicant notification jobs when Redis is enabled.
- [x] Enqueue failure does not fail the primary request mutation.

## Validation

- [x] `cargo check -p timekeeper-backend` — passed.

## Follow-up: MEDIUM operational risk — `app:notifications` unbounded growth (2026-07-11)

**Finding:** this ExecPlan explicitly scoped worker delivery for the application notification
variants out ("Out: ... worker delivery for the new application notification variants"). That
means `app:notifications` (`APPLICATION_NOTIFICATION_QUEUE_KEY`) has an enqueue side
(`enqueue_application_notification_job`, called from every leave/overtime submit, approval, and
rejection) but no consumer — unlike `auth:lockout-notifications`, which
`lockout_notification_worker` drains via `BRPOP`. In production this would let the Redis list
grow without bound and risk exhausting Redis memory.

**Fix:** `enqueue_application_notification_job` in
`backend/src/services/notification_queue.rs` now `LTRIM`s `app:notifications` down to
`APPLICATION_NOTIFICATION_QUEUE_MAX_LEN` (`10_000`, `pub const`) immediately after every
`RPUSH`, silently dropping the oldest entries once the cap is exceeded. The `RPUSH`/`LTRIM` pair
is issued as two sequential Redis commands (not a single atomic Lua script, unlike
`requeue_due_notification_jobs`'s move script) — a transient overshoot of at most one entry
under concurrent enqueues is accepted as a soft memory bound, not a hard correctness guarantee.
The shared `enqueue_notification_job` helper (also used by the lockout queue) is untouched, so
`auth:lockout-notifications` / `lockout_notification_worker` behavior is unaffected.

**Cap value rationale (10,000):** a generous multiple of realistic daily leave/overtime
submit + approval/rejection volume for this system; it exists to bound worst-case Redis memory
while no worker drains the queue, not as a tuned capacity figure. Revisit this constant (most
likely: remove the cap and rely on `BRPOP` draining instead) once a worker is implemented for
`app:notifications` — see the "Out of scope" line above for that follow-up.

**Tests added:**
- `backend/src/services/notification_queue.rs` unit tests (`cfg(test)`): pin the pure
  `ltrim_keep_newest_range` range-arithmetic helper extracted from the trim logic (no Redis
  required) — `ltrim_keep_newest_range_keeps_exactly_max_len_from_the_tail`,
  `ltrim_keep_newest_range_matches_configured_application_queue_cap`,
  `ltrim_keep_newest_range_handles_small_and_edge_lengths`.
- `backend/tests/application_notification_queue_redis_integration.rs` (new file, follows the
  `support::profile::db_and_redis()` + testcontainers pattern already used by
  `auth_lockout_redis_integration.rs`): `enqueue_application_notification_job_trims_oldest_entries_past_cap`
  bulk-seeds the queue to `MAX_LEN - 2` via a raw `RPUSH` (bypassing the function under test,
  since seeding volume isn't what's being verified) and then calls the real
  `enqueue_application_notification_job` three times, asserting the list never exceeds the cap
  and that the oldest entry (not the newly-pushed ones) is what gets dropped.
  `enqueue_application_notification_job_does_not_trim_below_cap` asserts the well-under-cap case
  is unaffected.

**Validation actually run in this environment (no Docker/Redis available in this sandbox, so
the `backend-integration` stage itself could not be executed — matches
`scripts/harness.sh`'s documented Docker dependency for that stage):**
- `cargo fmt --all --check` — passed.
- `cargo test -p timekeeper-backend --lib` — 406 passed; 0 failed.
- `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` — no warnings.
- `cargo test -p timekeeper-backend --test application_notification_queue_redis_integration` —
  attempted; hung/timed out because no Docker daemon is available in this environment for the
  `testcontainers` Redis fixture. **Not verified green in this environment** — must be run in
  an environment with Docker (e.g. CI, or a dev box per `scripts/harness.sh backend-integration`)
  before this is treated as fully validated.

**Files changed:** `backend/src/services/notification_queue.rs` (cap constant, doc comments,
trim logic, unit tests), `backend/tests/application_notification_queue_redis_integration.rs`
(new).

