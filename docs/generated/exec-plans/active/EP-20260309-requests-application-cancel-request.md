# EP-20260309-requests-application-cancel-request

## Goal
- `requests` の `cancel_request` を application/use-case 化し、handler から cancel 分岐と状態遷移責務を外す

## Scope
- In: `backend/src/requests/application/user_requests.rs`, `backend/src/handlers/requests.rs`, 関連ユニットテスト
- Out: `update_request` のユースケース化、admin request 操作の再設計、API contract 変更

## Done Criteria (Observable)
- [x] `cancel_request` の leave/overtime/attendance-correction 分岐と cancel 実行が `requests/application/user_requests.rs` に移動している
- [x] `handlers/requests.rs` は application 層の結果を JSON 応答へ変換するだけになっている
- [x] 新旧対象テストが成功する

## Constraints / Non-goals
- 既存 API response 形状は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] cancel 用の application 関数と戻り値型を追加
2. [x] request/correction cancel 分岐を handler から application 層へ移設
3. [x] `handlers/requests.rs` を application 呼び出しへ差し替え
4. [x] application / handler / app build テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib planned_hours_validation`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(requests): move cancel request into application service"`

## Progress Notes
- 2026-03-09: `cancel_request` の leave/overtime/attendance-correction cancel 分岐を `requests/application/user_requests.rs` に移し、handler には HTTP 応答だけを残した。既存 API 変更はなし。

