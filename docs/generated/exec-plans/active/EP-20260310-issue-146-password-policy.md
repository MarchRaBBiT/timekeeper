# EP-20260310-issue-146-password-policy

## Goal
- issue #146 の未実装差分として、弱い共通パスワード拒否、パスワード期限事前通知、settings 画面の強度インジケーターを追加する

## Scope
- In: `backend/src/utils/password.rs`, `backend/src/handlers/auth.rs`, `backend/src/models/user.rs`, `backend/tests/password_api.rs`, `frontend/src/api/types.rs`, `frontend/src/pages/settings/panel.rs`
- Out: DB migration 追加、通知メールの新設、ログイン画面の大幅改修

## Done Criteria (Observable)
- [x] 共通の弱いパスワードが backend で拒否される
- [x] `auth/login` / `auth/me` 系の `UserResponse` から期限切れ警告残日数を取得できる
- [x] settings のパスワード変更画面で強度表示と未達条件が見える
- [x] backend / frontend の対象テストが成功する

## Constraints / Non-goals
- 既存の password history / expiration enforcement は再実装しない
- warning window は issue 記載どおり 7 日を固定値で扱う

## Task Breakdown
1. [x] backend に weak-password denylist と expiry warning response を追加
2. [x] settings 画面に strength indicator と expiry warning banner を追加
3. [x] focused test / fmt / clippy を実行し、結果を記録する

## Validation Plan
- [x] `cargo test -p timekeeper-backend --lib utils::password::tests -- --nocapture`
- [x] `cargo test -p timekeeper-backend --lib handlers::auth::tests -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test password_api -- --nocapture`
- [x] `cargo test -p timekeeper-frontend --lib settings -- --nocapture`
- [x] `cargo fmt --all`
- [ ] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`
- [ ] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings`

## JJ Snapshot Log
- [x] `jj status`
- [x] backend / frontend 対象テスト pass
- [ ] `jj commit -m "feat(security): complete password policy gaps"`

## Progress Notes
- 2026-03-10: 計画作成。既存 backend に複雑性・履歴・期限 enforcement が入っていることを確認し、未実装差分の追加に絞る方針を決定。
- 2026-03-10: backend に weak-password denylist と `password_expiry_warning_days` を追加。`auth/me` と login/refresh 系で warning を返すよう変更。
- 2026-03-10: settings の password tab に強度表示、未達条件表示、期限警告バナーを追加。focused backend/frontend tests と `cargo fmt --all` は成功。
- 2026-03-10: `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` は `attendance_correction_requests` 系の既存違反で失敗。`cargo clippy -p timekeeper-frontend --all-targets -- -D warnings` は `layout.rs` unused import など既存 frontend 全体の違反で失敗。
