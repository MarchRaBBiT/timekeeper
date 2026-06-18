# EP-20260612-admin-attendance-page-read-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 4/5 に沿って、admin attendance pagination read を `crates/app` use case と `crates/infra-postgres` repository 経由へ移す

## Scope
- In: `crates/app`, `crates/infra-postgres`, `backend/src/handlers/admin/attendance.rs`, focused tests, this ExecPlan
- Out: admin attendance upsert migration, PII decrypt infrastructure migration, route/module split, wire format changes

## Done Criteria (Observable)
- [x] `crates/app` に attendance page read を表す `ListAttendancePage` use case がある
- [x] `crates/infra-postgres` が page count/list/break batch read port を実装している
- [x] backend admin get-all-attendance handler が direct repository pagination ではなく app use case を呼び出す
- [x] existing admin endpoint behavior は focused integration test で維持されている
- [x] app/infra/backend focused tests と lint が成功する

## Constraints / Non-goals
- `PaginatedResponse<AttendanceResponse>` の JSON shape は変更しない
- system admin authorization check と DTO translation は HTTP boundary に残す
- admin attendance upsert の transaction workflow は別 slice で扱う
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app use case の tests を先に追加する
2. [x] `crates/app` に `ListAttendancePage` use case を追加する
3. [x] `crates/infra-postgres` に page read port 実装を追加する
4. [x] backend admin handler を use case 呼び出しへ置き換える
5. [x] focused validation を実行する
6. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test list_attendance_page`
- [x] `cargo test -p timekeeper-infra-postgres --test attendance_workflow_repository`
- [x] `cargo test -p timekeeper-backend --test admin_attendance_api`
- [x] `cargo test -p timekeeper-backend --lib attendance`
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
- 2026-06-12: `ListAttendancePage` use case と infra read port 実装を追加し、admin attendance pagination handler を app/infra 経由へ移行開始。app/infra/admin focused tests は成功。
- 2026-06-12: backend attendance lib, fmt-check, docs-check, clippy, and diff whitespace validation completed successfully.
