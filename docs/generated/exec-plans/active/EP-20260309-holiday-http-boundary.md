# EP-20260309-holiday-http-boundary

## Goal
- 祝日・週次休日・個別祝日例外の HTTP 入口を `holiday` モジュールへ移し、`platform` はモジュール合成責務へ寄せる

## Scope
- In: 新規 `backend/src/holiday/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: holiday handler 本体の再実装、audit/admin user ルート分離、API contract 変更

## Done Criteria (Observable)
- [x] `/api/holidays*`, `/api/admin/holidays*`, `/api/admin/users/{user_id}/holiday-exceptions*` の route 定義が `holiday/interface/http.rs` に集約されている
- [x] user/admin の holiday 系権限制御が従来どおり維持されている
- [x] holiday / platform の対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `holiday` モジュール追加
2. [x] holiday / holiday-exception 系 route を `holiday/interface/http.rs` へ移設
3. [x] `platform::app` から holiday ルータを合成するよう変更
4. [x] holiday / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib holiday::interface::http::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(holiday): extract holiday http routes into module"`

## Progress Notes
- 2026-03-09: `holiday/interface/http.rs` を追加し、祝日・週次休日・祝日例外の route 定義を `platform` から切り離した。既存 API 変更はなし。

