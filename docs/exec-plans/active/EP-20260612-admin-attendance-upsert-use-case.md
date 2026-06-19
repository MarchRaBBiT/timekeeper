# EP-20260612-admin-attendance-upsert-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 4/5 に沿って、admin attendance upsert workflow を `crates/app` use case と `crates/infra-postgres` repository transaction 経由へ移す

## Scope
- In: `crates/app`, `crates/infra-postgres`, `backend/src/handlers/admin/attendance.rs`, obsolete backend transaction helpers, focused tests, this ExecPlan
- Out: request parsing/validation redesign, admin route/module split, API wire format changes, frontend admin UI changes

## Done Criteria (Observable)
- [x] `crates/app` に admin attendance replacement を表す `UpsertAttendance` use case がある
- [x] break duration and total work-hour calculation are covered by app tests
- [x] `crates/infra-postgres` が delete/insert attendance and breaks を transaction 内で実行する upsert port を実装している
- [x] backend admin upsert handler が direct transaction orchestration ではなく app use case を呼び出す
- [x] existing admin endpoint behavior は focused integration test で維持されている
- [x] app/infra/backend focused tests と lint が成功する

## Constraints / Non-goals
- `AttendanceResponse` の JSON shape は変更しない
- HTTP request parsing and bad-request messages stay in the handler
- system admin authorization check stays in the handler
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app use case の tests を先に追加する
2. [x] `crates/app` に `UpsertAttendance` use case を追加する
3. [x] `crates/infra-postgres` に transactional upsert port 実装を追加する
4. [x] backend admin handler を use case 呼び出しへ置き換える
5. [x] obsolete backend transaction helper methods を削除する
6. [x] focused validation を実行する
7. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test upsert_attendance`
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
- [x] commit recorded: c9b64b3 `feat(rebuild): add modular Rust workflow crates`

## Progress Notes
- 2026-06-12: `UpsertAttendance` use case and transactional infra implementation added; admin upsert handler delegates to app/infra and focused app/admin tests pass.
- 2026-06-12: backend attendance lib, fmt-check, docs-check, clippy, and diff whitespace validation completed successfully.
