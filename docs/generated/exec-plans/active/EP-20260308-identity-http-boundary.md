# EP-20260308-identity-http-boundary

## Goal
- 認証・セッション関連の HTTP 入口を `identity` モジュールへ移し、`platform` は業務モジュール合成に集中させる

## Scope
- In: 新規 `backend/src/identity/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: auth/sessions handler 本体のリライト、API contract 変更、DB 変更

## Done Criteria (Observable)
- [x] `/api/auth/*` と関連 public config route のルーティング定義が `identity/interface/http.rs` に集約されている
- [x] `platform::app::build_app` が identity ルータを合成している
- [x] identity と platform の対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `identity` モジュール追加
2. [x] auth/sessions/config の route 定義を `identity/interface/http.rs` へ移設
3. [x] `platform::app` から identity ルータを合成するよう変更
4. [x] identity / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib user_identity_routes_require_auth`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`
- [x] `cargo test -p timekeeper-backend --lib test_domain_route_groups_require_auth`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(identity): extract auth http routes into identity module"`

## Progress Notes
- 2026-03-08: `identity/interface/http.rs` を追加し、認証・セッション系ルートを `platform` から切り離した。既存 API 変更はなし。

