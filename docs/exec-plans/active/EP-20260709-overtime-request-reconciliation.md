# EP-20260709-overtime-request-reconciliation

## Goal

- T-07: approved overtime requests and actual overtime are reconciled as work-schedule anomalies.

## Scope

- In: `unapproved_overtime` / `overtime_exceeds_request` anomaly kinds, admin anomaly list and calendar exposure.
- Out: blocking punches or monthly close.

## Done Criteria

- [x] Unapproved overtime is detected.
- [x] Actual overtime above approved request minutes is detected.
- [x] `backend/tests/work_schedule_phase2_api.rs` covers both paths.
- [x] API catalog/OpenAPI updated.

## Validation

- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 18 passed.

