# EP-20260613-holiday-contract-dtos

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 2 / 7 に沿って、holiday / weekly holiday / holiday calendar の API DTO を `crates/contract` に移し、backend/frontend の重複 DTO 定義を contract re-export に寄せる

## Scope
- In: `crates/contract`, `backend/src/models/holiday.rs`, `backend/src/handlers/holidays.rs`, `backend/src/handlers/admin/holidays.rs`, `frontend/src/api/types.rs`, focused contract/backend/frontend tests, this ExecPlan
- Out: holiday SQL repository migration to `crates/infra-postgres`, holiday exception DTO migration, Google Calendar fetch behavior changes, holiday authorization behavior changes

## Done Criteria (Observable)
- [x] `crates/contract::holidays` に holiday/public weekly/admin list/calendar DTO がある
- [x] contract test が current holiday wire format を固定している
- [x] backend typed holiday database models remain backend-owned, while API payload/response types come from contract
- [x] frontend API types use contract re-exports instead of local duplicate holiday structs
- [x] focused contract/backend/frontend tests and fmt/docs/clippy validation が成功する

## Constraints / Non-goals
- existing JSON field names, nullable optional fields, snake_case admin holiday kind values, and date/timestamp formats are unchanged
- backend `Holiday` and `WeeklyHoliday` SQLx row structs remain backend-owned
- backend handler-facing aliases such as `CreateHolidayPayload` and `CreateWeeklyHolidayPayload` are retained for local compatibility
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] backend/frontend holiday DTO usages を確認する
2. [x] contract holiday DTO wire-format tests を先に追加し、missing module で red を確認する
3. [x] `crates/contract::holidays` に holiday DTOs を追加する
4. [x] backend model/handler modules を contract DTO re-export に置き換える
5. [x] frontend holiday DTOs を contract re-export に置き換える
6. [x] focused validation を実行し、plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-contract --test holidays_contract`
- [x] `cargo test -p timekeeper-backend --test admin_holidays_api`
- [x] `cargo test -p timekeeper-backend --test admin_holiday_list`
- [x] `cargo test -p timekeeper-backend holidays::`
- [x] `cargo test -p timekeeper-frontend holiday -- --nocapture --test-threads=1`
- [x] `cargo check -p timekeeper-frontend`
- [x] `cargo fmt --all --check`
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo clippy -p timekeeper-contract -p timekeeper-backend -p timekeeper-frontend --all-targets -- -D warnings`
- [x] `git diff --check`
- [x] `git status --short`

## Git Snapshot Log
- [x] focused tests pass
- [x] `git status --short`
- [x] commit recorded: c9b64b3 `feat(rebuild): add modular Rust workflow crates`

## Progress Notes
- 2026-06-13: holiday/public weekly/admin list/calendar DTOs を `crates/contract::holidays` に追加し、nullable description, default missing weekly end date, admin kind snake_case, and calendar/check response shape を contract tests で固定した。
- 2026-06-13: backend holiday model module now re-exports contract DTOs while keeping typed SQLx row structs and existing `From<Holiday>` / `From<WeeklyHoliday>` response conversions.
- 2026-06-13: backend public/admin holiday handlers now use contract response DTOs and retain existing query validation and payload aliases.
- 2026-06-13: frontend API types now re-export holiday DTOs from `timekeeper-contract`.
- 2026-06-13: focused contract/backend/frontend tests, frontend check, fmt-check, docs-check, targeted clippy, diff whitespace check, and git status check completed successfully.
