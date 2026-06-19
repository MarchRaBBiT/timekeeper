# EP-20260612-admin-active-breaks-read-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 4/5 に沿って、admin active-break listing を `crates/app` use case と `crates/infra-postgres` repository 経由へ移す

## Scope
- In: `crates/app`, `crates/infra-postgres`, `backend/src/handlers/admin/attendance.rs`, focused tests, this ExecPlan
- Out: admin attendance upsert/all attendance pagination の移行、PII decrypt infrastructure 移動、route/module split、wire format 変更

## Done Criteria (Observable)
- [x] `crates/app` に active break rows を取得する `ListActiveBreaks` use case がある
- [x] `crates/infra-postgres` が active-break listing port を実装している
- [x] backend admin list-active-breaks handler が direct SQL repository reads ではなく app use case を呼び出す
- [x] existing admin endpoint behavior は focused integration test で維持されている
- [x] app/infra/backend focused tests と lint が成功する

## Constraints / Non-goals
- `ActiveBreakResponse` の JSON shape は変更しない
- system admin authorization check と full_name decrypt は HTTP boundary に残す
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app use case の tests を先に追加する
2. [x] `crates/app` に `ListActiveBreaks` use case を追加する
3. [x] `crates/infra-postgres` に active-break listing port 実装を追加する
4. [x] backend admin handler を use case 呼び出しへ置き換える
5. [x] focused validation を実行する
6. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test list_active_breaks`
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
- 2026-06-12: `ListActiveBreaks` use case と infra read port 実装を追加し、admin active-break listing handler を app/infra 経由へ移行。focused validation が成功。
