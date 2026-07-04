# リポジトリ ガイドライン

**Updated:** 2026-06-10
**Project:** Timekeeper - 勤怠管理システム

## Purpose

この `AGENTS.md` は repo のハーネス入口です。
長い実装方針や運用手順を 1 ファイルに詰め込まず、以下の 3 つだけを先に決めます。

1. どこを source of truth にするか
2. どの順で検証を積み上げるか
3. 完了を何で判定するか

詳細な運用は次を参照します。

- コーディングエージェント共通規約: [docs/manual/CODING_AGENT.md](./docs/manual/CODING_AGENT.md)
- ハーネス利用手順: [docs/manual/HARNESS.md](./docs/manual/HARNESS.md)
- ハーネス設計意図: [docs/design-docs/harness-engineering.md](./docs/design-docs/harness-engineering.md)
- 1から作り直す場合の再構築方針: [docs/design-docs/rebuild-architecture.md](./docs/design-docs/rebuild-architecture.md)
- 複雑タスク計画: [.agent/PLANS.md](./.agent/PLANS.md)

## Source Of Truth

- ハーネス入口と完了判定: この `AGENTS.md`
- コーディングエージェント共通規約: `docs/manual/CODING_AGENT.md`
- 長時間・複数レイヤー作業の進捗: `ExecPlan`
- 実行可能な検証入口: `scripts/harness.sh`
- 現行 backend API 契約一覧: `docs/design-docs/backend-api-catalog.md`
- 再構築 target architecture: `docs/design-docs/rebuild-architecture.md`
- 再構築 implementation plan: `docs/exec-plans/active/EP-20260610-rebuild-architecture-harness.md`
- 各レイヤーの詳細規約:
  - [backend/AGENTS.md](./backend/AGENTS.md)
  - [frontend/AGENTS.md](./frontend/AGENTS.md)
  - [backend/tests/AGENTS.md](./backend/tests/AGENTS.md)

## Architecture Direction

通常の bug fix / feature work は現行構成を尊重します。
ただし、`1から作り直す`, `再設計`, `rebuild`, `architecture reset`, `harness 再構築` のような作業では、次の方針を優先します。

- Rust modular monolith を基本形にする
- PostgreSQL 専用にする。SQLite 互換を新しい前提にしない
- browser 認証は opaque server-side session + HttpOnly Secure cookie を第一候補にする
- JWT は外部 API / service token 用に限定する
- Redis / read replica / queue は初期必須ではなく、運用上の根拠が出た段階で追加する
- API DTO / OpenAPI / frontend client は contract 境界で同期する
- 巨大 handler / global API client / 巨大 view_model を増やさず、use case と feature 境界へ分解する

## Harness Loop

作業は必ず次の順で進めます。

1. `AGENTS.md`、`docs/manual/CODING_AGENT.md`、該当サブディレクトリの `AGENTS.md` を確認する
2. 複雑タスクなら `ExecPlan` を作る。再構築作業では `docs/design-docs/rebuild-architecture.md` も確認する
3. 最小の再現/検証を先に作る
4. 実装する
5. 変更 seam に近い harness stage から順に通す
6. green になった節目で `git commit` を作成する
7. issue / PR / plan を実測値で更新する

## Validation Ladder

重い検証を最初から回さず、下から順に積み上げます。

1. `doctor`
   - 必須コマンド、依存ツール、live URL 前提を確認
2. `docs-check`
   - ハーネスと再構築 source of truth の存在・整合を確認
3. `fmt-check`
   - `cargo fmt --all --check`
4. `backend-unit`
   - `cargo test -p timekeeper-backend --lib`
5. `backend-integration`
   - `cargo test -p timekeeper-backend --tests`
6. `backend-security-smoke`
   - auth / lockout / rate-limit / password / mfa / session の focused `cargo test`（詳細: [docs/manual/HARNESS.md](./docs/manual/HARNESS.md)）
7. `clippy-backend`
   - `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`
8. `clippy-frontend`
   - `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings`
9. `lint`
   - `docs-check + fmt-check + clippy-backend + clippy-frontend`
10. `api-smoke`
   - live backend に対する API スモーク
11. `frontend-login`
   - live frontend に対する Playwright login smoke
12. `full`
   - 上記を束ねた統合実行

共通入口:

```bash
bash scripts/harness.sh --list
bash scripts/harness.sh doctor
bash scripts/harness.sh docs-check
bash scripts/harness.sh fmt-check
bash scripts/harness.sh backend-unit
bash scripts/harness.sh backend-security-smoke
bash scripts/harness.sh lint
bash scripts/harness.sh smoke
bash scripts/harness.sh full
```

## Completion Rule

完了とみなす条件は次です。

- 変更対象の期待挙動を示す test がある
- 変更 seam に対応する harness stage が green
- docs / harness / architecture 変更では `docs-check` が green
- `cargo fmt --all --check` が通る
- `cargo clippy --all-targets -- -D warnings` が通る
- 関連 issue / PR / ExecPlan に実測結果が残っている
- 対応する `git` commit が作成されている

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
