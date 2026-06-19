# EP-20260613-user-attendance-summary-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 4/5 に沿って、user attendance summary を `crates/app` use case と `crates/infra-postgres` repository 経由へ移す

## Scope
- In: `crates/app`, `backend/src/handlers/attendance.rs`, focused tests, this ExecPlan
- Out: CSV export migration, correction request write workflows, API wire format changes, frontend changes

## Done Criteria (Observable)
- [x] `crates/app` に personal attendance summary を表す `GetUserAttendanceSummary` use case がある
- [x] app test が positive work-day counting and effective correction totals を固定している
- [x] backend `get_my_summary` handler が direct repository aggregation ではなく app use case を呼び出す
- [x] existing summary endpoint behavior は focused integration test で維持されている
- [x] app/backend focused tests と lint が成功する

## Constraints / Non-goals
- `AttendanceSummary` の JSON shape は変更しない
- query month parsing and bad-request behavior stay in the handler
- CSV export still uses the old path and will move in a later slice
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app summary use case の tests を先に追加する
2. [x] `crates/app` に `GetUserAttendanceSummary` use case を追加する
3. [x] backend `get_my_summary` handler を use case 呼び出しへ置き換える
4. [x] focused validation を実行する
5. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test list_user_attendance`
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
- 2026-06-13: `GetUserAttendanceSummary` use case added on top of the personal attendance read port; `get_my_summary` delegates to app/infra and focused app/API tests pass.
- 2026-06-13: backend attendance lib, fmt-check, docs-check, clippy, and diff whitespace validation completed successfully.
