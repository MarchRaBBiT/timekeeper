# EP-20260310-issue-288-login-hardening-phase3-redis

## Goal
- issue #288 の phase 3 として、ログイン失敗回数の Redis 集約を入れて DB 書き込みを閾値到達時に限定する

## Scope
- In: `backend/src/handlers/auth.rs`, `backend/src/repositories/auth.rs`, Redis 連携 integration test
- Out: MQ/worker 導入、監査 result 追加拡張の残り、lockout history 減衰ポリシー

## Done Criteria (Observable)
- [x] Redis 有効時、閾値未満の login failure では `users.failed_login_attempts` が増えない
- [x] Redis 有効時、閾値到達時のみ DB に lockout 状態が永続化される
- [x] 成功ログイン時に Redis 側の失敗カウントが best-effort で消去される
- [x] Redis 障害時に既存の DB-only path へフォールバックする
- [x] focused backend test が成功する

## Constraints / Non-goals
- API レスポンス shape は変えない
- Redis は pre-threshold の失敗回数だけを保持し、lockout state の source of truth は DB のままにする

## Task Breakdown
1. [x] `repositories/auth.rs` の lockout 永続化 helper を確定する
2. [x] `handlers/auth.rs` に Redis failure counter と DB fallback を追加する
3. [x] Redis integration test で threshold 未満/到達/成功時 clear を固定する
4. [ ] focused test / fmt / snapshot / PR 作成まで進める

## Validation Plan
- [x] `cargo test -p timekeeper-backend --lib handlers::auth::tests -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api login_locks_account_after_reaching_failure_threshold -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_lockout_redis_integration -- --nocapture`
- [x] `cargo fmt --all`
- [ ] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`

## JJ Snapshot Log
- [x] `jj status`
- [ ] backend 対象テスト pass
- [ ] `jj commit -m "feat(security): aggregate login failures in redis"`

## Progress Notes
- 2026-03-10: phase 2 branch を親にして Redis 集約の実装に着手。`persist_lockout_state()` の下地がローカル差分に残っていることを確認。
- 2026-03-10: `handlers/auth.rs` に Redis Lua script ベースの failure counter を追加。Redis 障害時は warn を出して既存 DB-only path にフォールバックする構成にした。
- 2026-03-10: `auth_lockout_redis_integration` を追加し、threshold 未満は DB 非更新、threshold 到達時のみ DB lockout、成功ログイン時の Redis key clear を固定した。
- 2026-03-10: `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` は `attendance_correction_requests` 系の既存違反で失敗。
