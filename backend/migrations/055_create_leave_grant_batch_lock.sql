-- M-1a (docs/design-docs/backend-api-catalog.md): mutual exclusion for concurrent
-- invocations of POST /api/admin/leave-grants/run.
--
-- Per-user correctness (no negative balances, no duplicate grants) is already
-- guaranteed by the `users` row FOR UPDATE lock taken inside each per-user
-- transaction (H-2, crates/infra-postgres/src/leave_ledger.rs::with_user_lock).
-- This table only prevents two full batch runs from executing at the same
-- time, to avoid wasted duplicate work.
--
-- Why a claim/release row instead of a Postgres advisory lock:
-- a session-level pg_advisory_lock must be held on a dedicated connection for
-- the whole batch duration. The grant batch runs one transaction per user on
-- the same pool, so pinning one connection for the batch deadlocks small
-- pools (and pg_advisory_xact_lock is impossible because there is no single
-- transaction spanning the batch). Claiming a row in a short standalone
-- UPDATE keeps every acquisition/release a normal pooled query.
--
-- Crash recovery: if the process dies between claim and release, the claim
-- goes stale. A claim older than the stale timeout
-- (crates/infra-postgres/src/leave_ledger.rs::LEAVE_GRANT_BATCH_STALE_AFTER_SECONDS)
-- is reclaimable by the next run, so no manual intervention is needed.
-- `claim_token` ensures a batch can only release its own claim (a slow batch
-- whose claim was reclaimed as stale cannot release the new owner's claim).
CREATE TABLE leave_grant_batch_lock (
    id SMALLINT PRIMARY KEY CHECK (id = 1),
    running BOOLEAN NOT NULL DEFAULT FALSE,
    claim_token UUID,
    started_at TIMESTAMPTZ,
    started_by TEXT
);

INSERT INTO leave_grant_batch_lock (id, running) VALUES (1, FALSE);
