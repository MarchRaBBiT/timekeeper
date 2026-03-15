# EP-20260314-cookie-csrf-protection-unification

## Goal
- cookie-authenticated mutation endpoint に対する CSRF 防御を一貫したルールへ統一する

## Scope
- In: `backend/src/utils/security.rs`, `backend/src/handlers/auth.rs`, 他の cookie-authenticated handlers, 関連 integration tests, 必要に応じて `docs/design-docs/backend-api-catalog.md`
- Out: SPA 以外のクライアント全面刷新、Bearer-only への全面移行、CSRF token 基盤の大規模新設

## Done Criteria (Observable)
- [ ] cookie-authenticated な `POST` / `PUT` / `DELETE` endpoint の保護対象一覧が固定されている
- [ ] 対象 endpoint で `Origin` / `Referer` 検証が共通化され、未検証 endpoint が残らない
- [ ] 許可 origin / 不許可 origin / header 欠落時の挙動を test で確認できる
- [ ] 認可要件または error 応答が変わる API があれば backend API catalog に反映されている

## Constraints / Non-goals
- 既存クライアント互換を保ちつつ最小差分で入れる
- まずは cookie auth 経由の mutation を優先し、read-only endpoint には広げない
- `SameSite` は補助策として維持し、主防御を header 検証に寄せる

## Task Breakdown
1. [ ] cookie auth が有効な mutation endpoint を棚卸しし、保護対象をリスト化する
2. [ ] 共通 middleware か helper で Origin/Referer 検証を統一適用する
3. [ ] auth/logout/profile/sessions/admin mutations の focused integration test を追加する
4. [ ] 必要に応じて backend API catalog の auth requirement / error section を更新する

## Validation Plan
- [ ] `bash scripts/harness.sh fmt-check`
- [ ] `cargo test -p timekeeper-backend --test auth_flow_api -- --nocapture`
- [ ] `cargo test -p timekeeper-backend --test session_api -- --nocapture`
- [ ] 関連 admin/request handler を cover する focused backend tests
- [ ] `bash scripts/harness.sh backend-integration`
- [ ] `bash scripts/harness.sh lint`

## Git Checkpoint Log
- [ ] `git status --short`
- [ ] CSRF focused tests pass
- [ ] `git commit -m "fix(security): unify csrf checks for cookie-authenticated mutations"`

## Progress Notes
- 2026-03-14: CSRF finding を個別対応できるよう、cookie-authenticated mutation の統一保護計画を作成。
