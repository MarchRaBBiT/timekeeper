# EP-20260613-department-contract-dtos

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 2 / 7 に沿って、department management の API DTO を `crates/contract` に移し、backend/frontend の重複 DTO 定義を contract re-export に寄せる

## Scope
- In: `crates/contract`, `backend/src/models/department.rs`, `frontend/src/api/types.rs`, focused contract/backend/frontend tests, this ExecPlan
- Out: department SQL repository migration to `crates/infra-postgres`, department authorization behavior changes, user DTO migration

## Done Criteria (Observable)
- [x] `crates/contract::organization` に department response/create/update/assign-manager DTO がある
- [x] contract test が current department management wire format を固定している
- [x] backend typed department database models remain backend-owned, while API payload/response types come from contract
- [x] frontend API types use contract re-exports instead of local duplicate structs
- [x] focused contract/backend/frontend tests and fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- existing JSON field names, optional parent behavior, and timestamp values are unchanged
- backend `Department` and `DepartmentManager` SQLx row structs remain backend-owned
- manager assignment response messages remain ad-hoc JSON for this slice
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] backend/frontend department DTO usages を確認する
2. [x] contract organization DTO wire-format tests を先に追加し、missing module で red を確認する
3. [x] `crates/contract::organization` に department DTOs を追加する
4. [x] backend model module を contract DTO re-export に置き換える
5. [x] frontend department DTOs を contract re-export に置き換える
6. [x] focused validation を実行し、plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-contract --test organization_contract`
- [x] `cargo test -p timekeeper-backend --test admin_departments_api`
- [x] `cargo test -p timekeeper-frontend department -- --nocapture --test-threads=1`
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
- 2026-06-13: department response/create/update/assign-manager DTOs を `crates/contract::organization` に追加し、missing parent omission, empty parent clearing, response timestamps, and assign-manager payload shape を contract tests で固定した。
- 2026-06-13: backend department model module now re-exports contract DTOs while keeping typed SQLx row structs and existing `From<Department>` response conversion.
- 2026-06-13: frontend API types now re-export department DTOs from `timekeeper-contract`.
- 2026-06-13: focused contract/backend/frontend tests, frontend check, fmt-check, docs-check, targeted clippy, and diff whitespace check completed successfully.
