# Coding Agent Guide

**Updated:** 2026-07-31
**Project:** Timekeeper - 勤怠管理システム

## Purpose

この文書は、Timekeeper でコーディングエージェントが守るべき共通規約をまとめたものです。
`AGENTS.md` はハーネス入口に留め、実装時の判断基準やレイヤー別の入り口はこの文書から参照します。

## Reading Order

実装前は次の順で確認します。

1. ルート `AGENTS.md`
2. この `docs/manual/CODING_AGENT.md`
3. 該当サブディレクトリの `AGENTS.md`
4. 複雑タスクなら `ExecPlan`
5. `1から作り直す` / `再設計` / `rebuild` 系の作業なら `docs/design-docs/rebuild-architecture.md`

## Architecture Modes

### Current Maintenance Mode

通常の issue / PR / bug fix では、現行 workspace を source of truth にします。

- Backend: `backend/`
- Frontend: `frontend/`
- API contract: `docs/design-docs/backend-api-catalog.md`
- Validation: `scripts/harness.sh`

既存 endpoint や画面の互換性を壊さず、変更 seam に近い test を追加・更新してください。

### Rebuild Mode

作り直し前提の設計・crate 分割・harness 再構築では、`docs/design-docs/rebuild-architecture.md` を target architecture とします。

優先する判断:

- PostgreSQL 専用。SQLite 互換を新しい制約にしない
- Rust modular monolith。初期段階では microservices にしない
- browser auth は opaque server-side session + HttpOnly Secure cookie を第一候補にする
- API DTO と OpenAPI を contract 境界で同期する
- handler / repository / frontend client の巨大化を避け、use case と feature 境界へ寄せる

## Task Routing

### Backend

- API / handler: `backend/src/handlers/`
- repository / SQL: `backend/src/repositories/`
- integration test: `backend/tests/`
- rebuild target use case: `crates/app/`
- rebuild target domain: `crates/domain/`
- rebuild target contract: `crates/contract/`

### Frontend

- page / MVVM: `frontend/src/pages/`
- API client: `frontend/src/api/`
- shared component: `frontend/src/components/`
- rebuild target frontend app: `apps/web/`
- rebuild target feature client: `apps/web/src/features/<feature>/api.rs` または contract-generated client

## Non-Negotiables

- ソースコード管理は `git` で行う
- 変更前後で relevant test を必ず回す
- 新規挙動は test 先行で固定する
- GitHub Issue / PR を作成する場合は `.github/ISSUE_TEMPLATE/` と `.github/PULL_REQUEST_TEMPLATE.md` を参照する
- SQLx migration は既存ファイル編集でなく新規追加
- 重い DB ロジックを handler に溜めない
- ユーザー許可なく仕様を簡略化しない
- backend API の route / method / request / response / error / 認可要件を変更した場合は、同じ変更内で `docs/design-docs/backend-api-catalog.md` を更新する
- docs / harness / architecture source of truth を変更した場合は、`bash scripts/harness.sh docs-check` を実行する

## 記述の責務

実装に関する情報は、次の責務に分けて記述します。

- プロダクションコードは、処理をどのように実現するか（**How**）を構造と命名で示す
- テストコードは、何が期待される挙動か（**What**）をテスト名・入力・期待値で示す
- コミットメッセージは、なぜ変更したか（**Why**）を示す
- コードコメントは、採用しなかった選択肢と、その理由（**Why not**）を示す

コードから明らかな処理内容をコメントで言い換えず、制約やトレードオフにより別案を採用しなかった理由を残してください。

## Notes

- 完了条件と検証順序はルート `AGENTS.md` と `scripts/harness.sh` を source of truth にする
- レイヤー固有の詳細規約は `backend/AGENTS.md`、`frontend/AGENTS.md`、`backend/tests/AGENTS.md` を参照する
