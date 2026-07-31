# EP-20260621-resolve-workday

## Goal

- 従業員と勤務日から、優先順位・公開版・曜日・祝日を合成した不変の日別勤務予定を解決し、PostgreSQLへ冪等に保存する

## Scope

- In: domain projection型、`ResolveWorkday` use case、依存port、PostgreSQL migration/repository、日別例外読み取り、部署階層、祝日合成、projection lock、unit/integration tests
- Out: 日別例外の管理API、予定一覧API、打刻との接続、未来一括生成、frontend、月次締め

## Done Criteria (Observable)

- [x] locked済みprojectionは再解決せず同じ内容を返す
- [x] `override > user > nearest department > ancestor department > organization` の順で決定する
- [x] 対象日のpublished版とISO曜日ルールを選択する
- [x] `non_working` と `follow_weekly_pattern` の祝日方針を正しく合成する
- [x] 日勤・夜勤・予定休憩をprojection子テーブルへ保存する
- [x] 未設定時は暗黙の標準勤務を作らず `WorkScheduleNotConfigured` を返す
- [x] 同時または反復実行で `(user_id, work_date)` が重複せず、unlockedだけを更新する
- [x] 新規モジュールのline coverageが80%以上である

## Constraints / Non-goals

- resolver規則は`crates/app`、SQLは`crates/infra-postgres`へ置く
- 現行backend migrationを追加し、既存migrationは変更しない
- すべてのSQL入力をbind parameterで渡す
- locked projectionはapplicationとDB triggerの両方で不変にする
- 過去日の部署所属は履歴化されていないため、locked projectionを唯一の再現性境界とする

## Task Breakdown

1. [x] app unit testsとPostgreSQL integration testsをREDにする
2. [x] domain projection型とapp resolver ports/use caseを実装する
3. [x] migration 044とPostgreSQL repositoryを実装する
4. [x] priority、holiday、overnight、lock、idempotencyをgreenにする
5. [x] coverage、fmt、clippy、harness、security reviewを実行する
6. [x] docs statusとExecPlanを更新する
7. [x] git snapshotを作成する

## Validation Plan

- [x] `cargo test -p timekeeper-app --test resolve_workday` — 7 passed
- [x] `cargo test -p timekeeper-infra-postgres --test workday_resolver_repository` — 2 passed
- [x] `cargo test -p timekeeper-backend --test workday_resolver` — 5 passed（resolver integration 3 + support 2）
- [x] `bash scripts/harness.sh backend-unit` — 371 passed
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] new-module line coverage >= 80% — app 94.67%、infra-postgres合算86.09%

## Git Snapshot Log

- [x] `git status --short`
- [x] focused tests pass
- [x] `git commit -m "feat: resolve daily work schedules"`

## Progress Notes

- 2026-06-21: 管理API実装済みの勤務体系マスタを入力とし、resolver本体とprojection永続化を今回の境界に固定した。
- 2026-06-21: app unit testsをRED→GREENとし、locked再利用、全優先順位、祝日2方針、override、未設定を検証した。
- 2026-06-21: migration 044とPostgreSQL adapterを追加し、夜勤・休憩snapshot、unlocked置換、冪等性、locked親子triggerを実DBで検証した。
- 2026-06-21: security reviewでbind parameter、UUID検証、generic error、secret非混入、locked競合処理を確認した。HTTP認証・CSRF・rate limitはAPIを追加しないため適用外。
- 2026-06-21: app全テスト、infra-postgres全テスト、resolver実DB 5 tests、backend unit 371 tests、workspace clippy、fmt、docs-checkがgreen。
- 2026-06-21: line coverageは`ResolveWorkday` 94.67%、PostgreSQL adapter 86.09%（291/338）を計測した。
