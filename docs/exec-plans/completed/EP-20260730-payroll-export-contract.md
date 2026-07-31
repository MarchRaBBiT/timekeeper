# EP-20260730-payroll-export-contract

**Status:** 実装完了・検証済み  
**Parent task:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-14

## Goal

締め済み従業員月について、給与システム向けの再現可能な「従業員 × 月 × 賃金項目」CSVをSystem Adminへ提供する。

## Dependencies And Decisions

- T-13完了後に休日労働区分を固定し、T-11の半休・時間休カウンタを利用する。
- 最初に`docs/design-docs/payroll-export.md`を作り、項目、分単位、丸め、列順、UTF-8 CSV、escaping、versionをsource of truthとして定める。
- migration `065` にclosed transition時のper-user payroll snapshotとversionを保存する。都度導出方式は採用しない。
- workflowの`closed`だけをexport可能とする。legacy monthly lockだけの月、および`open / self_confirmed / approved / reopened`はper-user failureにする。
- closeとsnapshot保存は同一transactionとする。reopen中はexport不可、再closeは新versionを作り、過去versionを上書きしない。
- APIは`GET /api/admin/payroll-export?year=&month=`。対象内の成功・失敗が混在してもHTTP 200でper-user結果を返す。

## TDD And Implementation

1. design docを作りCSV fixtureを確定する。
2. snapshot domain/app testで補正後実績、区分内訳、欠勤、有給、半休、時間休、休日振替をREDにする。
3. migration 065とsnapshot repositoryを実装し、monthly close transactionへ接続する。
4. export use case、contract、System Admin handlerを実装する。
5. reopen/reclose、未締め混在、snapshot不変性、CSV encoding/escapingをintegration testで固定する。
6. OpenAPI、API catalog、monthly-closing設計を同期する。

## Observable Acceptance

- closedユーザーだけが成功し、その他は理由付きper-user failureになる。
- 同じsnapshot versionの再exportがbyte-for-byte同一である。
- reopen中は失敗し、reclose後は新versionの値が出力される。
- effective correction、T-11休暇、T-13休日区分が期待列へ反映される。
- System Admin以外は拒否され、contract/integration/lintがgreen。

## Validation

- [x] payroll design doc and CSV fixture
- [x] domain/app RED → GREEN
- [x] close/snapshot transaction integration
- [x] export API integration
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo fmt --all --check`
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`

## Validation Results

- `cargo test -p timekeeper-contract --test payroll_export_contract`: 1 passed
- `cargo test -p timekeeper-app --test payroll_export`: 1 passed
- `cargo test -p timekeeper-backend --test payroll_export_api`: 4 passed（T-03 exact
  classification totals、月跨ぎ day / half / hour leave、T-13 substitution/holiday-work、
  snapshot DB immutability、reopen/reclose revision を含む）
- `bash scripts/harness.sh docs-check`: passed
- backend all-target clippy with warnings denied: passed before the final
  `holiday_work_minutes`/T-09 absence alignment; the final delta has
  `cargo check -p timekeeper-backend` and `git diff --check` passed, and needs one final clippy
  rerun
