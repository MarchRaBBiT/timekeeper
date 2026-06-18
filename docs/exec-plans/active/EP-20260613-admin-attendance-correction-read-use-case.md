# EP-20260613-admin-attendance-correction-read-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の handler/use case 分離方針に沿って、管理者の勤怠修正依頼 list/detail read policy を `crates/app` use case へ移し、backend handler から role check / pagination normalization / subordinate scoping / detail authorization policy を外す

## Scope
- In: `crates/app`, `backend/src/handlers/admin/attendance_correction_requests.rs`, `backend/src/handlers/attendance_correction_requests.rs` response helper visibility, focused app/backend tests, this ExecPlan
- Out: correction SQL repository の `crates/infra-postgres` 移設, frontend changes, API response shape changes

## Done Criteria (Observable)
- [x] `crates/app` に admin correction list/detail read use cases and repository port がある
- [x] app test が manager-scoped list, system-admin unscoped list, forbidden employee list, authorized detail, and unauthorized detail を固定している
- [x] backend admin list/detail handlers が直接 read policy を持たず、app use cases を呼び出す
- [x] existing correction API integration behavior が backend test で維持されている
- [x] fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- list/detail response JSON shape は変更しない
- manager scoping uses existing department repository semantics
- correction SQL persistence の `crates/infra-postgres` 移設は後続 slice に分ける
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app admin correction read use case の tests を先に追加する
2. [x] `crates/app` に list/detail query / filters / port / use case / error を追加する
3. [x] backend admin handler に current repository reads を app port に合わせる adapter を追加する
4. [x] backend list/detail handlers を app use case 呼び出しへ置き換える
5. [x] focused app/backend validation を実行する
6. [x] plan と検証結果を最終更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test read_admin_attendance_correction_request`
- [x] `cargo test -p timekeeper-app --test create_attendance_correction_request`
- [x] `cargo test -p timekeeper-app --test manage_attendance_correction_request`
- [x] `cargo test -p timekeeper-app --test decide_attendance_correction_request`
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
- 2026-06-13: `ListAdminAttendanceCorrectionRequests` and `GetAdminAttendanceCorrectionRequest` app use cases added; backend admin list/detail routes now delegate to app use cases through the local adapter while preserving existing response contracts and department authorization semantics.
- 2026-06-13: focused app/backend tests, fmt-check, docs-check, targeted clippy, diff whitespace check, and status check completed successfully.
