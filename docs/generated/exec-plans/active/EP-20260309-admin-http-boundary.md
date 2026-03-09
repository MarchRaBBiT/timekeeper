# EP-20260309-admin-http-boundary

## Goal
- 管理画面向けユーザー管理・監査ログ・CSV export の HTTP 入口を `admin` モジュールへ移し、`platform` の route 定義を空に近づける

## Scope
- In: 新規 `backend/src/admin/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: admin handler 本体の再実装、application/use-case 層抽出、API contract 変更

## Done Criteria (Observable)
- [x] `/api/admin/users*`, `/api/admin/archived-users*`, `/api/admin/audit-logs*`, `/api/admin/export` の route 定義が `admin/interface/http.rs` に集約されている
- [x] admin / system-admin の権限制御が従来どおり維持されている
- [x] `platform` の route group 関数が空ルータになり、対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `admin` モジュール追加
2. [x] admin/audit/export 系 route を `admin/interface/http.rs` へ移設
3. [x] `platform::app` から admin ルータを合成するよう変更
4. [x] admin / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib admin::interface::http::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`
- [x] `cargo test -p timekeeper-backend --lib platform_route_groups_are_empty_after_extraction`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(admin): extract admin http routes into module"`

## Progress Notes
- 2026-03-09: `admin/interface/http.rs` を追加し、管理者向けユーザー管理・監査ログ・export route 定義を `platform` から切り離した。既存 API 変更はなし。

