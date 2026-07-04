# Operational Runbook

## Redis Failure

### Symptoms
- Increased latency in API requests (especially authenticated ones).
- Tracing shows `redis_cache_token` or `redis_is_token_active` errors.
- System fallback to PostgreSQL for token validation.

### Recovery
The system implements a graceful fallback to PostgreSQL. If Redis is down, authentication will still work but with higher DB load.

1. **Verify Redis Status**: Check if the Redis container/server is running.
2. **Restore Redis**: Restart the Redis service.
3. **Cache Re-population**: The cache will be re-populated as users log in or perform requests (cache-aside pattern).

### Configuration
Disable caching if Redis remains unstable:
`FEATURE_REDIS_CACHE_ENABLED=false`

## Database Connection Issues

### Symptoms
- `db_pool_connect` errors in logs.
- API returns 500 status codes.

### Recovery
1. **Check primary DB**: Ensure PostgreSQL is reachable.
2. **Read Replica Fallback**: If `READ_DATABASE_URL` is configured, the system will still use it for GETs. If the primary is down, writes will fail but reads might still work.
3. **Scale Pool**: Increase `DB_MAX_CONNECTIONS` if the pool is exhausted.

## Notification Worker Operations

Covers `backend/src/bin/lockout_notification_worker.rs` (the queue/worker pair behind account
lockout emails) and the generic notification envelope it now runs on
(`backend/src/services/notification_queue.rs`, introduced by T-16 to give future notification
kinds — request submitted/approved/rejected, missing clock-out reminders per T-17 — a shared
queue/worker infrastructure instead of a bespoke one per event type). This closes
`docs/exec-plans/tech-debt-tracker.md` item #7 ("Queue / Worker Operational Debt")
Recommended Fix 1–2.

### Architecture in one paragraph

`enqueue_lockout_notification_job` (`backend/src/services/lockout_notification_queue.rs`) is
called synchronously from the login handler when an account gets locked
(`backend/src/handlers/auth.rs::enqueue_lockout_notification`); it only pushes a job onto a Redis
list and never sends email itself, so a slow/broken SMTP server cannot slow down the login
response. `lockout_notification_worker` (run as a separate process, `cargo run --bin
lockout_notification_worker`) is what actually sends the email, via
`services::lockout_notification_worker::work_once`. Today only the `account_lockout` kind exists
on this queue; the notification envelope is written so a future kind can share the same
RPUSH/BRPOP/ZSET/DLQ primitives (see `notification_queue.rs` module docs) without a parallel
implementation, but no other kind is wired up yet.

### Redis keys (queue / retry / DLQ)

| Purpose | Key | Redis type | Set by |
|---|---|---|---|
| Ready-to-process queue | `auth:lockout-notifications` | LIST | `enqueue_lockout_notification_job` (RPUSH), consumed by `work_once` (BRPOP) |
| Scheduled retries | `auth:lockout-notifications:retry` | ZSET (score = retry-at, ms epoch) | `schedule_lockout_notification_retry` (ZADD) on transient send failure |
| Dead letters | `auth:lockout-notifications:dlq` | LIST | `push_lockout_notification_dead_letter` (RPUSH) after retries are exhausted |

Idempotency/dedup markers (not queues, but relevant to "did this actually send twice?"
questions): `auth:lockout-notifications:sent:<user_id>:<locked_until_ms>` (SET with 7-day TTL,
prevents re-sending an already-sent notification) and
`auth:lockout-notifications:processing:<user_id>:<locked_until_ms>` (SET NX with 300s TTL, a
lease that prevents two worker instances from processing the same job concurrently).

### Retry / backoff / DLQ semantics

