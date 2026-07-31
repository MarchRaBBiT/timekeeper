# EP-20260620-work-schedule-master-api

## Goal

- 勤務体系マスタ、公開版、曜日別勤務区間・予定休憩、期間付き割り当てを、認証済み管理APIとして利用可能にする

## Scope

- In: domain validation、shared DTO、PostgreSQL migration/repository、管理API、認可、監査分類、OpenAPI/API catalog
- Out: 日別勤務予定resolver、打刻との接続、日別例外、frontend管理画面、月次締め、給与計算

## Done Criteria (Observable)

- [x] system adminが勤務体系を作成・更新・廃止できる
- [x] managerが勤務体系と公開版を参照でき、employeeは管理APIを参照できない
- [x] system adminが7曜日を持つdraft版を作成・置換・公開・削除できる
- [x] 公開版は変更・削除できず、有効期間の重複を拒否する
- [x] 全社・部署・従業員への期間付き割り当てを作成・一覧・削除できる
- [x] 同じ割り当て先の期間重複を拒否する
- [x] API入力、認可、SQL、エラー本文がsecurity checklistを満たす
- [x] API catalogとOpenAPIが実装に同期している

## Constraints / Non-goals

- 現行backendを動作入口にし、domainとcontractはrebuild target cratesへ置く
- PostgreSQL専用とし、既存migrationは変更せず新規migrationを追加する
- raw SQLはrepositoryに限定し、すべてbind parameterを使用する
- published versionはapplicationとDBの両方で不変にする
- API mutationは既存CSRF、rate limit、system admin middleware配下へ置く

## Task Breakdown

1. [x] domain / contract / API integration testsをREDにする
2. [x] domain modelとcontract DTOを実装する
3. [x] migrationとrepositoryを実装する
4. [x] handler、route、error mapping、audit classificationを実装する
5. [x] API catalogとOpenAPIを更新する
6. [x] focused tests、coverage計測、lint、security reviewを実行する
7. [x] git snapshotを作成する

## Validation Plan

- [x] `cargo test -p timekeeper-domain --test work_schedule` — 7 passed
- [x] `cargo test -p timekeeper-contract --test work_schedule_contract` — 3 passed
- [x] `cargo test -p timekeeper-backend --test admin_work_schedules_api -- --nocapture` — 11 passed
- [x] `cargo test -p timekeeper-backend --lib work_schedule` — 3 passed
- [x] `bash scripts/harness.sh backend-unit` — 371 passed
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] coverage report for new modules is at least 80% — domain 89.76%、backend新規モジュール合算85.39%（line coverage）

## Git Snapshot Log

- [x] `git status --short`
- [x] focused tests pass
- [x] `git commit -m "feat: implement work schedule master api"`

## Progress Notes

- 2026-06-20: Phase 1からmaster/version/assignment管理APIを今回の実装範囲として固定した。
- 2026-06-20: domain、contract、migration 043、repository、管理API、監査分類、OpenAPIを実装した。
- 2026-06-20: domain 7 tests、contract 3 tests、backend work schedule API 11 testsがgreen。
- 2026-06-20: coverage toolは導入できたが、`llvm-tools-preview`追加が承認上限で拒否された。新規挙動はunit/integrationの両方で網羅した。
- 2026-06-20: security reviewでsystem-admin write、manager read、CSRF/rate-limit適用、bind parameter SQL、generic error、監査分類を確認した。
- 2026-06-20: `cargo test -p timekeeper-backend --tests`は既存`admin_requests_api`の部署認可期待値3件（実際403）で停止。勤務体系focused suiteは再実行して11件green。
- 2026-06-20: `git add`がCodex approval usage limitにより拒否されたため、実装差分をworktreeに保持した。
- 2026-06-21: `llvm-tools-preview`を追加し、line coverageを計測した。domain新規モジュールは89.76%、backend新規モジュール合算は85.39%。contractはデータ型定義のみのため、wire formatとround-tripの3 testsで検証した。
- 2026-06-21: 全focused tests、backend unit、docs-check、fmt、workspace clippy、security reviewがgreenの状態でgit snapshotを作成した。
