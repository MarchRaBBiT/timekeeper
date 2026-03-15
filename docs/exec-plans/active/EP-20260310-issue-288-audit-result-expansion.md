# ExecPlan: Issue #288 Audit Result Expansion

## Goal

Issue #288 の checklist を現状に合わせて更新し、未完了項目のうち auth/login 周りの監査ログ `result` 拡張に着手する。

## Done Criteria

- [x] issue #288 の checklist が実装済み項目を反映している
- [x] lockout 通知の監査ログが `queued` / `success` / `failed` を区別して記録される
- [x] login failure 系監査ログで `denied` / `blocked` / `queued` / `failed` の区別が確認できる
- [x] auth の focused integration test が成功する
- [ ] 検証成功後に `jj` snapshot を作成する

## Plan

1. issue #288 の実装済み項目を整理し、更新する checklist を確定する
2. `backend/src/handlers/auth.rs` で lockout 通知の audit event を拡張する
3. `backend/tests/auth_flow_api.rs` に success/failure の audit trail test を追加する
4. focused test と `cargo fmt --all` を実行する
5. issue #288 の checklist を更新し、`jj commit` を作成する

## Validation

- [x] `cargo test -p timekeeper-backend --test auth_flow_api login_locks_account_after_reaching_failure_threshold -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api lockout_records_denied_blocked_and_notification_success_audit_results -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api lockout_notification_failure_records_failed_audit_result -- --exact`
- [x] `cargo fmt --all`

## Progress Log

- [x] Context gathered
- [x] Auth audit result implementation updated
- [x] Integration tests added
- [x] Issue checklist updated
- [ ] Snapshot created
