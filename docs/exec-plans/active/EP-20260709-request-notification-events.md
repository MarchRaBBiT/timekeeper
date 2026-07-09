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

