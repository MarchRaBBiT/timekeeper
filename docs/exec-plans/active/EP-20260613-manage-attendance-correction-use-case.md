# EP-20260613-manage-attendance-correction-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の handler/use case 分離方針に沿って、従業員の勤怠修正依頼 update/cancel policy を `crates/app` use case へ移し、backend handler から pending-only checks and snapshot validation を外す

## Scope
- In: `crates/app`, `backend/src/handlers/attendance_correction_requests.rs`, focused app/backend tests, this ExecPlan
- Out: admin approval/rejection migration, list/detail migration, SQLx correction repository の `crates/infra-postgres` 移設, frontend changes, API response shape changes

## Done Criteria (Observable)
- [x] `crates/app` に update/cancel attendance correction request use case and repository ports がある
- [x] app test が pending-only update, no-change rejection, proposed value construction, and cancel delegation を固定している
- [x] backend update/cancel correction handlers が直接 snapshot policy を持たず、app use cases を呼び出す
- [x] existing employee create/update/cancel integration behavior が focused backend test で維持されている
- [x] fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- `AttendanceCorrectionResponse` JSON shape と cancel response shape は変更しない
- create correction use case behavior は維持する
- admin approval/rejection は既存 repository path のまま残す
- correction SQL persistence の `crates/infra-postgres` 移設は後続 slice に分ける
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app update/cancel correction use case の tests を先に追加する
2. [x] `crates/app` に update/cancel command / port / use case を追加する
3. [x] backend repository adapter に update/cancel ports を実装する
4. [x] backend update/cancel handlers を app use case 呼び出しへ置き換える
5. [x] focused app/backend validation を実行する
6. [x] plan と検証結果を最終更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test manage_attendance_correction_request`
- [x] `cargo test -p timekeeper-app --test create_attendance_correction_request`
- [x] `cargo test -p timekeeper-backend --test attendance_correction_api employee_can_create_update_and_cancel_attendance_correction`
- [x] `cargo fmt --all --check`
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo clippy -p timekeeper-app -p timekeeper-backend --all-targets -- -D warnings`
- [x] `git diff --check`
- [x] `git status --short`

## Git Snapshot Log
- [x] `git status --short`
- [x] focused tests pass
- [ ] commit pending user direction

## Progress Notes
- 2026-06-13: `UpdateAttendanceCorrectionRequest` and `CancelAttendanceCorrectionRequest` app use cases added; backend user update/cancel correction routes now delegate to app use cases through the local adapter while preserving existing response contracts.
- 2026-06-13: focused app/backend tests, fmt-check, docs-check, targeted clippy, diff whitespace check, and status check completed successfully.
