# EP-20260613-user-attendance-range-read-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 4/5 に沿って、user attendance range read を `crates/app` use case と `crates/infra-postgres` repository 経由へ移す

## Scope
- In: `crates/app`, `crates/infra-postgres`, `backend/src/handlers/attendance.rs`, focused tests, this ExecPlan
- Out: attendance summary/export migration, correction request write workflows, API wire format changes, frontend changes

## Done Criteria (Observable)
- [x] `crates/app` に personal attendance range read を表す `ListUserAttendance` use case がある
- [x] app test が break aggregation and effective correction application を固定している
- [x] `crates/infra-postgres` が user attendance rows, break rows, effective correction rows を読む port を実装している
- [x] backend `get_my_attendance` handler が direct repository aggregation ではなく app use case を呼び出す
- [x] existing attendance endpoint behavior は focused integration test で維持されている
- [x] app/infra/backend focused tests と lint が成功する

## Constraints / Non-goals
- `Vec<AttendanceResponse>` の JSON shape は変更しない
- query date-window parsing and bad-request behavior stay in the handler
- summary and CSV export still use the old path and will move in later slices
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app use case の tests を先に追加する
2. [x] `crates/app` に `ListUserAttendance` use case を追加する
3. [x] `crates/infra-postgres` に personal attendance read port 実装を追加する
4. [x] backend `get_my_attendance` handler を use case 呼び出しへ置き換える
5. [x] focused validation を実行する
6. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test list_user_attendance`
- [x] `cargo test -p timekeeper-infra-postgres --test attendance_workflow_repository`
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
- 2026-06-13: `ListUserAttendance` use case and infra read port added; `get_my_attendance` delegates to app/infra and focused app/infra/attendance API tests pass.
- 2026-06-13: backend attendance lib, fmt-check, docs-check, clippy, and diff whitespace validation completed successfully.
