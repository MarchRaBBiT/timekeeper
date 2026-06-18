# EP-20260612-breaks-by-attendance-read-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 4/5 に沿って、attendance break-list read workflow を `crates/app` use case と `crates/infra-postgres` repository に移す

## Scope
- In: `crates/app`, `crates/infra-postgres`, `backend/src/handlers/attendance.rs`, `backend/src/handlers/attendance_utils.rs`, focused tests, this ExecPlan
- Out: attendance list/summary/export の移行、admin attendance repository 移行、wire format 変更

## Done Criteria (Observable)
- [x] `crates/app` に break list ownership check を持つ `GetBreaksByAttendance` use case がある
- [x] `crates/infra-postgres` が break-list read port を実装している
- [x] backend `get_breaks_by_attendance` handler が direct repository reads ではなく app use case を呼び出す
- [x] existing endpoint behavior は focused integration test で維持されている
- [x] app/infra/backend focused tests と lint が成功する

## Constraints / Non-goals
- `BreakRecordResponse` の JSON shape は変更しない
- status / list / summary / export 以外の read endpoints はこの slice では移動しない
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app use case の tests を先に追加する
2. [x] `crates/app` に `GetBreaksByAttendance` use case と read port を追加する
3. [x] `crates/infra-postgres` に read port 実装を追加する
4. [x] backend handler を use case 呼び出しへ置き換える
5. [x] focused validation を実行する
6. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test get_breaks_by_attendance`
- [x] `cargo test -p timekeeper-infra-postgres --test attendance_workflow_repository`
- [x] `cargo test -p timekeeper-backend --lib attendance`
- [x] `cargo test -p timekeeper-backend --test attendance_api`
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
- 2026-06-12: `GetBreaksByAttendance` use case と infra read port 実装を追加し、backend break-list handler を use case 呼び出しへ移行。focused validation が成功。
