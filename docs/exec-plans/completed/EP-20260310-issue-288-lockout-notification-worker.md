# ExecPlan: Issue #288 Lockout Notification Worker

## Goal

Issue #288 の未完了項目だった lockout notification worker を追加し、Redis queue から job を処理して再試行・DLQ・冪等を備えた非同期送信経路を用意する。

## Done Criteria

- [x] worker service と binary が追加される
- [x] queue job の retry / DLQ / idempotency marker が実装される
- [x] focused integration test で sent / DLQ / no-queue failure が確認できる
- [x] issue #288 の checklist が更新される
- [ ] 検証成功後に `jj` snapshot を作成する

## Validation

- [x] `cargo test -p timekeeper-backend --test auth_lockout_redis_integration worker_sends_enqueued_lockout_notification_job -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_lockout_redis_integration worker_moves_exhausted_notification_to_dlq -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api lockout_records_denied_blocked_and_notification_failed_without_queue -- --exact`
- [x] `cargo fmt --all`

## Progress Log

- [x] Queue/worker context gathered
- [x] Worker service and binary added
- [x] Retry / DLQ / idempotency added
- [x] Focused tests added
- [x] Issue checklist updated
- [ ] Snapshot created
