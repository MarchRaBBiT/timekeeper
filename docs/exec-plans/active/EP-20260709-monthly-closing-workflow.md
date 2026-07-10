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

