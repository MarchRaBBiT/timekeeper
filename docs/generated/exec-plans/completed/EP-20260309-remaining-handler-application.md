# EP-20260309-remaining-handler-application

## Goal
- 残っている高優先度 handler ロジックを優先順に application/use-case パターンへ移し、HTTP 層を adapter に寄せる

## Scope
- In: `backend/src/identity/application/*`, `backend/src/attendance/application/*`, `backend/src/handlers/auth.rs`, `backend/src/handlers/attendance_correction_requests.rs`, `backend/src/handlers/sessions.rs`, `backend/src/handlers/consents.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] `auth` handler の主要 orchestration が identity application 層へ移る
- [x] user 側 attendance correction request の create/update/cancel/list/detail ロジックが attendance application 層へ移る
- [x] user sessions と consents handler が application helper 呼び出し中心の adapter 構成になる
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `identity/application/auth.rs` を追加し、login/refresh/logout/profile/MFA/password reset の orchestration を移す
2. [x] `attendance/application/correction_requests.rs` を追加し、user 側 attendance correction request ロジックを移す
3. [x] `identity/application/sessions.rs` と `identity/application/consents.rs` に user sessions / consents の helper を揃える
4. [x] 対応 handler を adapter 構成へ差し替え、既存テストを application 側中心に通す

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test -p timekeeper-backend --test auth_api`
- [x] `cargo test -p timekeeper-backend --test requests_api`
- [x] `cargo test -p timekeeper-backend --lib attendance::application::correction_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib identity::application::consents::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::auth::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [x] remaining handler refactor tests pass
- [x] `jj commit -m "refactor(handler): extract remaining user application helpers"`

## Progress Notes
- 2026-03-09: `auth`、user attendance correction requests、user sessions、consents の順で残存 handler orchestration を application/use-case へ移す計画を作成。
- 2026-03-09: `identity/application/auth.rs` と `identity/application/consents.rs`、`attendance/application/correction_requests.rs` を追加し、`auth` / `consents` / user attendance correction / user sessions の handler を adapter 化。`cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`、`cargo test -p timekeeper-backend --test auth_api`、`cargo test -p timekeeper-backend --test requests_api`、`cargo test -p timekeeper-backend --lib attendance::application::correction_requests::tests::`、`cargo test -p timekeeper-backend --lib identity::application::consents::tests::`、`cargo test -p timekeeper-backend --lib handlers::auth::tests::`、`cargo test -p timekeeper-backend --lib test_app_router_builds` が成功。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `jj commit -m "refactor(handler): extract remaining user application helpers"` を実行し、`9f9a67c8` に保存。

