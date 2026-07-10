# EP-20260709-monthly-closing-workflow

## Goal

- T-12: add the monthly closing workflow before final lock.

## Scope

- In: design doc, workflow tables, self-confirm / approve / close / reopen APIs, existing lock integration.
- Out: payroll export freezing and rich admin dashboard.

## Done Criteria

- [x] `docs/design-docs/monthly-closing.md` defines states, authorization, lock semantics, and audit events.
- [x] Invalid transitions return `409 INVALID_MONTHLY_CLOSING_TRANSITION`.
- [x] `closed` transition applies existing resolved workday lock.
- [x] API catalog/OpenAPI updated.

## Validation

- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api monthly_closing_workflow_enforces_order_and_locks_on_close -- --nocapture` — passed.
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 18 passed.

## 2026-07-10: CRITICAL fix — close transition + lock atomicity

### Defect

`close_monthly_closing` (`backend/src/handlers/admin/work_schedules.rs`) called
`work_schedule::transition_monthly_closing` (own `pool.begin()...commit()`) and then
`work_schedule::close_month` (own separate `pool.begin()...commit()`) as two independent
transactions. If the first commit succeeded and the second call failed (DB error, connection
drop, etc.), `monthly_closing_workflows.status` was left `closed` while `resolved_workdays`
stayed unlocked — violating the invariant in `docs/design-docs/monthly-closing.md` that
`closed` always implies the lock is applied. Because `is_valid_monthly_transition` does not
allow `closed -> closed`, this corrupted state was unrecoverable via the API (retry returned
`409 INVALID_MONTHLY_CLOSING_TRANSITION`).

### Fix

- `backend/src/repositories/work_schedule/operations.rs`: split `transition_monthly_closing`
  and `close_month` into thin `pool`-level wrappers (unchanged public signatures/behavior,
  still begin/commit their own transaction for existing callers) plus private
  `transition_monthly_closing_tx` / `close_month_tx` core functions that operate on a borrowed
  `&mut sqlx::Transaction<'_, sqlx::Postgres>`.
- Added `close_monthly_closing_workflow(pool, user_id, year, month, actor_id, reason)`, which
  opens a single transaction, runs `transition_monthly_closing_tx` (to `closed`) then
  `close_month_tx` for the same user/month, and only commits after both succeed. Any error in
  either step drops the transaction (rollback) via `?`, so the workflow status and the
  resolved-workday lock are now updated atomically.
- `backend/src/repositories/work_schedule/mod.rs`: exported the new function.
- `backend/src/handlers/admin/work_schedules.rs`: `close_monthly_closing` now calls
  `close_monthly_closing_workflow` once instead of the two separate repository calls.
  Route/method/request/response/error contract is unchanged, so
  `docs/design-docs/backend-api-catalog.md` did not need an update (`docs-check` confirmed
  green).

### Test decision: no synthetic partial-failure test added

The existing regression `monthly_closing_workflow_enforces_order_and_locks_on_close` still
covers the end-to-end happy path (self-confirm → approve → close → status `closed` AND
`resolved_workdays.locked_at` set, in the same request) through the now-atomic code path.

A test that forces `close_month_tx` to fail *after* `transition_monthly_closing_tx` already
succeeded (to prove the old bug reproduces and the new code rolls both back) was investigated
but is not realistically constructible with this schema:
- `transition_monthly_closing_tx` and `close_month_tx` are invoked with the *same* `actor_id`
  and `reason` values. Both `monthly_closing_workflows.closed_by` and
  `work_schedule_monthly_closures.closed_by` are `REFERENCES users(id)`, and both `reason`
  columns share the same `CHECK (char_length(reason) <= 500)`. Any input that would make
  `close_month_tx`'s insert violate a constraint (bad actor id, oversized reason) also violates
  the same constraint in `transition_monthly_closing_tx`, which runs first — so the failure
  happens before `close_month_tx` is ever reached, which doesn't reproduce the original bug
  scenario.
- `close_month_tx`'s only Rust-level validation unique to it (`user_ids` existence check against
  `users`) can't be triggered for the target employee either: the employee must already exist to
  reach `approved` status, and by the time `close_monthly_closing_workflow` runs, the same
  employee already owns `resolved_workdays`/`attendance` rows (`ON DELETE RESTRICT`), so the
  employee row cannot be deleted mid-test to induce a mismatch.
- `transition_monthly_closing_tx` / `close_month_tx` are private (`async fn`, not `pub`), so an
  integration test in `backend/tests/` cannot drive them independently inside a hand-rolled
  transaction to inject a failure between the two steps without changing their visibility purely
  for test access (rejected — would widen the public surface for no product reason).

Given no realistic external input reproduces a step-1-succeeds/step-2-fails split under the new
schema, atomicity is guaranteed structurally instead: both steps run against the same borrowed
`&mut Transaction`, and `close_monthly_closing_workflow` only calls `transaction.commit()` after
both `?`-propagating calls return `Ok`. This mirrors the pattern already regression-tested by
`monthly_close_rolls_back_locks_when_closure_insert_fails`, which confirms `close_month_tx`'s
own internal steps (lock update + closure log insert) roll back together on failure.

