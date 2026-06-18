# EP-20260613-leave-overtime-request-contract-dtos

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 2 / 7 に沿って、leave / overtime request の API DTO を `crates/contract` に移し、backend/frontend の重複 DTO 定義を contract re-export に寄せる

## Scope
- In: `crates/contract`, `backend/src/models/leave_request.rs`, `backend/src/models/overtime_request.rs`, `backend/src/handlers/requests.rs`, `frontend/src/api/types.rs`, focused contract/backend/frontend tests, this ExecPlan
- Out: approval authorization behavior changes, DB row model changes, SQL repository migration to `crates/infra-postgres`, subject request DTO migration

## Done Criteria (Observable)
- [x] `crates/contract::requests` に leave / overtime create, update, response DTO がある
- [x] contract test が current leave / overtime request wire format を固定している
- [x] backend leave / overtime database models remain backend-owned, while API response conversion returns contract string IDs/status/type fields
- [x] backend create handlers validate the contract payload explicitly before constructing backend database models
- [x] frontend API types use contract re-exports instead of local duplicate structs
- [x] focused contract/backend/frontend request tests and fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- existing JSON field names, date values, status strings, and timestamp strings are unchanged
- backend SQLx row enums and typed IDs remain in backend models for this slice
- admin request approval authorization is not changed in this slice
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] backend/frontend leave and overtime DTO usages を確認する
2. [x] contract request DTO wire-format tests を先に追加し、missing module で red を確認する
3. [x] `crates/contract::requests` に leave / overtime DTOs を追加する
4. [x] backend model conversion and create validation を contract DTO に合わせる
5. [x] frontend request DTOs を contract re-export に置き換える
6. [x] focused validation を実行し、plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-contract --test requests_contract`
- [x] `cargo test -p timekeeper-backend --test leave_request_api`
- [x] `cargo test -p timekeeper-backend --test overtime_request_api`
- [x] `cargo test -p timekeeper-backend --test request_update_api`
- [x] `cargo test -p timekeeper-backend --test admin_requests_api test_admin_can_list_all_requests`
- [x] `cargo test -p timekeeper-backend requests::`
- [x] `cargo test -p timekeeper-frontend requests -- --nocapture --test-threads=1`
- [x] `cargo check -p timekeeper-frontend`
- [x] `cargo fmt --all --check`
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo clippy -p timekeeper-contract -p timekeeper-backend -p timekeeper-frontend --all-targets -- -D warnings`
- [x] `git diff --check`
- [x] `git status --short`

## Git Snapshot Log
- [x] `git status --short`
- [x] focused tests pass
- [ ] commit pending user direction

## Progress Notes
- 2026-06-13: leave / overtime create, update, and response DTOs を `crates/contract::requests` に追加し、existing string IDs/status/type and timestamp string wire format を contract tests で固定した。
- 2026-06-13: backend persistence models remain SQLx-aware, but response conversion now maps typed IDs/enums/timestamps into contract API DTO strings. Create handlers validate date windows, reason length, leave type, and overtime hour range before building backend models.
- 2026-06-13: frontend API types now re-export leave / overtime request DTOs from `timekeeper-contract`.
- 2026-06-13: `cargo test -p timekeeper-backend --test admin_requests_api` still has unrelated manager approval authorization failures (`test_admin_can_approve_leave_request`, `test_admin_can_reject_leave_request`, `test_approve_already_processed_request_fails` return 403 before DTO serialization assertions). The DTO/list seam is covered by `test_admin_can_list_all_requests`.
- 2026-06-13: focused request tests, frontend request tests, frontend check, fmt-check, docs-check, targeted clippy, and diff whitespace check completed successfully.
