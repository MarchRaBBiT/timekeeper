# EP-20260309-identity-application-sessions

## Goal
- `identity` モジュールに最初の application/use-case を導入し、session 一覧取得の整形責務を handler から外す

## Scope
- In: 新規 `backend/src/identity/application/*`, `backend/src/handlers/sessions.rs`, 関連ユニットテスト
- Out: session revoke のユースケース化、auth handler 全体の再設計、API contract 変更

## Done Criteria (Observable)
- [x] `list sessions` の repository 呼び出しと DTO 変換が `identity/application/sessions.rs` に移動している
- [x] `handlers/sessions.rs` は application 層の結果を HTTP レスポンスへ変換するだけになっている
- [x] 新旧対象テストが成功する

## Constraints / Non-goals
- 既存 API response 形状は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `identity/application/sessions.rs` を追加
2. [x] `list_sessions` の session view 生成を application 層へ移設
3. [x] `handlers/sessions.rs` を application 呼び出しへ差し替え
4. [x] application / handler / app build テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib identity::application::sessions::tests::`
- [x] `cargo test -p timekeeper-backend --lib extract_ip_`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(identity): add sessions application service"`

## Progress Notes
- 2026-03-09: `identity/application/sessions.rs` を追加し、session 一覧取得の整形責務を handler から移設した。既存 API 変更はなし。

