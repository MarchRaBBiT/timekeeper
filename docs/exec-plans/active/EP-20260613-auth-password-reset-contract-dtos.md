# EP-20260613-auth-password-reset-contract-dtos

## Goal
- `docs/design-docs/rebuild-architecture.md` の migration strategy 2 / 7 に沿って、password reset の API request/response DTO を `crates/contract` に移し、backend/frontend の重複 DTO 定義を contract re-export に寄せる

## Scope
- In: `crates/contract`, `backend/src/models/password_reset.rs`, `backend/src/handlers/auth.rs`, `backend/src/docs.rs`, `backend/tests/password_reset_api.rs`, `frontend/src/api/types.rs`, focused contract/backend/frontend tests, this ExecPlan
- Out: login/session/user DTO migration, password policy redesign, password reset repository migration, email delivery behavior changes

## Done Criteria (Observable)
- [x] `crates/contract::auth` に password reset request/response DTO がある
- [x] contract test が current password reset wire format and validation shape を固定している
- [x] backend password reset payload names remain available as aliases while using contract DTOs
- [x] backend password reset endpoints return the shared message response with the same JSON body
- [x] frontend API types use contract re-exports instead of local duplicate password reset structs
- [x] focused contract/backend/frontend tests and fmt/docs/clippy/diff validation が成功する

## Constraints / Non-goals
- existing JSON field names and success message strings are unchanged
- backend `PasswordReset` SQLx row struct remains backend-owned
- backend configurable password policy remains in `handlers/auth.rs`; the contract DTO only preserves the existing baseline validator shape
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない

## Task Breakdown
1. [x] backend/frontend password reset DTO usages を確認する
2. [x] contract auth DTO wire-format tests を先に追加し、missing module で red を確認する
3. [x] `crates/contract::auth` に password reset DTOs を追加する
4. [x] backend password reset model/handler/docs を contract DTO re-export に置き換える
5. [x] frontend password reset DTOs を contract re-export に置き換える
6. [x] focused validation を実行し、plan と検証結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-contract --test auth_contract`
- [x] `cargo test -p timekeeper-backend --test password_reset_api`
- [x] `cargo test -p timekeeper-frontend reset_password -- --nocapture --test-threads=1`
- [x] `cargo test -p timekeeper-frontend forgot_password -- --nocapture --test-threads=1`
- [x] `cargo test -p timekeeper-frontend api_client_admin_and_auth_endpoints_succeed -- --nocapture --test-threads=1`
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
- 2026-06-13: password reset request/reset request/message response DTOs を `crates/contract::auth` に追加し、email validation, token length, baseline password shape, and message response wire format を contract tests で固定した。
- 2026-06-13: backend password reset model module now re-exports contract DTOs while keeping the `PasswordReset` SQLx row struct backend-owned.
- 2026-06-13: backend password reset handlers now return contract `MessageResponse` with unchanged success message strings; OpenAPI docs now reference the shared response schema.
- 2026-06-13: frontend API types now re-export password reset DTOs from `timekeeper-contract`.
- 2026-06-13: `backend/tests/password_reset_api.rs` local user fixture was updated to return `department_id`, matching the current `User` row model required by the broader rebuild tree.
- 2026-06-13: focused contract/backend/frontend tests, frontend check, fmt-check, docs-check, targeted clippy, diff whitespace check, and git status check completed successfully.
