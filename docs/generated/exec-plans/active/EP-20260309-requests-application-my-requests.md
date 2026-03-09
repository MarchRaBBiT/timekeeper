# EP-20260309-requests-application-my-requests

## Goal
- `requests` の `get_my_requests` を application/use-case 化し、handler から repository 集約とレスポンス組み立て責務を外す

## Scope
- In: 新規 `backend/src/requests/application/*`, `backend/src/handlers/requests.rs`, 関連ユニットテスト
- Out: create/update/cancel request のユースケース化、admin request handler の再設計、API contract 変更

## Done Criteria (Observable)
- [x] `get_my_requests` の leave/overtime/correction 集約と view 組み立てが `requests/application/user_requests.rs` に移動している
- [x] `handlers/requests.rs` は application 層の結果を JSON 応答へ変換するだけになっている
- [x] 新旧対象テストが成功する

## Constraints / Non-goals
- 既存 API response 形状は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests/application/user_requests.rs` を追加
2. [x] `get_my_requests` の集約・整形を application 層へ移設
3. [x] `handlers/requests.rs` を application 呼び出しへ差し替え
4. [x] application / app build テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib get_my_requests`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(requests): add my-requests application service"`

## Progress Notes
- 2026-03-09: `requests/application/user_requests.rs` を追加し、`get_my_requests` の request/correction 集約とレスポンス組み立てを handler から移設した。既存 API 変更はなし。

