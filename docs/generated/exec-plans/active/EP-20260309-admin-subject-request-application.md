# EP-20260309-admin-subject-request-application

## Goal
- admin subject request の一覧/承認却下ロジックを application 層へ移し、handler を HTTP adapter に寄せる

## Scope
- In: `backend/src/handlers/admin/subject_requests.rs`, `backend/src/requests/application/admin_subject_requests.rs`, `backend/src/requests/application/mod.rs`, `.agent/PLANS.md`
- Out: subject request API path/method/payload 変更、DB schema 変更、user subject request handler の再設計

## Done Criteria (Observable)
- [x] subject request 一覧の filter 解釈と response 組み立てが application 層に移っている
- [x] approve/reject と pending 判定が application 層に移っている
- [x] 既存 admin subject request API の JSON 形状を維持したまま関連テストが成功する

## Constraints / Non-goals
- admin 権限チェックと `AppError -> HTTP` 変換は handler に残す
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests::application::admin_subject_requests` を追加し、一覧/decision/pending 判定を移設
2. [x] `handlers::admin::subject_requests` から直接 repository 呼び出しと query 解釈を除去
3. [x] fmt + 関連ユニットテスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::admin_subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(requests): extract admin subject request application service"`

## Progress Notes
- 2026-03-09: admin subject request の query 解釈・一覧整形・pending 判定・承認却下を application 層へ移動開始。handler 側は auth と HTTP エラー変換に限定。

