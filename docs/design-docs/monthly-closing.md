# Monthly Closing Workflow

**Status:** Phase 2 backend API implemented
**Updated:** 2026-07-09

## Scope

This document concretizes Work Schedule Master Follow-up Design 2.

In scope:

- Per-user monthly closing workflow state.
- Observable state transitions and authorization.
- Compatibility with the existing resolved workday lock.

Out of scope:

- Payroll export value freezing. That belongs to T-14.
- Rich administrator dashboard. That belongs to T-15.
- Unlocking already locked `resolved_workdays`. Reopen records workflow intent only.

## State Machine

The workflow is per `user_id + year + month`.

Allowed transitions:

| From | To | Actor | API |
| --- | --- | --- | --- |
| `open` | `self_confirmed` | User themself | `POST /api/monthly-closings/me/self-confirm` |
| `self_confirmed` | `approved` | Scoped Manager+ | `POST /api/admin/users/{user_id}/monthly-closings/approve` |
| `approved` | `closed` | System Admin | `POST /api/admin/users/{user_id}/monthly-closings/close` |
| `closed` | `reopened` | System Admin | `POST /api/admin/users/{user_id}/monthly-closings/reopen` |
| `reopened` | `closed` | System Admin | `POST /api/admin/users/{user_id}/monthly-closings/close` |

All other transitions are rejected with `409 INVALID_MONTHLY_CLOSING_TRANSITION`.

## Locking Semantics

`closed` is the workflow state that applies the existing monthly closure effect:

- `resolved_workdays.locked_at` is set for the target user/month through the existing close-month repository path.
- Existing DB triggers keep locked `resolved_workdays`, intervals, breaks, and workday overrides immutable.
- The legacy `POST /api/admin/work-schedule-closures/monthly` remains available for bulk operational lock compatibility.

`reopened` does not unlock `resolved_workdays`. It records that follow-up correction/re-close workflow is required. A later T-14/T-15 design can decide whether reopened payroll values are blocked from export.

## Audit Trail

Each transition writes:

- Current workflow row in `monthly_closing_workflows`.
- Append-only event in `monthly_closing_workflow_events` with previous status, next status, actor, reason, and timestamp.

The transition table is the workflow source of truth. The existing `work_schedule_monthly_closures` table remains the low-level lock audit for resolved workdays.

## Authorization

- Self-confirm: authenticated user only, always for self.
- Approve: system admin or manager with existing department approval scope.
- Approve rejects self-approval: if the target `user_id` equals the acting user's own id, the
  request is rejected with `403` before the department-scope check runs, even for a system
  admin. This mirrors the existing `approve_request` / `reject_request` self-approval ban for
  leave/overtime requests (`backend/src/handlers/admin/requests.rs`) and preserves the two-party
  self-confirm-then-approve control when a manager also belongs to a department they manage.
- Close/reopen: system admin only.

## Compatibility

The implementation does not change existing attendance, resolved workday, or monthly closure response contracts. It adds workflow APIs and tables only.
