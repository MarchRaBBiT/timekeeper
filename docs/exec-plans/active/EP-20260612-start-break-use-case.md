# EP-20260612-start-break-use-case

## Goal
- `docs/design-docs/rebuild-architecture.md` の移行戦略 4 に沿って、attendance break-start workflow を `crates/app` の use case として抽出する

## Scope
- In: `crates/app`, `backend/src/handlers/attendance.rs`, focused tests, this ExecPlan
- Out: break-end workflow の移行、SQLx repository の `crates/infra-postgres` 移動、DB migration、wire format 変更

## Done Criteria (Observable)
- [x] `crates/app` に attendance lookup / ownership check / clock-in required / active break rejection / break creation を持つ `StartBreak` use case がある
- [x] backend break-start handler が直接 orchestration せず、app use case を呼び出す
- [x] existing backend endpoint behavior は focused tests で維持されている
- [x] app use case tests と backend attendance focused tests が成功する

## Constraints / Non-goals
- 現行 endpoint path / method / response body は変更しない
- SQLx query implementation はこの slice では既存 backend repository に残す
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] app use case の tests を先に追加する
2. [x] `crates/app` の `StartBreak` use case を port-based workflow にする
3. [x] backend handler に app use case adapter を追加する
4. [x] focused validation を実行する
5. [x] plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-app --test start_break`
- [x] `cargo test -p timekeeper-backend --lib attendance`
- [x] `cargo test -p timekeeper-backend --test attendance_api`
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
- 2026-06-12: `crates/app::attendance::StartBreak` を repository port による workflow として追加。backend break-start handler から adapter 経由で呼び出す形へ移行し、focused validation が成功。
