# EP-20260314-wildcard-cors-hardening

## Goal
- credentials 付き CORS と origin 検証を fail-closed にし、wildcard 設定の危険な抜け道をなくす

## Scope
- In: `backend/src/main.rs`, `backend/src/config.rs`, `backend/src/utils/security.rs`, 関連 config / startup tests, 必要に応じて docs
- Out: CORS を使わない構成への全面移行、infra レベルの ALB/CDN 設定変更

## Done Criteria (Observable)
- [ ] `allow_credentials(true)` を使う構成で `CORS_ALLOW_ORIGINS="*"` が安全側で拒否される
- [ ] `verify_request_origin` が wildcard を許容しない
- [ ] `PRODUCTION_MODE` の値に依存せず、危険な CORS 組み合わせが起動時に弾かれる
- [ ] 設定異常時のエラーが test で固定されている

## Constraints / Non-goals
- local 開発で必要な cross-origin は明示 allowlist で成立させる
- 既存の正当な origin allowlist は壊さない
- CORS 緩和を必要とする将来要件があれば、明示設計として別計画に切り出す

## Task Breakdown
1. [ ] 現行 CORS 初期化と `verify_request_origin` の wildcard 分岐を整理する
2. [ ] credentials + wildcard を常時拒否する validation を config/startup に追加する
3. [ ] `verify_request_origin` から wildcard 許容を除去し、allowlist のみ許可する
4. [ ] config / startup / security helper の focused test を追加する

## Validation Plan
- [ ] `bash scripts/harness.sh fmt-check`
- [ ] `cargo test -p timekeeper-backend --test config_api -- --nocapture`
- [ ] `cargo test -p timekeeper-backend --lib security`
- [ ] `bash scripts/harness.sh backend-unit`
- [ ] `bash scripts/harness.sh lint`

## Git Checkpoint Log
- [ ] `git status --short`
- [ ] CORS/config focused tests pass
- [ ] `git commit -m "fix(security): fail closed on wildcard cors with credentials"`

## Progress Notes
- 2026-03-14: wildcard CORS finding を config / startup hardening の独立 ExecPlan として作成。
