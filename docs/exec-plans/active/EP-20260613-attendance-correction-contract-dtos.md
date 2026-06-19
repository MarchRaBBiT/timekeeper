# EP-20260613-attendance-correction-contract-dtos

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 2 に沿って、勤怠修正依頼の API DTO を `crates/contract` に移し、backend/frontend の重複 DTO 定義を contract re-export に寄せる

## Scope
- In: `crates/contract`, `backend/src/models/attendance_correction_request.rs`, `backend/src/handlers/attendance_correction_requests.rs`, `frontend/src/api/types.rs`, focused contract/backend/frontend tests, this ExecPlan
- Out: API response shape changes, DB migration changes, frontend UI behavior changes, non-correction request DTO migration

## Done Criteria (Observable)
- [x] `crates/contract::attendance` に correction create/update/decision/snapshot/response DTO がある
- [x] contract test が existing correction request/response wire format を固定している
- [x] backend correction model module が contract DTO を re-export し、handler response conversion が API string IDs を返す
- [x] frontend correction request payload types が local duplicate structs ではなく contract re-export を使う
- [x] focused contract/backend/frontend tests and fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- existing JSON field names and status values are unchanged
- app-layer correction workflow types remain app-owned; only API DTOs move
- frontend correction API responses still map through existing `Value` paths where the UI already expects mixed request payloads
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] backend/frontend correction DTO usages を確認する
2. [x] contract correction DTO wire-format tests を先に追加し、missing types で red を確認する
3. [x] `crates/contract::attendance` に correction DTOs を追加する
4. [x] backend model module を contract DTO re-export に置き換える
5. [x] frontend request payload DTOs を contract re-export に置き換える
6. [x] focused validation を実行し、plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-contract --test attendance_contract`
- [x] `cargo test -p timekeeper-backend --test attendance_correction_api`
- [x] `cargo test -p timekeeper-backend --test requests_api`
- [x] `cargo test -p timekeeper-frontend requests -- --nocapture --test-threads=1`
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
- 2026-06-13: correction create/update/decision/snapshot/response DTOs を `crates/contract::attendance` に追加し、wire-format tests で existing snake_case status, nested snapshots, nullable optional fields, and timestamp JSON を固定した。
- 2026-06-13: backend correction model module now re-exports contract DTOs, and response conversion returns API string IDs directly. Frontend correction request payload types now come from `timekeeper-contract` through the existing `crate::api` re-export path.
- 2026-06-13: focused contract/backend/frontend tests, frontend check, fmt-check, docs-check, targeted clippy, and diff whitespace check completed successfully.
