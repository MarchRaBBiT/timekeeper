# EP-20260612-attendance-contract-dtos

## Goal
- `docs/design-docs/rebuild-architecture.md` の移行戦略 2 に沿って、attendance DTO を `crates/contract` に移す

## Scope
- In: `crates/contract`, `backend/src/models/attendance.rs`, `backend/src/models/break_record.rs`, `backend/src/handlers/attendance.rs`, `backend/src/docs.rs`, `frontend/src/api/attendance.rs`, `frontend/src/api/types.rs`, Cargo manifests, focused tests, this ExecPlan
- Out: SQLx repository 移動、handler の use case 化、wire format 変更

## Done Criteria (Observable)
- [x] clock-in / clock-out / break-start / break-end request DTO が `crates/contract` に定義されている
- [x] attendance status response DTO が `crates/contract` に定義されている
- [x] break record response DTO が `crates/contract` に定義されている
- [x] attendance summary DTO が `crates/contract` に定義されている
- [x] full attendance response DTO が `crates/contract` に定義されている
- [x] frontend の attendance mutation client が contract DTO を使って request body を生成している
- [x] backend の attendance handler が contract DTO を受け取り、string ID を typed ID に変換している
- [x] backend と frontend が contract-owned attendance status / break record / summary / full response DTO を使っている
- [x] old backend import path は re-export で互換維持されている
- [x] contract wire format test と backend/frontend focused tests が成功する

## Constraints / Non-goals
- 既存 endpoint path / method / response body は変更しない
- SQLx row model の `Attendance` と DB enum の `AttendanceStatus` は backend model に残す
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] contract DTO の wire format test を先に追加する
2. [x] `crates/contract::attendance` に mutation request DTO を追加する
3. [x] backend model の request DTO 定義を contract re-export に置き換える
4. [x] backend handler で string ID を typed ID に変換する
5. [x] frontend attendance API client を contract DTO 利用へ切り替える
6. [x] attendance status response を contract crate へ移し backend/frontend から利用する
7. [x] break record response を contract crate へ移し backend/frontend から利用する
8. [x] attendance summary を contract crate へ移し backend/frontend から利用する
9. [x] full attendance response を contract crate へ移し backend/frontend/admin から利用する
10. [x] focused validation を実行する

## Validation Plan
- [x] `cargo test -p timekeeper-contract --test attendance_contract`
- [x] `cargo test -p timekeeper-backend --lib attendance`
- [x] `cargo test -p timekeeper-backend --lib break_record`
- [x] `cargo test -p timekeeper-backend --test attendance_breaks_api`
- [x] `cargo test -p timekeeper-backend --test attendance_api`
- [x] `cargo test -p timekeeper-backend --test admin_attendance_api`
- [x] `cargo test -p timekeeper-frontend attendance -- --nocapture --test-threads=1`
- [x] `cargo fmt --all --check`
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo clippy -p timekeeper-contract -p timekeeper-app -p timekeeper-backend -p timekeeper-frontend --all-targets -- -D warnings`
- [x] `git diff --check`
- [x] `git status --short`

## Git Snapshot Log
- [x] `git status --short`
- [x] focused tests pass
- [ ] commit pending user direction

## Progress Notes
- 2026-06-12: attendance mutation request DTO を contract crate へ移行。frontend は ad hoc JSON ではなく contract DTO を serialize し、backend は旧 import path を re-export で維持しつつ handler 境界で string ID を typed ID に変換する。
- 2026-06-12: `AttendanceStatusResponse` も contract crate へ移行し、backend docs/handler と frontend api types は contract-owned type を参照するようにした。contract/backend/frontend focused tests、workspace fmt、docs-check、touched package clippy、`git diff --check` が成功。
- 2026-06-12: `BreakRecordResponse` と `AttendanceSummary` も contract crate へ移行。backend は typed ID を response boundary で string 化し、frontend は `crate::api::*` re-export 互換を維持。`attendance_breaks_api` も wire-format string ID 期待へ更新して成功。
- 2026-06-12: `AttendanceResponse` も contract crate へ移行。backend の `build_attendance_response` と admin attendance handler で typed ID / DB enum を contract string に変換する境界を明示化。`attendance_api` と `admin_attendance_api` を含む focused validation が成功。
