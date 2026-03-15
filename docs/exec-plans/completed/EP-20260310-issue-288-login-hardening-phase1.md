# EP-20260310-issue-288-login-hardening-phase1

## Goal
- issue #288 のうち、即時に適用できる login hardening を先行実装する

## Scope
- In: `backend/src/handlers/auth.rs`, `backend/src/repositories/auth.rs`, `backend/tests/auth_flow_api.rs`
- Out: Redis 集約、MQ/worker 追加、通知ジョブ化、ランダムジッター導入

## Done Criteria (Observable)
- [x] 成功ログイン後も `lockout_count` が保持される
- [x] 監査ログ result が `login_failure => denied`, `account_lockout => blocked` になる
- [x] backend の対象テストが成功する

## Constraints / Non-goals
- issue #288 を一度に閉じ切らず、独立して安全に出せる改善だけを先行する
- 既存 API shape やログイン成功/失敗の外向きレスポンスは変えない

## Task Breakdown
1. [x] `clear_login_failures` の reset 対象から `lockout_count` を外す
2. [x] locked account / threshold 到達時の audit result を `blocked` / `denied` に修正する
3. [x] focused test / fmt / jj snapshot / PR 作成まで行う

## Validation Plan
- [x] `cargo test -p timekeeper-backend --lib handlers::auth::tests -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api login_locks_account_after_reaching_failure_threshold -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api login_success_preserves_lockout_history_while_clearing_active_failures -- --exact`
- [x] `cargo fmt --all`
- [ ] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`

## JJ Snapshot Log
- [ ] `jj status`
- [x] backend 対象テスト pass
- [ ] `jj commit -m "fix(security): preserve lockout history and classify login audit results"`

## Progress Notes
- 2026-03-10: issue #288 の current main との差分を確認。ダミーハッシュ検証と通知 async spawn は既に入っているため、未解消の実害である `lockout_count` reset と audit result を phase 1 として切り出す方針に決定。
- 2026-03-10: `clear_login_failures` から `lockout_count` reset を除去し、成功ログイン後もバックオフ履歴を保持するよう変更。
- 2026-03-10: locked account の監査イベントを `account_lockout` に寄せ、`login_failure => denied`, `account_lockout => blocked` へ result を修正。
- 2026-03-10: `handlers::auth` unit test と `auth_flow_api` の focused integration test は成功。`cargo clippy -p timekeeper-backend --all-targets -- -D warnings` は `attendance_correction_requests` 系の既存違反で失敗。