### Validation (measured 2026-07-10)

- `cargo fmt --all --check` — passed (no diff).
- `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 18 passed;
  0 failed (includes `monthly_closing_workflow_enforces_order_and_locks_on_close` and
  `monthly_close_rolls_back_locks_when_closure_insert_fails`).
- `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` — no warnings.
- `bash scripts/harness.sh docs-check` — green (no API contract change, so no catalog edit
  required).

## 2026-07-10: HIGH fix — self-approval of monthly closing not forbidden

### Defect

`approve_monthly_closing` (`backend/src/handlers/admin/work_schedules.rs`) only called
`authorize_scope(&state, &user, target)`, which checks *department* scope (`can_manager_approve`)
but never checks whether `target == user.id`. The existing request approval handlers
(`approve_request` / `reject_request` in `backend/src/handlers/admin/requests.rs:43,95`) already
reject `applicant_id == user.id` with `AppError::Forbidden` before running the scope check, but
`approve_monthly_closing` had no equivalent guard. Since `can_manager_approve` matches on
`users.department_id IN (subordinate depts of the manager)` with no self-exclusion, a manager who
is also a member of a department they manage could: (1) `POST
/api/monthly-closings/me/self-confirm` to self-confirm their own month, then (2) `POST
/api/admin/users/{own_id}/monthly-closings/approve` to approve it themselves — completing the
"self-confirm → manager approve" two-party control alone, defeating the separation of duties the
workflow is designed to enforce (`docs/design-docs/monthly-closing.md` Authorization section).

### Fix

- `backend/src/handlers/admin/work_schedules.rs::approve_monthly_closing`: added a
  `target == user.id` check that returns `AppError::Forbidden("Managers cannot approve their own
  monthly closing")` immediately after payload validation and before `authorize_scope` runs. The
  check applies unconditionally (including to system admins), matching the existing
  `approve_request` / `reject_request` precedent, which also bans self-approval for every role.
- `docs/design-docs/monthly-closing.md`: documented the self-approval ban explicitly under
  Authorization, referencing the `approve_request` precedent.
- `docs/design-docs/backend-api-catalog.md`: updated the `Primary Errors` and summary columns for
  `/api/admin/users/{user_id}/monthly-closings/approve` to mention the `403` self-approval
  rejection (authorization requirement change, so the catalog update is in the same change per
  `docs/manual/CODING_AGENT.md`).

### Test

Added `manager_cannot_approve_own_monthly_closing` to
`backend/tests/work_schedule_phase2_api.rs`: seeds a system admin (to generate the projection,
since `/api/admin/work-schedule-projections/generate` requires system admin) and a manager who is
also placed as a member of the department they manage via `assign_manager_to_employee_department`.
The manager self-confirms their own month, then attempts
`POST /api/admin/users/{manager_id}/monthly-closings/approve` on themselves and asserts `403`, and
asserts the workflow status in `monthly_closing_workflows` is still `self_confirmed` (the approve
attempt did not mutate state).

### Validation (measured 2026-07-10)

- `cargo fmt --all --check` — passed (no diff).
- `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 19 passed
  (18 existing + new `manager_cannot_approve_own_monthly_closing`); 0 failed. (Test initially
  failed with `left: 403, right: 200` because the projection-generation step used the manager
  instead of a system admin; fixed by seeding a dedicated `system_admin` user for that step.)
- `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` — no warnings.
- `bash scripts/harness.sh docs-check` — green.

## 2026-07-10: MEDIUM fixes — phantom-insert race and unmapped FK violation in `transition_monthly_closing_tx`

### Defect 1: initial workflow INSERT race surfaces as raw 500

`transition_monthly_closing_tx` (`backend/src/repositories/work_schedule/operations.rs`) fetched
the existing `monthly_closing_workflows` row with `SELECT ... FOR UPDATE`, but `FOR UPDATE` only
locks rows that exist — it cannot protect against a *phantom insert*. Two concurrent first-requests
for the same `user_id`/`year`/`month` (e.g. a double-click on self-confirm) could both observe
`existing = None`, and both then ran a bare `INSERT INTO monthly_closing_workflows (...) VALUES
(..., 'open')`. The loser of that race hit the `monthly_closing_workflows_user_month_key` UNIQUE
violation, which propagated through `?` as `WorkScheduleRepositoryError::Sqlx` — a raw `500`
instead of the intended `409 INVALID_MONTHLY_CLOSING_TRANSITION`.

### Defect 2: transition to a nonexistent `user_id` surfaces as raw 500

`approve_monthly_closing`, `close_monthly_closing`, and `reopen_monthly_closing`
(`backend/src/handlers/admin/work_schedules.rs`) only validate the path `user_id` with
`UserId::from_str` (format only) before calling into `transition_monthly_closing` /
`close_monthly_closing_workflow`. `approve_monthly_closing`'s `authorize_scope` returns `Ok`
immediately for system admins without checking the target exists. A well-formed but nonexistent
`user_id` therefore reached the repository INSERT, which violated
`monthly_closing_workflows.user_id REFERENCES users(id)` — surfaced as a raw `500` because the
INSERT/UPDATE statements in `transition_monthly_closing_tx` used bare `?` instead of the existing
`map_database_error` helper (`backend/src/repositories/work_schedule/mod.rs`) that already
translates `*_fkey` constraint violations into `WorkScheduleRepositoryError::InvalidReference` (used
elsewhere, e.g. `master.rs:88`, `versions.rs:68`).

### Fix

Both fixed in `backend/src/repositories/work_schedule/operations.rs`,
`transition_monthly_closing_tx`, without adding any new error variant or error code:

- Replaced the bare first-insert with
  `INSERT ... ON CONFLICT (user_id, year, month) DO NOTHING RETURNING id, status`, wrapped in
  `.map_err(map_database_error)` (imported via `super::map_database_error`, already `pub(super)`-
  visible to sibling submodules — see `master.rs`/`versions.rs` for the existing pattern). If the
  conflict branch returns nothing (i.e. this request lost the race), the code falls through to a
  second `SELECT id, status ... FOR UPDATE`, which blocks until the winner's transaction commits
  and then reads the now-current row. Execution then continues through the normal
  `is_valid_monthly_transition` check exactly as it would have for a genuinely pre-existing row, so
  the loser gets the ordinary `409 INVALID_MONTHLY_CLOSING_TRANSITION` (or `200`, if the winner's
  transition happens to leave the row in a state from which the loser's requested transition is
  still valid) instead of a raw `500`.
- Added `.map_err(map_database_error)` to the `UPDATE monthly_closing_workflows ... RETURNING`
  statement as well, since its `self_confirmed_by` / `approved_by` / `closed_by` / `reopened_by`
  columns are also `REFERENCES users(id)` and are populated with `actor_id` when the corresponding
  status transition matches — covering the same FK-violation-to-`InvalidReference` translation for
  that statement.
- No changes to `close_month_tx`, `is_valid_monthly_transition`, or any error code/variant. The
  existing `map_repository_error` in `backend/src/handlers/admin/work_schedules.rs` already maps
  `InvalidReference` → `400 INVALID_WORK_SCHEDULE_REFERENCE` and `InvalidStateTransition` → `409
  INVALID_MONTHLY_CLOSING_TRANSITION`, so no handler changes were needed beyond what already
  existed.

### Tests added (`backend/tests/work_schedule_phase2_api.rs`)

- `approve_monthly_closing_rejects_unknown_target_user`: system admin (bypasses `authorize_scope`
  department check) calls approve with a syntactically valid but nonexistent `user_id`
  (`Uuid::new_v4()`); asserts `400 INVALID_WORK_SCHEDULE_REFERENCE` and that no
  `monthly_closing_workflows` row was created for that id (confirms the failed INSERT did not leave
  partial state — expected, since it's inside a transaction that gets rolled back on error).
- `concurrent_first_self_confirm_requests_do_not_500`: fires two concurrent
  `POST /api/monthly-closings/me/self-confirm` requests for the same employee/year/month via
  `tokio::join!` (two separate `request_json` futures against `pool.clone()`, so they genuinely race
  at the DB level — `integration_guard()` only serializes across `#[tokio::test]` functions in this
  file, not within one). Asserts the resulting status codes, sorted, are exactly `[200, 409]` (never
  `500`), and that exactly one `monthly_closing_workflows` row exists afterward. Ran 5x locally with
  no flakes (deterministic because the second `SELECT ... FOR UPDATE` blocks on the winner's
  transaction).
