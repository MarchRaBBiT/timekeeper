# EP-20260309-admin-request-read-application

## Goal
- admin request の read 系ユースケースを requests application 層へ移し、admin handler を権限確認と HTTP adapter に寄せる

## Scope
- In: `backend/src/handlers/admin/requests.rs`, `backend/src/requests/application/admin_requests.rs`, `backend/src/requests/application/mod.rs`, `.agent/PLANS.md`
- Out: approve/reject 更新系の再設計、API path/method/payload 変更、DB schema 変更

## Done Criteria (Observable)
- [x] admin request 一覧取得の filter 解釈と response 組み立てが application 層に移っている
- [x] admin request 詳細取得の kind/data 判定が application 層に移っている
- [x] 既存 admin request API の JSON 形状を維持したまま関連ユニットテストが成功する

## Constraints / Non-goals
- OpenAPI で参照している request/query 型は handler 側に残す
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests::application::admin_requests` を追加し、一覧/詳細の query 解釈と response 変換を移設
2. [x] `handlers::admin::requests` から read 系 repository 呼び出しを除去
3. [x] fmt + 対象テスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::admin_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(requests): extract admin request read application service"`

## Progress Notes
- 2026-03-09: admin request の一覧/詳細取得と filter/page 解釈を `requests::application::admin_requests` へ移動開始。handler 側は auth と query 受け渡しに集中させる。

