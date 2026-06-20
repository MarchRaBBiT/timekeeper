# EP-20260620-work-schedule-master-design

## Goal

- Timekeeperに勤務体系マスタを導入するための、ドメイン・DB・API・既存勤怠接続の基準設計を定義する

## Scope

- In: 勤務体系の版管理、割り当て、日別解決、休日・部署・打刻との接続、実装フェーズ
- Out: 実装、給与計算、有休残高、月次締め、フレックス・変形労働の詳細規則

## Done Criteria (Observable)

- [x] 過去の再現性を保つ不変バージョンモデルが定義されている
- [x] 全社・部署・従業員・日別例外の優先順位が定義されている
- [x] 日勤・夜勤・祝日を解決できるデータモデルが定義されている
- [x] PostgreSQLテーブル、制約、API、認可、error codeが定義されている
- [x] 現行attendance / holidays / departmentsからの移行方針が定義されている
- [x] 実装フェーズとacceptance criteriaが定義されている

## Constraints / Non-goals

- PostgreSQL専用、Rust modular monolithのtarget boundaryへ合わせる
- 公開済み勤務体系と打刻済み日別予定は変更不能にする
- raw punchを勤務予定によって拒否または上書きしない
- 実装コードとmigrationは今回追加しない

## Task Breakdown

1. [x] 現行attendance、holiday、department、API契約を確認する
2. [x] domain modelとresolution ruleを定義する
3. [x] persistence、API、認可、監査を定義する
4. [x] migration strategy、delivery phases、test matrixを定義する
5. [x] docs validationとgit snapshotを記録する

## Validation Plan

- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `git diff --check`

## Git Snapshot Log

- [x] `git status --short`
- [x] docs validation pass
- [x] `git commit -m "docs: design work schedule master"`

## Progress Notes

- 2026-06-20: 現行の打刻、祝日、部署階層、rebuild target boundaryを確認した。
- 2026-06-20: 不変版、期間付き割当、日別projectionを中核とする勤務体系マスタ設計を作成した。
- 2026-06-20: `docs-check`、workspace fmt、workspace clippy、diff whitespace checkがgreen。
