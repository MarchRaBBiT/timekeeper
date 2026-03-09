# EP-20260309-application-common-abstractions

## Goal
- application 層の共通 `error` / DTO / clock 抽象を追加し、既存 handler/application の重複実装を減らす

## Scope
- In: `backend/src/application/*`, `backend/src/identity/application/consents.rs`, `backend/src/requests/application/user_subject_requests.rs`, `backend/src/admin/application/{http_errors,sessions,attendance_correction_requests}.rs`, `backend/src/handlers/{attendance,consents,subject_requests}.rs`, `backend/src/handlers/admin/{requests,subject_requests,sessions,attendance_correction_requests}.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更、認証フローの再設計

## Done Criteria (Observable)
- [x] 共通 `application::clock` / `application::dto` / `application::http` が追加されている
- [x] `consents` と `subject_requests` が共通 HTTP error と DTO を利用する
- [x] `attendance` と request decision 系 handler が共通 clock 抽象を利用する
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `backend/src/application/` を追加し、共通 clock / DTO / HTTP error mapper を実装する
2. [x] `admin::application::http_errors` を共通 mapper への互換層へ置き換える
3. [x] `consents` / `subject_requests` / admin sessions / admin attendance correction に共通 DTO を適用する
4. [x] `attendance` / request decision 系 handler に共通 clock 抽象を適用する

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test -p timekeeper-backend --lib application::clock::tests::`
- [x] `cargo test -p timekeeper-backend --lib application::http::tests::`
- [x] `cargo test -p timekeeper-backend --lib identity::application::consents::tests::`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::sessions::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::attendance_correction_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [x] validation commands pass
- [x] `jj commit -m "refactor(application): add shared error dto and clock abstractions"`

## Progress Notes
- 2026-03-09: `backend/src/application/clock.rs`、[`backend/src/application/dto.rs`](/home/mrabbit/Documents/timekeeper/backend/src/application/dto.rs)、[`backend/src/application/http.rs`](/home/mrabbit/Documents/timekeeper/backend/src/application/http.rs) を追加し、共通 clock / DTO / HTTP error mapper を導入。
- 2026-03-09: `identity/application/consents.rs` と `handlers/consents.rs`、`handlers/subject_requests.rs`、`handlers/admin/subject_requests.rs` を共通 error/clock に寄せ、`requests/application/user_subject_requests.rs`、`admin/application/sessions.rs`、`admin/application/attendance_correction_requests.rs` で共通 DTO を使う形に整理。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `cargo test -p timekeeper-backend --lib handlers::attendance::tests::` と `cargo test -p timekeeper-backend --lib handlers::admin::requests::tests::` を追加で通し、`jj commit -m "refactor(application): add shared error dto and clock abstractions"` を実行する。