- Each job carries an `attempt` counter, starting at 0.
- On send failure, if `attempt + 1 < LOCKOUT_NOTIFICATION_MAX_RETRIES` (5), the job is
  re-scheduled via `schedule_lockout_notification_retry` with `attempt` incremented and a delay
  of `next_retry_delay_seconds(attempt) = 2^attempt` seconds. With the default
  `LOCKOUT_NOTIFICATION_MAX_RETRIES = 5`, the sequence of delays before dead-lettering is
  **2s, 4s, 8s, 16s** (4 scheduled retries; the 5th failure is dead-lettered instead of scheduled
  again). `next_retry_delay_seconds` itself caps its exponent at 6 (i.e. never schedules more
  than `2^6 = 64s` out), which only matters if `LOCKOUT_NOTIFICATION_MAX_RETRIES` is raised above
  5 in the future.
- On each `work_once` call (including the harness `worker-once` stage and every iteration of the
  worker's main loop), `requeue_due_jobs` first moves any retry-ZSET entries whose score
  (retry-at) is `<= now` back onto the ready queue (LIST), capped at 32 jobs per call
  (`LOCKOUT_NOTIFICATION_REQUEUE_LIMIT`). This is the "retry drain" step.
- Once `attempt + 1 >= LOCKOUT_NOTIFICATION_MAX_RETRIES`, the job is pushed onto the DLQ list
  instead of being retried again. Nothing automatically drains the DLQ — see "Draining the DLQ"
  below.

### Observing queue / retry / DLQ depth

Point `redis-cli` at the same Redis the worker/backend use (`REDIS_URL`):

```bash
# Queue depth: jobs waiting to be processed
redis-cli LLEN auth:lockout-notifications

# Retry depth: jobs scheduled for a future retry (not yet due)
redis-cli ZCARD auth:lockout-notifications:retry

# How many retry-ZSET entries are already due (should normally be ~0; a persistently
# non-zero value means the worker isn't running or can't keep up)
redis-cli ZCOUNT auth:lockout-notifications:retry -inf "$(date +%s%3N)"

# DLQ depth: jobs that exhausted all retries and need manual attention
redis-cli LLEN auth:lockout-notifications:dlq
```

A growing queue depth with a live worker running usually means SMTP is failing (check DLQ depth
growing in parallel) or the worker process is down/crashed. A growing DLQ depth means the
underlying SMTP failure is not transient — treat it like the "Redis Failure" / SMTP outage
scenarios elsewhere in this document.

### Running the worker

```bash
# Continuous loop (production mode): polls every 250ms when the queue is empty, otherwise
# processes jobs back-to-back. Requires DATABASE_URL, JWT_SECRET, and REDIS_URL to be set
# (the process exits immediately with an error if REDIS_URL is unset).
cargo run --bin lockout_notification_worker

# Drain-one-and-exit mode: runs requeue_due_jobs once, processes at most one job (if any is
# ready), then exits. This is what the harness `worker-once` stage
# (`bash scripts/harness.sh worker-once`) runs as an operational smoke check — see
# docs/manual/HARNESS.md.
cargo run --bin lockout_notification_worker -- --once
```

### Draining the DLQ

There is currently no automated DLQ replay tool; recovery is manual:

```bash
# Peek at (without removing) up to 10 dead-lettered entries
redis-cli LRANGE auth:lockout-notifications:dlq 0 9

# After fixing the underlying issue (e.g. SMTP restored), replay a single entry by moving it
# back onto the ready queue. LPOP removes-and-returns the oldest DLQ entry; RPUSH re-enqueues it
# as a fresh job (its `attempt` counter is whatever was recorded at dead-letter time, so it will
# be retried a bounded number of times again before being dead-lettered a second time if it
# still fails).
redis-cli --no-raw EVAL "local e = redis.call('LPOP', KEYS[1]); if e then redis.call('RPUSH', KEYS[2], e) end; return e" 2 auth:lockout-notifications:dlq auth:lockout-notifications

# To discard a dead-lettered entry instead of replaying it (e.g. the locked-out user's account
# was since deleted), just LPOP it without the RPUSH.
redis-cli LPOP auth:lockout-notifications:dlq
```

### Notes

- The queue/worker split means a user is never blocked at login by SMTP latency: enqueue is a
  single RPUSH and always succeeds unless Redis itself is unavailable, in which case the login
  handler logs `lockout_notification_failed` / `lockout_notification_queue_unavailable` (see
  `backend/tests/auth_flow_api.rs::lockout_records_denied_blocked_and_notification_failed_without_queue`)
  and login still proceeds — a missing lockout email is not treated as a login failure.
  `backend/tests/auth_lockout_redis_integration.rs::lockout_enqueue_latency_is_stable_even_when_worker_smtp_fails`
  is the test that pins this: enqueue latency must not measurably change based on whether the
  worker's *eventual* SMTP send later succeeds or fails.
- The wire format is generalized (T-16) but the DLQ/queue/retry Redis key names above are
  unchanged from before that generalization — only the JSON payload gained an internal
  `notification_kind` tag. Manual `redis-cli` inspection of queue/DLQ entries will show this
  extra field; it does not require different tooling to read.
- `dequeue_lockout_notification_job` falls back to decoding the pre-T-16, untagged
  `LockoutNotificationJob` shape if the tagged parse fails, so an in-flight job enqueued just
  before a T-16 deploy and still sitting on `auth:lockout-notifications` (or its retry ZSET) is
  not silently dropped by `BRPOP` after the rollout.

## Top-Level Manager Self-Approval (Requests Stuck in `pending`)

### Symptoms
- A manager whose department is a root department (`departments.parent_id IS NULL`) submits a leave or overtime request, and it stays `pending` forever.
- No other manager can approve it: `check_approval_authorization` (`backend/src/handlers/admin/common.rs`) only grants approval to a manager who oversees the applicant's department chain (`department::can_manager_approve`), and a root-department manager has no superior department to supply such a manager.
- This is an accepted design limitation, not a bug — see `docs/design-docs/department-hierarchy.md` ("既知の制約と将来の課題" → 最上位マネージャーの自己申請), which states the intended workaround is manual approval by `is_system_admin`.

### Recovery (manual approval by a system admin)
`check_approval_authorization` always authorizes `is_system_admin` users regardless of department, so only an account with `is_system_admin = true` can unblock the request.

1. **Sign in as a system admin** and open `/admin` (page title `申請承認` / "Request Approval"). This is the page that lists leave/overtime requests for approval — component `AdminRequestsSection` (i18n key `admin_components.requests.title` = `申請一覧`).
2. **Find the stuck request**: use the ユーザー (user) filter to pick the top-level manager and/or the status filter to narrow to `pending`.
   - Underlying call: `GET /api/admin/requests?status=pending&user_id=<manager_user_id>` (see `docs/design-docs/backend-api-catalog.md`).
   - Note: unlike a regular manager's list (scoped to subordinates via `list_subordinate_user_ids`), a system admin's list is unscoped (`allowed_user_ids = None` in `backend/src/handlers/admin/requests.rs::list_requests`), so the top-level manager's own request shows up here even though a manager could never approve it normally.
3. Click 詳細 (details) to review, then click 承認 (Approve) or 却下 (Reject) and enter the required comment.
   - Underlying API: `PUT /api/admin/requests/{id}/approve` or `PUT /api/admin/requests/{id}/reject` with body `{"comment": "..."}`.
   - A manager can never approve/reject their own request (`approve_request`/`reject_request` reject when `applicant_id == user.id`), so this step must be performed from a system admin account, never the applicant's own account.
4. **Verify**: reload `/admin` (or have the applicant check `/requests`, backed by `GET /api/requests/me`) and confirm the request now shows `approved` or `rejected`.

### Notes
- Attendance correction requests (`/api/admin/attendance-corrections/{id}/approve|reject`) have the same `is_system_admin` override, enforced separately by `ApproveCorrectionUseCase` / `RejectCorrectionUseCase` in `backend/src/handlers/admin/attendance_correction_requests.rs`. There is currently no admin UI section wired up for these (no route lists them), so a stuck attendance correction from a top-level manager must be resolved by calling the API directly (e.g. via `curl` with an authenticated system admin session) rather than through `/admin`.
- Longer-term fix (delegated approver / automatic escalation to system admin) is not implemented yet — tracked as `docs/exec-plans/tech-debt-tracker.md` item #11, Recommended Fix 2.
