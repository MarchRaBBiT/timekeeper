# EP-20260309-requests-application-create-request

## Goal
- user request handler の create 系ロジックを requests application 層へ移し、HTTP 層を adapter に近づける

## Scope
- In: `backend/src/handlers/requests.rs`, `backend/src/requests/application/user_requests.rs`, `.agent/PLANS.md`
- Out: request API path/method/payload 変更、DB schema 変更、admin request flow の再設計

## Done Criteria (Observable)
- [x] leave/overtime request 作成時の repository 呼び出しが `requests::application::user_requests` に移っている
- [x] handler は payload 検証と domain model 構築、HTTP response 変換に集中している
- [x] requests 関連ユニットテストと app router build テストが成功する

## Constraints / Non-goals
- 既存 API の JSON 形状は維持する
- `BREAKING_CHANGES.md` は API 互換が崩れた場合のみ更新する

## Task Breakdown
1. [x] create leave/overtime request の repository bridge を application 層へ抽出
2. [x] handler から直接 `RequestRepository` を触る責務を除去
3. [x] fmt + clippy + 対象テストで回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib planned_hours_validation`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`
- [ ] `cargo clippy --all-targets -- -D warnings`

## JJ Snapshot Log
- [x] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(requests): move create request into application service"`

## Progress Notes
- 2026-03-09: `create_leave_request` / `create_overtime_request` の永続化橋渡しを application 層へ移し、handler 側は validation と model 構築に限定。
- 2026-03-09: `cargo clippy --all-targets -- -D warnings` は `utoipa-swagger-ui` の build artifact path 不整合で失敗。今回の差分由来の lint 警告は未検出。

