# EP-20260309-admin-audit-log-application

## Goal
- `handlers/admin/audit_logs.rs` の権限確認・filter 正規化・PII policy・export 組み立てを admin application 層へ移す

## Scope
- In: `backend/src/admin/application/*`, `backend/src/handlers/admin/audit_logs.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] admin audit log handler が application helper 呼び出し中心の adapter 構成になる
- [x] filter 正規化、access control、PII masking、export 用 payload 生成が application 層へ移る
- [x] 既存 API path / method / JSON shape が変わらない

## Task Breakdown
1. [x] `backend/src/admin/application/audit_logs.rs` を追加する
2. [x] `handlers/admin/audit_logs.rs` から orchestration と helper を application 層へ移す
3. [x] ロジック unit test を application 側へ移し、handler 側は DTO/adapter の最小確認に寄せる

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib admin::application::audit_logs::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::audit_logs::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [x] admin audit log refactor tests pass
- [x] `jj commit -m "refactor(admin): extract admin audit log application helpers"`

## Progress Notes
- 2026-03-09: `admin/audit_logs` の責務を access control・filter 正規化・PII masking・export に分け、application 層へ移す作業に着手。
- 2026-03-09: `cargo fmt --all`、`cargo test -p timekeeper-backend --lib admin::application::audit_logs::tests::`、`cargo test -p timekeeper-backend --lib handlers::admin::audit_logs::tests::`、`cargo test -p timekeeper-backend --lib test_app_router_builds` が成功。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `jj commit -m "refactor(admin): extract admin audit log application helpers"` を実行し、`f5f11a61` に保存。

