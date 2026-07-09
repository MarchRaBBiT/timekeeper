# EP-20260709-overtime-monitor-api

## Goal

- T-08: expose a 36-agreement overtime monitoring read API.

## Scope

- In: effective-dated overtime monitor settings, admin read API, threshold status calculation.
- Out: frontend dashboard integration.

## Done Criteria

- [x] `overtime_monitor_settings` migration exists with default thresholds.
- [x] `GET /api/admin/overtime-monitor` returns monthly/yearly/rolling-average statuses.
- [x] System admin can get/upsert monitor settings.
- [x] API catalog/OpenAPI updated.

## Validation

- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api overtime_monitor_reports_threshold_statuses -- --nocapture` — passed.
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 18 passed.

