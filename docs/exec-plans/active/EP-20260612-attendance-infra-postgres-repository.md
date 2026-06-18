# EP-20260612-attendance-infra-postgres-repository

## Goal
- `docs/design-docs/rebuild-architecture.md` の移行戦略 5 に沿って、attendance workflow の SQLx-backed repository 実装を `crates/infra-postgres` へ移す

## Scope
- In: `crates/infra-postgres`, `backend/src/handlers/attendance.rs`, `backend/src/handlers/attendance_utils.rs`, focused tests, this ExecPlan
- Out: 全 repository の移行、DB migration、route/module split、wire format 変更

## Done Criteria (Observable)
- [x] `crates/infra-postgres` に app-layer attendance repository ports を実装する SQLx repository がある
- [x] backend attendance handlers が workflow SQLx adapter をローカルに持たず、infra repository を use case に渡す
- [x] existing attendance endpoint behavior は focused integration test で維持されている
- [x] infra repository shape test / backend focused tests / lint が成功する

## Constraints / Non-goals
- 現行 endpoint path / method / response body は変更しない
- 既存 backend read/query endpoints はこの slice では移動しない
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] infra repository shape test を先に追加する
2. [x] `crates/infra-postgres` に `AttendanceWorkflowRepository` を追加する
3. [x] backend attendance workflow handlers から infra repository を使う
4. [x] focused validation を実行する
5. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-infra-postgres --test attendance_workflow_repository`
- [x] `cargo test -p timekeeper-backend --lib attendance`
- [x] `cargo test -p timekeeper-backend --test attendance_api`
- [x] `cargo fmt --all --check`
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo clippy -p timekeeper-infra-postgres -p timekeeper-backend --all-targets -- -D warnings`
- [x] `git diff --check`
- [x] `git status --short`

## Git Snapshot Log
- [x] `git status --short`
- [x] focused tests pass
- [ ] commit pending user direction

## Progress Notes
- 2026-06-12: `crates/infra-postgres::attendance::AttendanceWorkflowRepository` を追加し、backend-local workflow SQLx adapter を置き換えた。attendance workflow endpoints は focused integration test で互換確認済み。
