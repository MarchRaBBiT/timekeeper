# EP-20260613-admin-attendance-correction-decision-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の handler/use case 分離方針に沿って、管理者の勤怠修正依頼 approve/reject decision policy を `crates/app` use case へ移し、backend handler から comment validation / role check / self-decision denial / manager authorization policy を外す

## Scope
- In: `crates/app`, `backend/src/handlers/admin/attendance_correction_requests.rs`, `backend/src/handlers/attendance_correction_requests.rs` conversion helper visibility, `backend/tests/attendance_correction_api.rs`, focused tests, this ExecPlan
- Out: admin list/detail migration, correction SQL repository の `crates/infra-postgres` 移設, frontend changes, API response shape changes

## Done Criteria (Observable)
- [x] `crates/app` に approve/reject attendance correction request use case and repository port がある
- [x] app test が manager authorization, system admin rejection, self-decision denial, and comment validation を固定している
- [x] backend approve/reject handlers が直接 decision policy を持たず、app use cases を呼び出す
- [x] existing correction approval/rejection integration behavior and concurrency behavior が backend test で維持されている
- [x] manager success test fixtures explicitly establish department authorization
- [x] fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- approve/reject response JSON shape は変更しない
- approval transaction and effective-value upsert stay in the current backend repository for this slice
- admin list/detail remain on existing repository path
- correction SQL persistence の `crates/infra-postgres` 移設は後続 slice に分ける
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app approve/reject decision use case の tests を先に追加する
2. [x] `crates/app` に decision command / port / use case / error を追加する
3. [x] backend admin handler に current repository transaction を app port に合わせる adapter を追加する
4. [x] backend approve/reject handlers を app use case 呼び出しへ置き換える
5. [x] manager success integration fixtures に department authorization setup を追加する
6. [x] focused app/backend validation を実行する
7. [x] plan と検証結果を最終更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test decide_attendance_correction_request`
- [x] `cargo test -p timekeeper-app --test create_attendance_correction_request`
- [x] `cargo test -p timekeeper-app --test manage_attendance_correction_request`
- [x] `cargo test -p timekeeper-backend --test attendance_correction_api`
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
- 2026-06-13: `ApproveAttendanceCorrectionRequest` and `RejectAttendanceCorrectionRequest` app use cases added; backend admin approve/reject routes now delegate to app use cases through a local adapter while preserving existing response contracts and repository transaction semantics.
- 2026-06-13: correction API manager success fixtures now explicitly assign the employee to a department managed by the acting manager, matching the department-scoped authorization model.
- 2026-06-13: focused app/backend tests, fmt-check, docs-check, targeted clippy, diff whitespace check, and status check completed successfully.
