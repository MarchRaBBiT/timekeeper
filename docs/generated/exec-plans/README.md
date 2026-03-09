# Exec Plans

ExecPlan は 1 タスク 1 ファイルで管理する。

## Directories
- `active/`: 実行中で、未完了チェックが残っている計画
- `completed/`: 完了済みで、記録として保持する計画

## Naming
- `EP-YYYYMMDD-<short-slug>.md`

## Workflow
1. 新しい複雑タスクを始めるときは `.agent/PLANS.md` のテンプレートから `active/` に新規作成する
2. 実装中は対象ファイルのチェックボックスと `Progress Notes` を更新する
3. 完了したら同名ファイルを `completed/` に移す
