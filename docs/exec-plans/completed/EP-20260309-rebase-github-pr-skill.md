# EP-20260309-rebase-github-pr-skill

## Goal
- GitHub PR 番号を受けて、`gh` と `jj` を使って PR ブランチの rebase を進める専用 agent/skill を追加する

## Scope
- In: `.codex/skills/rebase-github-prs/*`, `docs/generated/exec-plans/active/*`
- Out: 実際の PR rebase 実行、GitHub Actions 追加、gh app 設定変更

## Done Criteria (Observable)
- [x] PR 番号で起動すべき skill が追加されている
- [x] batch helper script が同梱され、複数 PR 番号を順に処理できる
- [x] same-repo PR と fork PR の両方を扱う手順が skill に書かれている
- [x] 追加した script の構文検証が成功する
- [x] conflict を伴う rebase で helper script が成功扱いせず、非ゼロ終了と次の確認コマンドを返す

## Constraints / Non-goals
- 既存 repo ルールに従い、バージョン管理操作は `jj` を優先する
- remote 更新は `--push` 指定時のみ helper script が行う
- conflict 解消自体は agent が対話的に進める前提で、script は orchestration を担う

## Task Breakdown
1. [x] skill 名・トリガー条件・ワークフローを定義する
2. [x] `gh` + `jj` ベースの batch helper script を実装する
3. [x] `agents/openai.yaml` を追加する
4. [x] `bash -n` などの最小検証を行う
5. [x] PR #355 を使って conflict 検知の実地検証を行い、helper script の誤成功を修正する

## Validation Plan
- [x] `bash -n .codex/skills/rebase-github-prs/scripts/rebase_prs.sh`
- [x] `bash .codex/skills/rebase-github-prs/scripts/rebase_prs.sh --help`
- [x] `sed -n '1,200p' .codex/skills/rebase-github-prs/SKILL.md`
- [x] `bash .codex/skills/rebase-github-prs/scripts/rebase_prs.sh --allow-dirty 355`

## JJ Snapshot Log
- [ ] `jj status`
- [x] skill validation pass
- [ ] `jj commit -m "feat(skill): add github pr rebase agent"`

## Progress Notes
- 2026-03-09: 計画作成。batch で複数 PR を扱うため、PR metadata 解決・bookmark alias・optional push を helper script に寄せる方針を採用。
- 2026-03-09: `.codex/skills/rebase-github-prs/` を追加し、`SKILL.md`、`agents/openai.yaml`、`scripts/rebase_prs.sh` を実装。`gh pr view` で metadata を解決し、`jj bookmark set` と `jj rebase`、`jj git push --named` を組み合わせるワークフローを採用。
- 2026-03-09: `bash -n .codex/skills/rebase-github-prs/scripts/rebase_prs.sh`、`bash .codex/skills/rebase-github-prs/scripts/rebase_prs.sh --help` を実行し、構文と CLI help を確認。
- 2026-03-09: conflict 発生時の自動停止だけでは不十分だったため、`SKILL.md` に intent-based conflict resolution workflow を追加し、`references/conflict-resolution.md` に判断基準を切り出した。
- 2026-03-09: PR #355 を使った実地検証で、`jj rebase` が conflict commit を作っても helper script が成功扱いしてしまう不備を確認。`pr-<number>` bookmark の `(conflict)` 表示を検知して非ゼロ終了し、`gh pr view`、`gh pr diff`、`jj resolve --list` など次の確認コマンドを返すよう修正した。
