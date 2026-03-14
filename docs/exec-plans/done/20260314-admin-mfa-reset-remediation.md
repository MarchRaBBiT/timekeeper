# EP-20260314-admin-mfa-reset-remediation

## Goal
- admin による MFA reset を、インシデント対応として期待される「即時セッション遮断」まで含む安全な挙動にする

## Scope
- In: `backend/src/handlers/admin/users.rs`, `backend/src/repositories/user.rs`, `backend/src/repositories/auth.rs`, `backend/src/repositories/active_session*`, `backend/src/services/token_cache.rs`, 関連 backend tests
- Out: MFA UX の再設計、認証方式の変更、フロント画面の文言見直し

## Done Criteria (Observable)
- [x] admin MFA reset 実行後、対象ユーザーの `refresh_tokens` / `active_access_tokens` / `active_sessions` がすべて失効する
- [x] reset 前に発行済みの access token で保護 API を叩いても `401/403` になることを test で確認できる
- [x] token cache が有効な構成でも stale token が通らないことを実装と invalidate hook で確認し、focused validation を実施する

## Constraints / Non-goals
- system admin 限定の権限制御は維持する
- 既存 route / method は変更しない
- 既存の self-service MFA disable の invalidate 挙動と整合させる

## Task Breakdown
1. [x] 現状の admin MFA reset と self-service MFA disable の invalidate 差分を棚卸しする
2. [x] admin MFA reset でも access token / active session / token cache を確実に失効させる
3. [x] admin reset 後に旧 access token が拒否される integration test を追加する
4. [x] 影響する audit / session repository の副作用を確認する

## Validation Plan
- [x] `bash scripts/harness.sh fmt-check`
- [ ] `cargo test -p timekeeper-backend --test mfa_api -- --nocapture`
- [ ] `cargo test -p timekeeper-backend --test session_api -- --nocapture`
- [x] admin MFA reset を cover する focused backend test
- [ ] `bash scripts/harness.sh backend-integration`
- [x] `cargo test -p timekeeper-backend --test admin_users_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test user_repository -- --nocapture`
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`

## Git Checkpoint Log
- [x] `git status --short`
- [x] admin MFA reset focused tests pass
- [x] `git commit -m "fix(security): revoke active sessions on admin mfa reset"`

## Progress Notes
- 2026-03-14: `security_best_practices_report.md` の admin MFA reset finding から個別 ExecPlan を作成。
- 2026-03-14: issue #448 を作成し、`fix/admin-mfa-reset-revoke-active-sessions` ブランチで着手開始。回帰テストを先に追加して現状の欠陥を固定する方針にした。
- 2026-03-14: `test_system_admin_reset_mfa_revokes_existing_access_tokens_and_sessions` を追加し、現状コードで `active_access_tokens` が残る失敗を確認した。
- 2026-03-14: admin MFA reset を transaction 内で `active_sessions` / `active_access_tokens` / `refresh_tokens` 全失効へ拡張し、handler 側で token cache invalidate と API catalog 更新を反映した。
- 2026-03-14: `bash scripts/harness.sh fmt-check`、`cargo test -p timekeeper-backend --test admin_users_api -- --nocapture`、`cargo test -p timekeeper-backend --test user_repository -- --nocapture`、`cargo clippy -p timekeeper-backend --all-targets -- -D warnings` を green 確認。
- 2026-03-14: commit `08a586c` を push し、PR #449 `fix: revoke active sessions on admin MFA reset` を作成。ExecPlan を `done` へ移動した。
