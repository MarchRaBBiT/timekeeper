# EP-20260309-admin-attendance-application

## Goal
- `handlers/admin/attendance.rs` の一覧・upsert・強制休憩終了ロジックを admin application 層へ移す

## Scope
- In: `backend/src/admin/application/*`, `backend/src/handlers/admin/attendance.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] admin attendance handler が application helper 呼び出し中心の adapter 構成になる
- [x] 一覧の pagination 集約、upsert の parse/transaction、強制休憩終了と再計算が application 層へ移る
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `backend/src/admin/application/attendance.rs` を追加する
2. [x] `handlers/admin/attendance.rs` から orchestration と helper を application 層へ移す
3. [x] ロジック unit test を application 側へ移し、handler 側は DTO/adapter の最小確認に寄せる

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib admin::application::attendance::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::attendance::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [x] admin attendance refactor tests pass
- [x] `jj commit -m "refactor(admin): extract admin attendance application helpers"`

## Progress Notes
- 2026-03-09: `admin/attendance` の責務を一覧・upsert・強制休憩終了に分け、application 層へ移す作業に着手。
- 2026-03-09: `cargo fmt --all`、`cargo test -p timekeeper-backend --lib admin::application::attendance::tests::`、`cargo test -p timekeeper-backend --lib handlers::admin::attendance::tests::`、`cargo test -p timekeeper-backend --lib test_app_router_builds` が成功。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `jj commit -m "refactor(admin): extract admin attendance application helpers"` を実行し、`dbc7439c` に保存。

