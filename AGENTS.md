# リポジトリ ガイドライン

**Generated:** 2026-03-10  
**Project:** Timekeeper - 勤怠管理システム

## Purpose

この `AGENTS.md` は repo のハーネス入口です。  
長い実装方針や運用手順を 1 ファイルに詰め込まず、以下の 3 つだけを先に決めます。

1. どこを source of truth にするか
2. どの順で検証を積み上げるか
3. 完了を何で判定するか

詳細な運用は次を参照します。

- ハーネス利用手順: [docs/manual/HARNESS.md](./docs/manual/HARNESS.md)
- ハーネス設計意図: [docs/design-docs/harness-engineering.md](./docs/design-docs/harness-engineering.md)
- 複雑タスク計画: [.agent/PLANS.md](./.agent/PLANS.md)

## Source Of Truth

- 実装制約と作業順序: この `AGENTS.md`
- 長時間・複数レイヤー作業の進捗: `ExecPlan`
- 実行可能な検証入口: `scripts/harness.sh`
- 現行 backend API 契約一覧: `docs/design-docs/backend-api-catalog.md`
- 各レイヤーの詳細規約:
  - [backend/AGENTS.md](./backend/AGENTS.md)
  - [frontend/AGENTS.md](./frontend/AGENTS.md)
  - [backend/tests/AGENTS.md](./backend/tests/AGENTS.md)

## Harness Loop

作業は必ず次の順で進めます。

1. `AGENTS.md` と該当サブディレクトリの `AGENTS.md` を確認する
2. 複雑タスクなら `ExecPlan` を作る
3. 最小の再現/検証を先に作る
4. 実装する
5. 変更 seam に近い harness stage から順に通す
6. green になった節目で `git commit` を作成する
7. issue / PR / plan を実測値で更新する

## Validation Ladder

重い検証を最初から回さず、下から順に積み上げます。

1. `doctor`
   - 必須コマンド、依存ツール、live URL 前提を確認
2. `fmt-check`
   - `cargo fmt --all --check`
3. `backend-unit`
   - `cargo test -p timekeeper-backend --lib`
4. `backend-integration`
   - `cargo test -p timekeeper-backend --tests`
5. `clippy-backend`
   - `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`
6. `clippy-frontend`
   - `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings`
7. `lint`
   - `fmt-check + clippy-backend + clippy-frontend`
8. `api-smoke`
   - live backend に対する API スモーク
9. `frontend-login`
   - live frontend に対する Playwright login smoke
10. `full`
   - 上記を束ねた統合実行

共通入口:

```bash
bash scripts/harness.sh --list
bash scripts/harness.sh doctor
bash scripts/harness.sh fmt-check
bash scripts/harness.sh backend-unit
bash scripts/harness.sh lint
bash scripts/harness.sh smoke
bash scripts/harness.sh full
```

## Completion Rule

完了とみなす条件は次です。

- 変更対象の期待挙動を示す test がある
- 変更 seam に対応する harness stage が green
- `cargo fmt --all --check` が通る
- `cargo clippy --all-targets -- -D warnings` が通る
- 関連 issue / PR / ExecPlan に実測結果が残っている
- 対応する `git` commit が作成されている

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
- SQLx migration は既存ファイル編集でなく新規追加
- 重い DB ロジックを handler に溜めない
- ユーザー許可なく仕様を簡略化しない
- backend API の route / method / request / response / error / 認可要件を変更した場合は、同じ変更内で `docs/design-docs/backend-api-catalog.md` を更新する

## Command Quick Reference

```bash
# ハーネス入口
bash scripts/harness.sh --list
bash scripts/harness.sh lint

# backend
cd backend && cargo run

# frontend
pwsh -File .\\scripts\\frontend.ps1 start

# focused backend integration
./scripts/test_backend_integrated.sh

# live API smoke
./scripts/test_backend.sh
```
