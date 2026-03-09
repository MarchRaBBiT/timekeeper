# EP-20260309-admin-holiday-application

## Goal
- `handlers/admin/holidays.rs` の query/command ロジックを holiday application 層へ移し、handler を HTTP adapter に薄くする

## Scope
- In: `backend/src/holiday/application/*`, `backend/src/handlers/admin/holidays.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] admin holiday list/create/delete と weekly holiday list/create/delete が application helper を経由する
- [x] query 正規化と weekly holiday バリデーションが application 層へ移る
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `backend/src/holiday/application/admin.rs` を追加する
2. [x] `handlers/admin/holidays.rs` から orchestration を application 層へ移す
3. [x] query/validation unit test を application 側へ移し、handler 側は DTO の最小確認に寄せる

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib holiday::application::admin::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::holidays::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [x] admin holiday refactor tests pass
- [x] `jj commit -m "refactor(holiday): extract admin holiday application helpers"`

## Progress Notes
- 2026-03-09: holiday exception 抽出の次段として、admin holiday handler の adapter 化に着手。
- 2026-03-09: `cargo fmt --all`、`cargo test -p timekeeper-backend --lib holiday::application::admin::tests::`、`cargo test -p timekeeper-backend --lib handlers::admin::holidays::tests::`、`cargo test -p timekeeper-backend --lib test_app_router_builds` が成功。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `jj commit -m "refactor(holiday): extract admin holiday application helpers"` を実行し、`649ff3e4` に保存。

