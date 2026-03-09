# EP-20260309-handler-remaining-followups

## Goal
- 残っている高優先度の user-side handler ロジックを application/use-case 層へ寄せ、HTTP 層を adapter に薄くする

## Scope
- In: `backend/src/identity/application/*`, `backend/src/attendance/application/*`, `backend/src/handlers/auth.rs`, `backend/src/handlers/attendance_correction_requests.rs`, `backend/src/handlers/sessions.rs`, `backend/src/handlers/consents.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] `auth` handler の login/refresh/logout/MFA/password 系 orchestration が identity application 層へ移る
- [x] user-side attendance correction request の create/update/cancel/snapshot validation が attendance application 層へ移る
- [x] user-side sessions / consents handler が application helper 呼び出し中心の adapter 構成になる
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `backend/src/identity/application/auth.rs` を追加し、auth orchestration と helper を移す
2. [x] `backend/src/attendance/application/correction_requests.rs` を追加し、attendance correction request orchestration を移す
3. [x] `backend/src/identity/application/sessions.rs` と新規 `consents.rs` へ user-side session/consent logic を整理する
4. [x] 対象 handler を adapter 構成へ差し替え、関連 unit test を通す

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test -p timekeeper-backend --lib identity::application::auth::`
- [x] `cargo test -p timekeeper-backend --lib attendance::application::correction_requests::`
- [x] `cargo test -p timekeeper-backend --lib identity::application::consents::`
- [x] `cargo test -p timekeeper-backend --lib identity::application::sessions::`
- [x] `cargo test -p timekeeper-backend --lib handlers::attendance_correction_requests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::auth::`
- [x] `cargo test -p timekeeper-backend --lib handlers::sessions::`
- [x] `cargo test -p timekeeper-backend --lib handlers::consents::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`
- [x] `cargo test -p timekeeper-backend --test auth_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test requests_api -- --nocapture`

## JJ Snapshot Log
- [x] `jj status`
- [x] remaining handler follow-up tests pass
- [ ] `jj commit -m "refactor(identity): move remaining user handler orchestration into application"`

## Progress Notes
- 2026-03-09: `auth`、user-side `attendance_correction_requests`、`sessions`、`consents` を優先順に application/use-case 化する計画を作成。
- 2026-03-09: `identity/application/auth.rs` と `consents.rs`、`attendance/application/correction_requests.rs` を追加し、対応 handler を adapter 化。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`、対象 unit test 群、`auth_api`、`requests_api` が成功。

