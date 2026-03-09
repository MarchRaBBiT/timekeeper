# EP-20260308-requests-http-boundary

## Goal
- 申請・同意・本人開示請求の HTTP 入口を `requests` モジュールへ移し、`platform` はモジュール合成責務へ寄せる

## Scope
- In: 新規 `backend/src/requests/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: request handler 本体の再実装、audit/holiday ルート分離、API contract 変更

## Done Criteria (Observable)
- [x] `/api/requests*`, `/api/consents*`, `/api/subject-requests*`, `/api/admin/requests*`, `/api/admin/subject-requests*` の route 定義が `requests/interface/http.rs` に集約されている
- [x] user/admin の申請系権限制御が従来どおり維持されている
- [x] requests / platform の対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests` モジュール追加
2. [x] request/consent/subject-request 系 route を `requests/interface/http.rs` へ移設
3. [x] `platform::app` から requests ルータを合成するよう変更
4. [x] requests / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::interface::http::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`
- [x] `cargo test -p timekeeper-backend --lib test_domain_route_groups_require_auth`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(requests): extract request http routes into module"`

## Progress Notes
- 2026-03-08: `requests/interface/http.rs` を追加し、申請・同意・本人開示請求系 route 定義を `platform` から切り離した。既存 API 変更はなし。

