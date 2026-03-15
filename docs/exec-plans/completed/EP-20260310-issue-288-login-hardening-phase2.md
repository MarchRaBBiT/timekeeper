# EP-20260310-issue-288-login-hardening-phase2

## Goal
- issue #288 の phase 2 として、列挙耐性の追加ジッターと lockout 通知の非同期化を入れる

## Scope
- In: `backend/src/handlers/auth.rs`, `backend/src/handlers/admin/subject_requests.rs`, 関連 backend テスト
- Out: Redis 集約、MQ/worker、監査 result 拡張の残り、通知再試行/DLQ

## Done Criteria (Observable)
- [x] unknown username の login failure path に小さいランダムジッターが入る
- [x] lockout 通知送信が認証リクエスト本体から切り離される
- [x] backend の focused test が成功する

## Constraints / Non-goals
- API レスポンス shape は変えない
- 通知の完全な job queue 化はこの phase では行わない

## Task Breakdown
1. [x] unknown user path の jitter helper を追加し、範囲テストを入れる
2. [x] lockout 通知を fire-and-forget 化する
3. [x] focused test / fmt / snapshot / PR 作成まで進める

## Validation Plan
- [x] `cargo test -p timekeeper-backend --lib handlers::auth::tests -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api login_rejects_unknown_username -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api login_locks_account_after_reaching_failure_threshold -- --exact`
- [x] `cargo fmt --all`
- [ ] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`

## JJ Snapshot Log
- [ ] `jj status`
- [x] backend 対象テスト pass
- [ ] `jj commit -m "fix(security): jitter unknown-user login failures and async lockout notifications"`

## Progress Notes
- 2026-03-10: issue #288 の残差分を確認し、phase 2 は unknown-user path の jitter と lockout 通知の request-path 切り離しに絞る方針を決定。
- 2026-03-10: unknown username の login failure path に 15-35ms のランダムジッターを追加し、範囲 unit test を追加。
- 2026-03-10: lockout 通知を `tokio::spawn` で fire-and-forget 化し、認証リクエスト本体が SMTP 送信を待たないよう変更。
- 2026-03-10: `handlers::auth` の focused unit test と login integration test は成功。`cargo clippy -p timekeeper-backend --all-targets -- -D warnings` は `attendance_correction_requests` 系の既存違反で失敗。
