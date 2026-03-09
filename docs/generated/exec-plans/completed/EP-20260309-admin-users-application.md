# EP-20260309-admin-users-application

## Goal
- `handlers/admin/users.rs` を分解し、一覧・作成更新・MFA/unlock・アーカイブ操作の orchestration を admin application 層へ移す

## Scope
- In: `backend/src/admin/application/*`, `backend/src/admin/mod.rs`, `backend/src/handlers/admin/users.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] `admin/users` handler が application helper 呼び出し中心の adapter 構成になる
- [x] PII 復号/マスク、作成更新、MFA/unlock 監査、アーカイブ復元削除のロジックが application 層へ移る
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `backend/src/admin/application/users.rs` を追加する
2. [x] `handlers/admin/users.rs` から orchestration と共通 helper を application 層へ移す
3. [x] ロジック unit test を application 側へ移し、handler 側は DTO/adapter の最小確認に寄せる

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib admin::application::users::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::users::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [x] admin users refactor tests pass
- [x] `jj commit -m "refactor(admin): extract admin user application helpers"`

## Progress Notes
- 2026-03-09: `admin/users` の責務を一覧・作成更新・MFA/unlock・アーカイブ操作に分け、application 層へ移す作業に着手。
- 2026-03-09: `cargo fmt --all`、`cargo test -p timekeeper-backend --lib admin::application::users::tests::`、`cargo test -p timekeeper-backend --lib handlers::admin::users::tests::`、`cargo test -p timekeeper-backend --lib test_app_router_builds` が成功。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `jj commit -m "refactor(admin): extract admin user application helpers"` を実行し、`0cd5c807` に保存。

