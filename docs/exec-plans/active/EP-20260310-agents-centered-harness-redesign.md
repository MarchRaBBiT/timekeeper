## ExecPlan: AGENTS-Centered Harness Redesign

## Goal

`AGENTS.md` をハーネスの入口として再設計し、詳細手順と実行入口を `docs/` と `scripts/harness.sh` に分離する。

## Done Criteria

- [x] `AGENTS.md` が harness-first の入口へ再設計される
- [x] `docs/manual/HARNESS.md` に実行手順が追加される
- [x] `docs/design-docs/harness-engineering.md` に設計意図が追加される
- [x] `scripts/harness.sh` が共通入口として追加される
- [x] `bash -n scripts/harness.sh` が成功する
- [x] `bash scripts/harness.sh --list` が成功する
- [x] `bash scripts/harness.sh doctor` が成功する
- [ ] `jj` snapshot を作成する

## Validation

- [x] `bash -n scripts/harness.sh`
- [x] `bash scripts/harness.sh --list`
- [x] `bash scripts/harness.sh doctor`

## Progress Log

- [x] 現状の `AGENTS.md` / docs / scripts を確認
- [x] AGENTS-first の情報分割を設計
- [x] docs / script を追加
- [x] harness 入口を検証
- [ ] snapshot 作成
