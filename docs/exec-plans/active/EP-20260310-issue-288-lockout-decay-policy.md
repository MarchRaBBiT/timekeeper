# ExecPlan: Issue #288 Lockout Decay Policy

## Goal

Issue #288 の未完了項目である `lockout_count` の保持/減衰ポリシーを実装し、長時間静穏だったアカウントのバックオフ強度が段階的に緩和されるようにする。

## Done Criteria

- [x] `users` に最後の失敗時刻を保持するカラムが追加される
- [x] lockout 判定時に静穏期間に応じて `lockout_count` が段階的に減衰する
- [x] 成功ログイン時は履歴を保持しつつ、長時間静穏後の再攻撃では減衰済みの backoff が使われる
- [x] focused test で lockout_count の減衰を確認できる
- [x] issue #288 の checklist が更新される
- [ ] 検証成功後に `jj` snapshot を作成する

## Plan

1. `last_login_failure_at` を追加する migration を作成する
2. auth repository の failure/lockout 更新ロジックに減衰計算を組み込む
3. auth integration test に静穏期間後の再攻撃シナリオを追加する
4. focused test と `cargo fmt --all` を実行する
5. issue #288 の checklist を更新し、`jj commit` を作成する

## Validation

- [x] `cargo test -p timekeeper-backend --test auth_flow_api login_success_preserves_lockout_history_while_clearing_active_failures -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api lockout_count_decays_after_extended_quiet_period -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_lockout_redis_integration redis_lockout_uses_decayed_history_after_quiet_period -- --exact`
- [x] `cargo fmt --all`
- [ ] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` (既存の `attendance_correction_requests` 系違反で失敗)

## Progress Log

- [x] Context gathered
- [x] Migration added
- [x] Repository decay logic implemented
- [x] Focused tests added
- [x] Issue checklist updated
- [ ] Snapshot created
