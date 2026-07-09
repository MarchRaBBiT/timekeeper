# EP-20260709-punctuality-break-anomalies

## Goal

- T-09/T-10: detect late, early leave, absent, and insufficient break anomalies.

## Scope

- In: fixed-schedule punctuality anomalies, past-day absence, configured break minimum checks.
- Out: hard rejection of punches or closes.

## Done Criteria

- [x] `late`, `early_leave`, `absent`, and `insufficient_break` anomaly kinds are in the contract.
- [x] Approved leave days are excluded from absence detection.
- [x] Boundary behavior is covered by focused integration tests.
- [x] API catalog/OpenAPI updated.

## Validation

- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api anomaly_list_detects_overtime_punctuality_absence_and_break_warnings -- --nocapture` — passed.
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 18 passed.

