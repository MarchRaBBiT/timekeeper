# EP-20260309-user-subject-request-application

## Goal
- user subject request の create/list/cancel を application 層へ移し、handler を HTTP adapter に寄せる

## Scope
- In: `backend/src/handlers/subject_requests.rs`, `backend/src/requests/application/user_subject_requests.rs`, `backend/src/requests/application/mod.rs`, `.agent/PLANS.md`
- Out: subject request API path/method/payload 変更、DB schema 変更、admin subject request flow 変更

## Done Criteria (Observable)
- [x] create/list/cancel の repository 呼び出しが application 層に移っている
- [x] details validation が application 層に移っている
- [x] 既存 user subject request API の JSON 形状を維持したまま関連テストが成功する

## Constraints / Non-goals
- HTTP エラーマッピングは handler に残す
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests::application::user_subject_requests` を追加し、create/list/cancel と details validation を移設
2. [x] `handlers::subject_requests` から直接 repository 呼び出しを除去
3. [x] fmt + 関連ユニットテスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::subject_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(requests): extract user subject request application service"`

## Progress Notes
- 2026-03-09: user subject request の create/list/cancel と details validation を application 層へ移し、handler 側は HTTP エラーマッピングと DTO 受け渡しに限定。

