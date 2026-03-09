# EP-20260309-harness-engineering-foundation

## Goal
- OpenAI の harness engineering 記事で強調されている「短い実行入口」「repo 内の system of record」「可観測な検証フィードバック」を、このリポジトリの検証フローに適用する

## Scope
- In: `scripts/*`, `AGENTS.md`, `docs/manual/*`, `docs/design-docs/*`, `docs/generated/exec-plans/*`
- Out: CI サービス新設、クラウド observability 導入、E2E シナリオ全面書き換え

## Done Criteria (Observable)
- [x] 単一コマンドで検証プロファイルを選んで実行できるハーネス入口がある
- [x] doctor で前提コマンド・必要 URL・依存関係の欠落を早期に検出できる
- [x] ハーネスの使い方と設計意図が repo 内ドキュメントとして参照できる
- [x] 既存の backend/API/E2E スクリプトを再利用しつつ、関連する軽量検証が成功する

## Constraints / Non-goals
- 既存テストスクリプトの意味論は変えず、ハーネス層で束ねる
- 環境固有の起動責務（Podman、PowerShell、WASM browser install）は明示するが自動化しすぎない
- observability stack の新規導入までは行わず、まずは agent が使える repo-local な検証入口を整える

## Task Breakdown
1. [x] 現行テスト/スモーク/E2E 入口の整理とハーネス設計の文書化
2. [x] `scripts/harness.sh` を追加し、stage/profiles/doctor/summary を実装
3. [x] `AGENTS.md` と docs に新ハーネス入口を反映
4. [x] `doctor` と軽量ステージを実行して回帰確認

## Validation Plan
- [x] `bash scripts/harness.sh doctor`
- [x] `bash scripts/harness.sh backend-unit`
- [x] `bash scripts/harness.sh --list`
- [x] `bash scripts/harness.sh`
- [x] `cargo fmt --all`

## JJ Snapshot Log
- [x] `jj status`
- [x] harness validation pass
- [ ] `jj commit -m "feat(harness): add unified validation entrypoint"`

## Progress Notes
- 2026-03-09: 計画作成。OpenAI の harness engineering 記事に合わせ、まずは repo-local な単一入口とドキュメント化を優先する方針に決定。
- 2026-03-09: `scripts/harness.sh` を追加し、`agent-fast` / `smoke` / `full` の profile、`doctor` preflight、stage summary を実装。`scripts/test_frontend_login.mjs` も `e2e/` 配下の Playwright を安定して解決するよう修正。
- 2026-03-09: `docs/manual/HARNESS.md` と `docs/design-docs/harness-engineering.md` を追加し、`AGENTS.md` のコマンド入口と ExecPlan 運用先を更新。
- 2026-03-09: `bash scripts/harness.sh doctor`、`bash scripts/harness.sh backend-unit`、`bash scripts/harness.sh --list`、`bash scripts/harness.sh`、`cargo fmt --all` を実行。`backend-unit` は 442 tests passed。
- 2026-03-09: `jj status` を確認。working copy に先行していた docs 再編差分も同居しているため、今回のハーネス変更だけを切り出した snapshot はまだ作成していない。
