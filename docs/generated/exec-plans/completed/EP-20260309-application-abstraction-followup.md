# EP-20260309-application-abstraction-followup

## Goal
- 共通 application 抽象の適用漏れを解消し、主要 handler から `Json<Value>` と `utils::time` 直接依存を除去する

## Scope
- In: `backend/src/handlers/{auth,attendance,requests,sessions}.rs`, `backend/src/handlers/admin/{requests,subject_requests,users,holidays,export,audit_logs}.rs`, `backend/src/identity/application/{auth,sessions}.rs`, `backend/src/requests/application/{user_requests,admin_requests,admin_subject_requests}.rs`, `backend/src/holiday/application/admin.rs`, `backend/src/admin/application/{attendance,users}.rs`, `backend/src/application/dto.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更、OpenAPI の大規模再設計

## Done Criteria (Observable)
- [x] `auth` / `requests` / `sessions` の action response が typed DTO で返る
- [x] admin requests / subject_requests / users / holidays の action response が typed DTO で返る
- [x] `attendance` export と admin export / audit_logs / attendance / holiday validation が共通 clock を使う
- [x] `backend/src/handlers` / `backend/src/*/application` の production code に `Json<Value>` と `utils::time` 直接依存が残らない

## Task Breakdown
1. [x] `auth` の request/response を typed DTO に寄せる
2. [x] user/admin request・session・holiday・user action response を共通 DTO または typed struct へ寄せる
3. [x] admin/export・audit_logs・attendance・holiday validation の time 依存を `SYSTEM_CLOCK` に置き換える
4. [x] fmt / clippy / 影響テストを通し、残差分を確認する

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test -p timekeeper-backend --lib handlers::attendance::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::auth::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::sessions::`
- [x] `cargo test -p timekeeper-backend --lib identity::application::sessions::`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib requests::application::admin_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib requests::application::admin_subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::users::tests::`
- [x] `cargo test -p timekeeper-backend --lib admin::application::users::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::holidays::tests::`
- [x] `cargo test -p timekeeper-backend --lib holiday::application::admin::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::audit_logs::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::export::tests::`
- [x] `cargo test -p timekeeper-backend --lib admin::application::attendance::tests::`
- [x] `cargo test -p timekeeper-backend --lib platform::app::tests::test_app_router_builds -- --exact`
- [x] `cargo test -p timekeeper-backend --test auth_api`

## JJ Snapshot Log
- [x] `jj status`
- [x] validation commands pass
- [x] `jj commit -m "refactor(application): finish typed response and clock cleanup"`

## Progress Notes
- 2026-03-09: `auth` に `RefreshPayload` / `LogoutPayload` を追加し、MFA / password / logout 系レスポンスを `MessageResponse` に統一。API 形状は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `requests` / `sessions` / admin requests / admin subject requests / admin users / admin holidays を typed DTO に寄せ、`attendance` export と admin export / audit_logs / attendance / holiday validation を `SYSTEM_CLOCK` に切り替えた。
- 2026-03-09: `rg -n "Json<Value>|time::now_in_timezone|time::now_utc|time::today_local" src/handlers src/*/application -g '!**/tests/**'` は production code では一致なしとなった。
- 2026-03-09: `jj commit -m "refactor(application): finish typed response and clock cleanup"` を実行し、今回の適用漏れ整理を保存した。
- 2026-03-09: `application::http::forbidden_error` を追加し、handler / application に残っていた直接 `AppError::Forbidden("Forbidden".into())` を共通化した。
- 2026-03-09: `handlers/subject_requests.rs` と `handlers/admin/subject_requests.rs` を `crate::application::http::map_app_error` へ切り替え、互換層 `admin/application/http_errors.rs` を削除した。
- 2026-03-09: `cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`、`cargo test -p timekeeper-backend --lib application::http::tests::`、`cargo test -p timekeeper-backend --lib handlers::admin::requests::tests::`、`cargo test -p timekeeper-backend --lib handlers::holidays::tests::`、`cargo test -p timekeeper-backend --lib handlers::subject_requests::tests::`、`cargo test -p timekeeper-backend --lib handlers::admin::subject_requests::tests::`、`cargo test -p timekeeper-backend --lib platform::app::tests::test_app_router_builds -- --exact` を通し、`rg -n 'AppError::Forbidden\\(\"Forbidden\"\\.into\\(\\)\\)|admin::application::http_errors|pub mod http_errors' backend/src` は一致なしを確認した。
