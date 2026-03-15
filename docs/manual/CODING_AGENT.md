# Coding Agent Guide

**Generated:** 2026-03-15  
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

## Task Routing

### Backend

- API / handler: `backend/src/handlers/`
- repository / SQL: `backend/src/repositories/`
- integration test: `backend/tests/`

### Frontend

- page / MVVM: `frontend/src/pages/`
- API client: `frontend/src/api/`
- shared component: `frontend/src/components/`

## Non-Negotiables

- ソースコード管理は `git` で行う
- 変更前後で relevant test を必ず回す
- 新規挙動は test 先行で固定する
- GitHub Issue / PR を作成する場合は `.github/ISSUE_TEMPLATE/` と `.github/PULL_REQUEST_TEMPLATE.md` を参照する
- SQLx migration は既存ファイル編集でなく新規追加
- 重い DB ロジックを handler に溜めない
- ユーザー許可なく仕様を簡略化しない
- backend API の route / method / request / response / error / 認可要件を変更した場合は、同じ変更内で `docs/design-docs/backend-api-catalog.md` を更新する

## Notes

- 完了条件と検証順序はルート `AGENTS.md` と `scripts/harness.sh` を source of truth にする
- レイヤー固有の詳細規約は `backend/AGENTS.md`、`frontend/AGENTS.md`、`backend/tests/AGENTS.md` を参照する
