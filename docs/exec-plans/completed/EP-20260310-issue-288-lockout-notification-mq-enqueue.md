# ExecPlan: Issue #288 Lockout Notification MQ Enqueue

## Goal

Issue #288 の未完了項目である「認証APIの通知処理をMQ enqueueへ置換」を実装し、login request path から SMTP 送信を外して Redis queue への job 登録だけを担当させる。

## Done Criteria

- [x] lockout 通知 job の payload / queue helper が追加される
- [x] auth login path は SMTP を直接呼ばず、Redis queue enqueue のみを行う
- [x] enqueue 成功時は `queued`、queue 未設定/失敗時は `failed` の監査結果が残る
- [x] focused integration test で queue 登録と failure path を確認できる
- [x] issue #288 の checklist が更新される
- [ ] 検証成功後に `jj` snapshot を作成する

## Plan

1. lockout notification queue 用 service/payload を追加する
2. auth handler の lockout 通知 path を SMTP から queue enqueue へ置換する
3. Redis integration test を追加して queue payload と failure path を検証する
4. focused test と `cargo fmt --all` を実行する
5. issue #288 の checklist を更新し、`jj commit` を作成する

## Validation

- [x] `cargo test -p timekeeper-backend --test auth_lockout_redis_integration lockout_notification_is_enqueued_in_redis -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api lockout_records_denied_blocked_and_notification_failed_without_queue -- --exact`
- [x] `cargo fmt --all`

## Progress Log

- [x] Context gathered
- [x] Queue service added
- [x] Auth enqueue path implemented
- [x] Focused tests added
- [x] Issue checklist updated
- [ ] Snapshot created
