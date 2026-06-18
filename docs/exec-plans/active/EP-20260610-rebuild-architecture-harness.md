# EP-20260610-rebuild-architecture-harness

## Goal

- Timekeeper を 1 から作り直す場合の採用パッケージ、基本アーキテクチャ、移行順序を source of truth 化する
- `AGENTS.md` / manual / harness script が、その再構築方針を前提に agent を誘導できる状態にする

## Scope

- In:
  - root `AGENTS.md`
  - `docs/manual/CODING_AGENT.md`
  - `docs/manual/HARNESS.md`
  - `docs/design-docs/harness-engineering.md`
  - `docs/design-docs/rebuild-architecture.md`
  - `scripts/harness.sh`
  - `.agent/PLANS.md`
- Out:
  - 実アプリの crate 分割
  - Leptos / Axum / SQLx の version migration
  - DB migration 作成
  - live 環境 smoke

## Done Criteria (Observable)

- [x] 再構築アーキテクチャの package / boundary / migration strategy が `docs/design-docs/rebuild-architecture.md` に記録されている
- [x] root `AGENTS.md` が現行保守と rebuild work の source of truth を分けている
- [x] harness manual と `scripts/harness.sh --list` に docs/architecture 検証 stage がある
- [x] `.agent/PLANS.md` のテンプレートが git workflow と整合している
- [x] `bash scripts/harness.sh docs-check` が成功する
- [x] docs-only 変更として差分が確認され、必要なら commit できる状態になっている

## Constraints / Non-goals

- 現行 production code はこの計画では移動しない
- 既存 API 互換性は文書上の移行制約として扱う
- Redis / read replica / microservices を初期再構築の必須要件にしない
- user の未追跡ファイルや stash には触れない

## Task Breakdown

1. [x] 現行 harness と architecture docs を棚卸しする
2. [x] rebuild architecture doc を追加する
3. [x] `AGENTS.md` と manual docs を新しい source of truth に合わせる
4. [x] `scripts/harness.sh` に docs-check stage を追加する
5. [x] docs-check と軽量確認を実行する
6. [x] 結果を plan / final response に記録する

## Validation Plan

- [x] `bash scripts/harness.sh --list`
- [x] `bash scripts/harness.sh docs-check`
- [x] `bash -n scripts/harness.sh`
- [x] `git diff --check`
- [x] `git status --short`

## Git Snapshot Log

- [x] `git status --short`
- [x] docs-check pass
- [x] `git commit -m "docs: define rebuild architecture harness"`

## Progress Notes

- 2026-06-10: 現行 `AGENTS.md`, manual, harness script, backend/frontend architecture docs を確認。再構築方針は PostgreSQL 専用 + Rust modular monolith + Leptos 継続として source of truth 化する。
- 2026-06-10: `docs/design-docs/rebuild-architecture.md` を追加し、root/subdir AGENTS、HARNESS manual、harness engineering、`scripts/harness.sh` を docs-check stage に対応させた。`bash scripts/harness.sh --list`、`bash scripts/harness.sh docs-check`、`bash -n scripts/harness.sh`、`git diff --check` は成功。
- 2026-06-10: `git commit b958562 "docs: define rebuild architecture harness"` 作成。EP 完了。
