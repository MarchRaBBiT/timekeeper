# EP-20260612-admin-force-end-break-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 4/5 に沿って、admin force-end-break workflow を `crates/app` use case と `crates/infra-postgres` repository 経由へ移す

## Scope
- In: `crates/app`, `backend/src/handlers/admin/attendance.rs`, `backend/src/handlers/attendance.rs`, focused tests, this ExecPlan
- Out: admin attendance upsert/list active breaks の移行、route/module split、wire format 変更

## Done Criteria (Observable)
- [x] `crates/app` に owner check なしで active break を閉じる `ForceEndBreak` use case がある
- [x] backend admin force-end handler が direct SQL repository writes ではなく app use case を呼び出す
- [x] existing admin endpoint behavior は focused integration test で維持されている
- [x] app/backend focused tests と lint が成功する

## Constraints / Non-goals
- `BreakRecordResponse` の JSON shape は変更しない
- system admin authorization check は HTTP boundary に残す
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app use case の tests を先に追加する
2. [x] `crates/app` に `ForceEndBreak` use case を追加する
3. [x] backend admin handler を use case 呼び出しへ置き換える
4. [x] focused validation を実行する
5. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test end_break`
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
- 2026-06-12: `ForceEndBreak` use case を追加し、admin force-end handler を app/infra 経由へ移行。focused validation が成功。
