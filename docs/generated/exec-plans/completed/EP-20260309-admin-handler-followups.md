# EP-20260309-admin-handler-followups

## Goal
- admin handler の残存ロジックを優先度順に application/use-case パターンへ寄せる

## Scope
- In: `backend/src/admin/application/*`, `backend/src/handlers/admin/*`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] `attendance_correction_requests` の承認/却下 orchestration が application 層へ移る
- [x] `sessions` の list/revoke と audit helper が application 層へ移る
- [x] `subject_requests` と `requests` に残っていた handler helper が application/common へ整理される
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `admin/application/attendance_correction_requests.rs` と `admin/application/sessions.rs` を追加する
2. [x] 対応する handler を adapter 構成へ差し替える
3. [x] `admin/application/http_errors.rs` を追加し、subject request error mapping を移す
4. [x] `handlers/admin/requests.rs` に残る wrapper/helper を整理する

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib admin::application::attendance_correction_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib admin::application::sessions::tests::`
- [x] `cargo test -p timekeeper-backend --lib admin::application::http_errors::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::attendance_correction_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::sessions::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [x] admin follow-up refactor tests pass
- [x] `jj commit -m "refactor(admin): move remaining handler orchestration into application"`

## Progress Notes
- 2026-03-09: 優先度高の `attendance_correction_requests` と `sessions` を先に application 化し、その後 `subject_requests` / `requests` に残っていた handler helper を整理。
- 2026-03-09: 上記 9 本の検証コマンドが成功。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `jj commit -m "refactor(admin): move remaining handler orchestration into application"` を実行し、`b536e506` に保存。

