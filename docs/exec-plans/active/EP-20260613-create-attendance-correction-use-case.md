# EP-20260613-create-attendance-correction-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の handler/use case 分離方針に沿って、従業員の勤怠修正依頼作成 policy を `crates/app` use case へ移し、backend handler を request parsing / use case invocation / response conversion へ寄せる

## Scope
- In: `crates/app`, `backend/src/handlers/attendance_correction_requests.rs`, focused app/backend tests, this ExecPlan
- Out: admin approval/rejection migration, user update/cancel migration, SQLx correction repository の `crates/infra-postgres` 移設, frontend changes, API response shape changes

## Done Criteria (Observable)
- [x] `crates/app` に create attendance correction request use case and repository port がある
- [x] app test が original snapshot assembly, proposed overrides, no-change rejection, reason validation, and break ordering validation を固定している
- [x] backend create correction handler が直接 snapshot policy を持たず、app use case を呼び出す
- [x] existing employee create/update/cancel integration behavior が focused backend test で維持されている
- [x] fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- `AttendanceCorrectionResponse` JSON shape は変更しない
- user update/cancel と admin approval/rejection は既存 repository path のまま残す
- correction SQL persistence の `crates/infra-postgres` 移設は後続 slice に分ける
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app create correction use case の tests を先に追加する
2. [x] `crates/app` に command / record / snapshot / repository port / error / use case を追加する
3. [x] backend handler に existing repositories を app port に合わせる adapter を追加する
4. [x] backend create handler を app use case 呼び出しへ置き換える
5. [x] focused app/backend validation を実行する
6. [x] plan と検証結果を最終更新する

## Validation Plan
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
- [x] commit recorded: c9b64b3 `feat(rebuild): add modular Rust workflow crates`

## Progress Notes
- 2026-06-13: `CreateAttendanceCorrectionRequest` use case and app-layer policy tests added; backend create correction route now delegates to the use case through a local adapter while preserving the existing response contract.
- 2026-06-13: focused app/backend tests, fmt-check, docs-check, targeted clippy, diff whitespace check, and status check completed successfully.
