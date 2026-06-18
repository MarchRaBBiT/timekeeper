# EP-20260613-admin-attendance-export-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 4/5 と `apps/api` rule に沿って、admin attendance CSV export の authorization/scoping/query orchestration を `crates/app` use case / `crates/infra-postgres` repository 経由へ移し、CSV construction を handler から外す

## Scope
- In: `crates/app`, `crates/infra-postgres`, `backend/src/handlers/admin/export.rs`, `backend/src/utils/csv.rs`, focused tests, this ExecPlan
- Out: PII decrypt/mask infrastructure migration, admin audit-log export migration, response JSON/header shape changes, frontend changes

## Done Criteria (Observable)
- [x] `crates/app` に admin attendance export を表す `ExportAdminAttendance` use case がある
- [x] app test が forbidden access, system-admin unscoped export, manager subordinate scoping を固定している
- [x] `crates/infra-postgres` が manager subordinate user scope and filtered admin attendance export rows を読む port を実装している
- [x] backend `export_data` handler が direct SQL query construction and CSV construction を持たず、use case と CSV utility を呼び出す
- [x] existing admin export endpoint behavior は focused integration test で維持されている
- [x] app/infra/backend focused tests と lint が成功する

## Constraints / Non-goals
- `{"csv_data","filename"}` body and `X-PII-Masked` header shape は変更しない
- full-name decrypt/mask remains in backend until `infra-security` migration
- audit-log export remains a separate migration slice
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app admin export use case の tests を先に追加する
2. [x] `crates/app` に `ExportAdminAttendance` use case を追加する
3. [x] `crates/infra-postgres` に subordinate scope and filtered export read port 実装を追加する
4. [x] backend CSV rendering helper を追加する
5. [x] backend `export_data` handler を use case and utility 呼び出しへ置き換える
6. [x] focused validation を実行する
7. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test export_admin_attendance`
- [x] `cargo test -p timekeeper-infra-postgres --test attendance_workflow_repository`
- [x] `cargo test -p timekeeper-backend --lib csv`
- [x] `cargo test -p timekeeper-backend --test admin_export_api`
- [x] `cargo test -p timekeeper-backend --lib admin`
- [x] `cargo fmt --all --check`
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo clippy -p timekeeper-app -p timekeeper-infra-postgres -p timekeeper-backend --all-targets -- -D warnings`
- [x] `git diff --check`
- [x] `git status --short`

## Git Snapshot Log
- [x] `git status --short`
- [x] focused tests pass
- [ ] commit pending user direction

## Progress Notes
- 2026-06-13: `ExportAdminAttendance` use case and infra read port added; admin `export_data` delegates to app/infra and `utils::csv`, and focused app/infra/backend route tests pass.
- 2026-06-13: backend admin lib, fmt-check, docs-check, clippy, and diff whitespace validation completed successfully.
