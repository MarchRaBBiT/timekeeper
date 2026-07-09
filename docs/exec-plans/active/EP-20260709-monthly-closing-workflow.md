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

