# EP-20260309-remaining-handler-followups

## Goal
- top-level handler に残っている優先度高の orchestration を application/use-case パターンへ寄せる

## Scope
- In: `backend/src/identity/application/*`, `backend/src/attendance/application/*`, `backend/src/handlers/auth.rs`, `backend/src/handlers/attendance_correction_requests.rs`, `backend/src/handlers/sessions.rs`, `backend/src/handlers/consents.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] `auth` handler が login/refresh/logout/MFA/password 系で application helper 呼び出し中心の adapter 構成になる
- [x] user 側 `attendance_correction_requests`、`sessions`、`consents` の orchestration が application 層へ移る
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `identity/application/auth.rs` を追加し、auth orchestration を移す
2. [x] `attendance/application/correction_requests.rs` を追加し、user correction request の create/update/cancel を移す
3. [x] `identity/application/sessions.rs` と `identity/application/consents.rs` に残存 helper を移す
4. [x] handler を adapter 化し、関連テストを更新する

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test -p timekeeper-backend --lib handlers::auth::tests::build_login_response_sets_auth_cookies -- --exact`
- [x] `cargo test -p timekeeper-backend --lib attendance::application::correction_requests::tests::validate_reason_rejects_empty_and_long_values -- --exact`
- [x] `cargo test -p timekeeper-backend --lib identity::application::consents::tests::validate_string_field_rejects_empty_and_long_values -- --exact`
- [x] `cargo test -p timekeeper-backend --lib identity::application::sessions::tests::extract_ip_prefers_x_forwarded_for -- --exact`
- [x] `cargo test -p timekeeper-backend --lib platform::app::tests::test_app_router_builds -- --exact`

## JJ Snapshot Log
- [x] `jj status`
- [x] remaining handler refactor tests pass
- [x] `jj commit -m "refactor(handler): move remaining top-level orchestration into application"`

## Progress Notes
- 2026-03-09: `auth` を最優先に、続いて user 側 `attendance_correction_requests`、`sessions`、`consents` の application 化に着手。
- 2026-03-09: `identity/application/auth.rs`、`identity/application/consents.rs`、`attendance/application/correction_requests.rs` を追加し、top-level handler の orchestration を application 層へ移行。互換 API は維持し、`BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`、代表ユニットテスト 5 本が成功。
- 2026-03-09: `jj commit -m "refactor(handler): move remaining top-level orchestration into application"` を実行し、`a5584e14` に保存。working copy はクリーン。

