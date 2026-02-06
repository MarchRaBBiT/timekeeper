# Exec Plans

複雑タスクのみ、このファイルに実行計画を作成する。  
小規模修正では計画作成は任意。

## 運用ルール
- 複数レイヤー横断（`backend` + `frontend` + `e2e` など）の変更は原則 `ExecPlan` を作成する
- 完了条件は「確認可能な挙動」で定義する（例: APIレスポンス、画面表示、テスト成功）
- 実装中はチェックボックスを更新し、未完了の作業を残す
- テスト成功の節目ごとに `jj` スナップショットを残す

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
- [ ] `pwsh -File .\scripts\test_backend.ps1`（必要時）
- [ ] `cd frontend; wasm-pack test --headless --firefox`（必要時）
- [ ] `cd e2e; node run.mjs`（必要時）

## JJ Snapshot Log
- [ ] `jj status`
- [ ] <対象テスト> pass
- [ ] `jj commit -m "chore(test): snapshot after <test_target> pass"`

## Progress Notes
- YYYY-MM-DD: <実施内容>
```
