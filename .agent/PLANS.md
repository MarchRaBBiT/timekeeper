# Exec Plans

複雑タスクの計画テンプレートと運用ルールだけをこのファイルに置く。  
実際の計画は 1 タスク 1 ファイルで `docs/generated/exec-plans/active/` または `docs/generated/exec-plans/completed/` に保存する。

## 配置ルール
- 実行中の計画: `docs/generated/exec-plans/active/EP-YYYYMMDD-<short-slug>.md`
- 完了済み計画: `docs/generated/exec-plans/completed/EP-YYYYMMDD-<short-slug>.md`
- 完了したら同名ファイルを `active` から `completed` へ移す
- 1 ファイルには 1 つの ExecPlan だけを置く

## 運用ルール
- 複数レイヤー横断（`backend` + `frontend` + `e2e` など）の変更は原則 `ExecPlan` を作成する
- 完了条件は「確認可能な挙動」で定義する（例: APIレスポンス、画面表示、テスト成功）
- 実装中はチェックボックスを更新し、未完了の作業を残す
- テスト成功の節目ごとに `jj` スナップショットを残す
- 依頼時は対象 plan ファイルを明示し、「最後の完了条件まで一気に実施する」と指示する

## テンプレート

```md
# EP-YYYYMMDD-<short-slug>

## Goal
- <このタスクで達成すること>

## Scope
- In: <対象>
- Out: <対象外>

## Done Criteria (Observable)
- [ ] <確認可能な完了条件1>
- [ ] <確認可能な完了条件2>

## Constraints / Non-goals
- <制約や今回やらないこと>

## Task Breakdown
1. [ ] <実装タスク1>
2. [ ] <実装タスク2>
3. [ ] <実装タスク3>

## Validation Plan
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `./scripts/test_backend_integrated.sh` または `cargo test --test <target>`
- [ ] `pwsh -File .\\scripts\\test_backend.ps1`（必要時）
- [ ] `cd frontend; wasm-pack test --headless --firefox`（必要時）
- [ ] `cd e2e; node run.mjs`（必要時）

## JJ Snapshot Log
- [ ] `jj status`
- [ ] <対象テスト> pass
- [ ] `jj commit -m "<message>"`

## Progress Notes
- YYYY-MM-DD: <実施内容>
```
