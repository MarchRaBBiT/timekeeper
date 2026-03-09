# EP-20260309-admin-request-decision-application

## Goal
- admin request の approve/reject と自己承認防止を requests application 層へ移し、admin handler を更新系でも adapter に寄せる

## Scope
- In: `backend/src/handlers/admin/requests.rs`, `backend/src/requests/application/admin_requests.rs`, `.agent/PLANS.md`
- Out: request API path/method/payload 変更、DB schema 変更、監査ミドルウェア仕様変更

## Done Criteria (Observable)
- [x] approve/reject の status update 呼び出しが application 層に移っている
- [x] 自己承認防止と comment validation の業務ルールが application/handler 境界で明確になっている
- [x] 既存 admin request 更新系 API の JSON 形状を維持したまま関連テストが成功する

## Constraints / Non-goals
- admin 権限チェック自体は handler に残す
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests::application::admin_requests` に decision use case と自己承認防止ロジックを追加
2. [x] `handlers::admin::requests` から直接 repository 更新を除去
3. [x] fmt + 関連ユニットテスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::admin_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(requests): move admin request decisions into application service"`

## Progress Notes
- 2026-03-09: approve/reject の repository 更新と自己承認防止を application 層へ移し、handler は auth・body 受け取り・HTTP 変換に限定。

