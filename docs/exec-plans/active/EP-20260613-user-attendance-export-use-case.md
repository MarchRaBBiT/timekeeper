# EP-20260613-user-attendance-export-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 4/5 と `apps/api` rule に沿って、user attendance CSV export の read orchestration を `crates/app` use case / `crates/infra-postgres` repository 経由へ移し、CSV construction を handler から外す

## Scope
- In: `crates/app`, `crates/infra-postgres`, `backend/src/handlers/attendance.rs`, `backend/src/utils/csv.rs`, focused tests, this ExecPlan
- Out: admin CSV export migration, response JSON shape changes, frontend changes, generated OpenAPI changes

## Done Criteria (Observable)
- [x] `crates/app` に personal attendance export rows を返す `ExportUserAttendance` use case がある
- [x] app test が optional range export and effective correction export values を固定している
- [x] `crates/infra-postgres` が optional date range 付き user attendance read port を実装している
- [x] backend `export_my_attendance` handler が direct repository aggregation and CSV construction を持たず、use case と CSV utility を呼び出す
- [x] existing export endpoint behavior は focused integration test で維持されている
- [x] app/infra/backend focused tests と lint が成功する

## Constraints / Non-goals
- `{"csv_data","filename"}` の JSON shape は変更しない
- filename timestamp generation stays in the handler
- admin export remains a separate migration slice
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app export use case の tests を先に追加する
2. [x] `crates/app` に `ExportUserAttendance` use case を追加する
3. [x] `crates/infra-postgres` に optional range read port 実装を追加する
4. [x] backend CSV rendering helper を追加する
5. [x] backend `export_my_attendance` handler を use case and utility 呼び出しへ置き換える
6. [x] focused validation を実行する
7. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test list_user_attendance`
- [x] `cargo test -p timekeeper-infra-postgres --test attendance_workflow_repository`
- [x] `cargo test -p timekeeper-backend --lib csv`
- [x] `cargo test -p timekeeper-backend --test attendance_api`
- [x] `cargo test -p timekeeper-backend --lib attendance`
- [x] `cargo fmt --all --check`
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo clippy -p timekeeper-app -p timekeeper-infra-postgres -p timekeeper-backend --all-targets -- -D warnings`
- [x] `git diff --check`
- [x] `git status --short`

## Git Snapshot Log
- [x] `git status --short`
- [x] focused tests pass
- [x] commit recorded: c9b64b3 `feat(rebuild): add modular Rust workflow crates`

## Progress Notes
- 2026-06-13: `ExportUserAttendance` use case and optional-range infra read port added; `export_my_attendance` delegates to app/infra and `utils::csv`, and focused app/infra/backend route tests pass.
- 2026-06-13: backend attendance lib, fmt-check, docs-check, clippy, and diff whitespace validation completed successfully.
