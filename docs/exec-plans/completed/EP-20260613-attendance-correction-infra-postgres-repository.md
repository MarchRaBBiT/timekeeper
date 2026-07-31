# EP-20260613-attendance-correction-infra-postgres-repository

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 5 に沿って、勤怠修正依頼の SQLx persistence を `crates/infra-postgres` repository へ移し、backend handler / repository 層から correction SQL ownership を外す

## Scope
- In: `crates/infra-postgres`, `backend/src/handlers/attendance_correction_requests.rs`, `backend/src/handlers/admin/attendance_correction_requests.rs`, `backend/src/handlers/requests.rs`, obsolete backend correction repository, focused app/infra/backend tests, this ExecPlan
- Out: API response shape changes, DB migration changes, frontend changes, non-correction request repositories

## Done Criteria (Observable)
- [x] `crates/infra-postgres` に correction create/update/cancel/admin-read/decision app ports を実装する SQLx repository がある
- [x] infra-postgres test が correction repository の app port conformance と PgPool ownership を固定している
- [x] user/admin correction handlers and combined requests handler が backend-local correction SQL repository ではなく infra-postgres repository を使う
- [x] obsolete `backend/src/repositories/attendance_correction_request.rs` が削除されている
- [x] focused app/infra/backend tests and fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- existing API JSON contract は変更しない
- existing attendance correction tables and migrations は変更しない
- approval transaction semantics and conflict detection are preserved
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] correction repository usages and current infra-postgres structure を確認する
2. [x] infra-postgres correction repository conformance test を先に追加し、missing module で red を確認する
3. [x] `crates/infra-postgres::attendance_correction::AttendanceCorrectionRepository` を追加する
4. [x] user/admin correction handlers and combined requests handler を infra-postgres repository に wire する
5. [x] obsolete backend correction repository and dead row model helpers を削除する
6. [x] focused validation を実行し、plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-infra-postgres --test attendance_correction_repository`
- [x] `cargo test -p timekeeper-backend --test attendance_correction_api`
- [x] `cargo test -p timekeeper-backend --test requests_api`
- [x] `cargo test -p timekeeper-backend --test request_update_api`
- [x] `cargo test -p timekeeper-app --test create_attendance_correction_request`
- [x] `cargo test -p timekeeper-app --test manage_attendance_correction_request`
- [x] `cargo test -p timekeeper-app --test decide_attendance_correction_request`
- [x] `cargo test -p timekeeper-app --test read_admin_attendance_correction_request`
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
- 2026-06-13: `AttendanceCorrectionRepository` を `crates/infra-postgres` に追加し、correction create/update/cancel/admin read/approve/reject app ports and department authorization queries を SQLx-backed implementation に移した。
- 2026-06-13: user/admin correction handlers and combined requests handler now use the infra-postgres correction repository. Obsolete `backend/src/repositories/attendance_correction_request.rs` and dead backend row-model conversion helpers were removed while preserving existing API response DTOs.
- 2026-06-13: focused app/infra/backend tests, fmt-check, docs-check, targeted clippy, and diff whitespace check completed successfully.