- Decided against adding a similarly deterministic test for defect 2 against `close`/`reopen` (only
  `approve` was tested): all three routes go through the same
  `transition_monthly_closing_tx`/`map_database_error` code path, and `close_monthly_closing`/
  `reopen_monthly_closing` require `require_system_admin` (simpler auth than `approve`'s
  scope+self-approval checks), so the `approve` case is the one most likely to have been reached by
  a caller in practice and is a representative regression test for all three.

### Validation (measured 2026-07-10)

- `cargo fmt --all --check` — passed (no diff after `cargo fmt --all`).
- `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 24 passed; 0
  failed (22 existing + `approve_monthly_closing_rejects_unknown_target_user` +
  `concurrent_first_self_confirm_requests_do_not_500`).
- `concurrent_first_self_confirm_requests_do_not_500` alone, run 5x — 5/5 passed, no flakes.
- `cargo test -p timekeeper-backend --lib` — 403 passed; 0 failed (no regression in unrelated unit
  tests).
- `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` — no warnings.
- `bash scripts/harness.sh docs-check` — green, after updating
  `docs/design-docs/backend-api-catalog.md`: added `400 INVALID_WORK_SCHEDULE_REFERENCE` to the
  Primary Errors column for the `approve` / `close` / `reopen` monthly-closing rows (defect 2), and
  added a note to the `self-confirm` row's description clarifying that concurrent first-requests no
  longer produce a `500` (defect 1; no new status code, so no Primary Errors column change needed
  there).

