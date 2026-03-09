# EP-20260306-auth-security-topics

## Goal
- backend 認証フローのレビュー指摘 4 件を、それぞれ独立した topic branch で修正する

## Scope
- In: `backend/src/handlers/auth.rs`, `backend/src/repositories/password_reset.rs`, `backend/src/models/user.rs`, `backend/src/utils/security.rs`, `backend/src/middleware/auth.rs`, `backend/tests/*`, `.agent/PLANS.md`
- Out: frontend 変更、既存 `.takt/.gitignore` 変更、インフラ設定の本番反映

## Done Criteria (Observable)
- [x] パスワードリセット token が単回利用になり、旧 token を無効化できる
- [x] メール変更時に再認証が必要になる
- [x] Cookie ベース認証の状態変更 API で CSRF 防御が統一される
- [x] ログインで未知ユーザーと既知ユーザーの処理差が緩和される
- [x] 各修正が独立した `jj` topic branch / snapshot として分離されている

## Constraints / Non-goals
- 既存の [`.takt/.gitignore`](/home/mrabbit/Documents/timekeeper/.takt/.gitignore) には触れない
- 各修正は `@-` を起点にした別 workspace で行い、ユーザーの未コミット変更を混ぜない
- SQLx migration 変更は必要な場合のみ新規追加で対応する

## Task Breakdown
1. [x] `jj workspace` を 4 つ作成し、各 topic branch の起点を分離する
2. [x] Finding 1: password reset の単回利用保証と旧 token 無効化を実装する
3. [x] Finding 2: メール変更に current password 再認証を導入する
4. [x] Finding 3: refresh/logout/session 操作に Origin 検証を統一適用する
5. [x] Finding 4: ダミーハッシュでログインのタイミング差を緩和する
6. [x] 各 workspace で関連テストを実行し、成功ごとに `jj commit` する

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --test password_reset_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test auth_api -- --nocapture`
- [x] 必要に応じて追加の unit/integration test を実行する

## JJ Snapshot Log
- [x] `jj status`
- [x] topic 1 tests pass
- [x] `jj commit -m "fix(auth): harden password reset token lifecycle"`
- [x] topic 2 tests pass
- [x] `jj commit -m "fix(auth): require re-authentication for email changes"`
- [x] topic 3 tests pass
- [x] `jj commit -m "fix(auth): enforce origin checks on cookie auth actions"`
- [x] topic 4 tests pass
- [x] `jj commit -m "fix(auth): reduce login timing side-channel"`

## Progress Notes
- 2026-03-06: 計画作成。既存 `.takt/.gitignore` 変更を避けるため、`jj workspace` で 4 件を分離して対応する方針に決定。
- 2026-03-06: `topic/auth-reset-token-lifecycle` を `75b3060d` に配置。`password_reset_api` は 9 passed。未使用 token の事前失効と、token 消費の原子的更新で単回利用を担保。
- 2026-03-06: `topic/auth-email-reauth` を `7e10dea6` に配置。`auth_flow_api` と `user_update_api` が成功。メール変更時のみ `current_password` を必須化。
- 2026-03-06: `topic/auth-origin-checks` を `f96bdcea` に配置。`verify_origin_if_cookie_present` の unit test、`session_api`、`auth_flow_api` が成功。Cookie 認証の状態変更 API に Origin 検証を統一適用。
- 2026-03-06: `topic/auth-login-timing` を `422b56aa` に配置。`auth_flow_api` 17 passed、`auth_api` 5 passed、`verify_missing_user_login_returns_unauthorized` も成功。未知ユーザー時もダミー Argon2 hash を検証してタイミング差を緩和。

