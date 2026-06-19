# EP-20260613-admin-attendance-contract-dtos

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 2 に沿って、admin attendance upsert と active-break response の API DTO を `crates/contract` に移し、backend/frontend の重複 DTO 定義を contract re-export に寄せる

## Scope
- In: `crates/contract`, `backend/src/handlers/admin/attendance.rs`, `backend/src/models/break_record.rs`, `backend/src/repositories/break_record.rs`, `backend/src/docs.rs`, `backend/tests/admin_attendance_api.rs`, `frontend/src/api/types.rs`, `frontend/src/pages/admin/components/attendance.rs`, focused contract/backend/frontend tests, this ExecPlan
- Out: API response shape changes, DB migration changes, admin attendance use-case routing changes, non-admin attendance DTO migration

## Done Criteria (Observable)
- [x] `crates/contract::attendance` に admin attendance upsert / break item / active-break response DTO がある
- [x] contract test が current admin attendance upsert and active-break wire format を固定している
- [x] backend admin attendance handler/docs/model modules が local duplicate DTO ではなく contract DTO を使う
- [x] backend active-break repository が DB row type と API response DTO を分離している
- [x] frontend admin attendance payload and active-break response types が local duplicate structs ではなく contract re-export を使う
- [x] focused contract/backend/frontend tests and fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- existing JSON field names and timestamp string formats are unchanged
- app-layer admin attendance workflow types remain app-owned; only API DTOs move
- repository SQL row mapping stays backend-owned until the infra-postgres boundary is migrated further
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] backend/frontend admin attendance DTO usages を確認する
2. [x] contract admin attendance DTO wire-format tests を先に追加し、missing types で red を確認する
3. [x] `crates/contract::attendance` に admin upsert / active-break DTOs を追加する
4. [x] backend handler/docs/model/repository を contract DTO re-export と local DB row mapping に置き換える
5. [x] frontend admin payload builders and API fixtures を contract string DTO に合わせる
6. [x] focused validation を実行し、plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-contract --test attendance_contract`
- [x] `cargo test -p timekeeper-backend --test admin_attendance_api`
- [x] `cargo test -p timekeeper-frontend admin -- --nocapture --test-threads=1`
- [x] `cargo test -p timekeeper-frontend api_client_attendance_and_requests_endpoints_succeed -- --nocapture --test-threads=1`
- [x] `cargo check -p timekeeper-frontend`
- [x] `cargo fmt --all --check`
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo clippy -p timekeeper-contract -p timekeeper-backend -p timekeeper-frontend --all-targets -- -D warnings`
- [x] `git diff --check`
- [x] `git status --short`

## Git Snapshot Log
- [x] `git status --short`
- [x] focused tests pass
- [x] commit recorded: c9b64b3 `feat(rebuild): add modular Rust workflow crates`

## Progress Notes
- 2026-06-13: admin attendance upsert and active-break DTOs を `crates/contract::attendance` に追加し、wire-format tests で existing string date/datetime fields, nullable break end, and active-break timestamp JSON を固定した。
- 2026-06-13: backend admin attendance handler/docs/model modules now use contract DTOs, while active-break SQL mapping uses a backend-local row type before converting to API response DTO.
- 2026-06-13: frontend admin attendance payload builders now validate date/datetime with chrono but emit contract string DTO fields through the existing `crate::api` re-export path.
- 2026-06-13: focused contract/backend/frontend tests, frontend check, fmt-check, docs-check, targeted clippy, and diff whitespace check completed successfully.
