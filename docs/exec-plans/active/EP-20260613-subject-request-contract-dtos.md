# EP-20260613-subject-request-contract-dtos

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 2 / 7 に沿って、data subject request の API DTO を `crates/contract` に移し、backend/frontend の重複 DTO 定義を contract re-export に寄せる

## Scope
- In: `crates/contract`, `backend/src/models/subject_request.rs`, `backend/src/handlers/subject_requests.rs`, `backend/src/handlers/admin/subject_requests.rs`, `backend/tests/subject_requests_api.rs`, `backend/tests/admin_subject_requests_api.rs`, `frontend/src/api/types.rs`, focused contract/backend/frontend tests, this ExecPlan
- Out: SQL repository migration to `crates/infra-postgres`, data subject request business policy changes, non-subject-request DTO migration

## Done Criteria (Observable)
- [x] `crates/contract::subject_requests` に create, response, list, decision DTO と API enum がある
- [x] contract test が current subject request wire format を固定している
- [x] backend SQLx-aware subject request model/enum remains backend-owned, with explicit conversion to/from contract API DTOs
- [x] backend user and admin handlers use contract DTOs at the HTTP boundary
- [x] frontend API types use contract re-exports instead of local duplicate structs
- [x] focused contract/backend/frontend tests and fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- existing JSON field names, snake_case request types, status strings, and timestamp values are unchanged
- backend persistence enum retains SQLx traits; `crates/contract` remains free of database row types
- admin subject request endpoints remain system-admin only
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] backend/frontend subject request DTO usages を確認する
2. [x] contract subject request DTO wire-format tests を先に追加し、missing module で red を確認する
3. [x] `crates/contract::subject_requests` に create / response / list / decision DTOs と API enum を追加する
4. [x] backend model conversion and handler payload conversion を contract DTO に合わせる
5. [x] frontend subject request DTOs を contract re-export に置き換える
6. [x] focused validation を実行し、plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-contract --test subject_requests_contract`
- [x] `cargo test -p timekeeper-backend --test subject_requests_api`
- [x] `cargo test -p timekeeper-backend --test admin_subject_requests_api`
- [x] `cargo test -p timekeeper-frontend subject_request -- --nocapture --test-threads=1`
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
- 2026-06-13: subject request create, response, list, and decision DTOs を `crates/contract::subject_requests` に追加し、snake_case request type, string status, nullable decision fields, and timestamp JSON を contract tests で固定した。
- 2026-06-13: backend persistence model keeps SQLx-aware `DataSubjectRequestType`, while HTTP handlers convert between backend enum/model and contract API DTOs.
- 2026-06-13: admin subject request integration fixtures now use `UserRole::Manager` with `is_system_admin = true`, matching the endpoint's system-admin-only authorization and allowing the DTO/list/decision seam to be validated.
- 2026-06-13: focused contract/backend/frontend tests, frontend check, fmt-check, docs-check, targeted clippy, and diff whitespace check completed successfully.
