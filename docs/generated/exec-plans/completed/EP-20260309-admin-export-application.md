# EP-20260309-admin-export-application

## Goal
- `handlers/admin/export.rs` の query 正規化・DB 取得・PII masking・CSV 組み立てを admin application 層へ移す

## Scope
- In: `backend/src/admin/application/*`, `backend/src/handlers/admin/export.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] admin export handler が application helper 呼び出し中心の adapter 構成になる
- [x] export query の date 検証、attendance export SQL、CSV 組み立て、PII masking が application 層へ移る
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `backend/src/admin/application/export.rs` を追加する
2. [x] `handlers/admin/export.rs` から orchestration と helper を application 層へ移す
3. [x] ロジック unit test を application 側へ移し、handler 側は DTO/adapter の最小確認に寄せる

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib admin::application::export::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::export::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [x] admin export refactor tests pass
- [x] `jj commit -m "refactor(admin): extract admin export application helpers"`

## Progress Notes
- 2026-03-09: `admin/export` の責務を query 正規化・attendance export 取得・CSV 組み立てに分け、application 層へ移す作業に着手。
- 2026-03-09: `cargo fmt --all`、`cargo test -p timekeeper-backend --lib admin::application::export::tests::`、`cargo test -p timekeeper-backend --lib handlers::admin::export::tests::`、`cargo test -p timekeeper-backend --lib test_app_router_builds` が成功。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `jj commit -m "refactor(admin): extract admin export application helpers"` を実行し、`be9259e7` に保存。

