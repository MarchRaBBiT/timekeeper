# EP-20260730-admin-attendance-report

**Status:** Review Approve（実DB testcontainer再実行のみ未確認）  
**Parent task:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-15

## Goal

Scoped Manager以上が認可範囲内の月次労働時間、区分内訳、遅刻・早退・欠勤、anomaly、36協定状態をページングされた一覧とadmin画面で確認できるようにする。

## Scope And Decisions

- migrationは追加しない。T-03、T-08、T-09のread-modelを合成し、新しい時間計算を作らない。
- report readはprojectionを生成しない。scope全員の既存projection/実績/補正/休憩/規則を
  set-based preloadし、月内projection不足は`unresolved_days`にする。
- APIは`GET /api/admin/attendance-report?year=&month=&department_id=&page=&per_page=`。
- department未指定時はactorの認可範囲へ自動限定する。明示されたscope外departmentは403。
- sortは`exceeded > warning > ok`、次に安定したユーザーキーとする。全対象をsortしてからpaginationし、ページ内だけを並べ替えない。
- T-03が計算不能の場合は0へ潰さずtagged statusを返す。
- N+1を避け、月・scope単位のbatch readでclassification、overtime status、anomaly countを合成する。
- frontendは`frontend/src/pages/admin/`配下のfeature module、repository、view modelへ分け、global clientを肥大化させない。

## TDD And Implementation

1. contract/app testでpagination、scope、sort、計算不能statusをREDにする。
2. batch repositoryとreport use caseを実装する。
3. 薄いhandler、route、OpenAPI、API catalogを追加する。
4. admin feature UIへ年月・部署filter、status badge、loading/error/empty、paginationを追加する。
5. API integrationとfrontend host testで認可・表示・操作を固定する。

## Observable Acceptance

- managerは配下のみ、System Adminは全社を取得でき、明示scope外departmentは403。
- severity順がページを跨いでも保たれ、pagination metadataが正しい。
- 区分内訳、遅刻・早退・欠勤、anomaly件数、36協定状態が既存read-modelと一致する。
- loading/error/empty/filter/paginationをfrontend host testで確認できる。
- API integration、frontend host、lintがgreen。

## Validation

- [x] contract/app RED → GREEN
- [x] batch repository integration（scope user/anomaly/overtimeを月単位で取得）
- [x] API scope/pagination integration（4 passed）
- [x] frontend host tests（2 passed）
- [x] `bash scripts/harness.sh docs-check`
- [x] `bash scripts/harness.sh lint`

## Review 2026-07-30

- **Approve:** HighだったT-03 per-user SQL round tripは、`BatchClassificationSnapshot`で
  resolved/actual/effective correction/break/work-rule/settlementをscope単位にpreloadし、
  in-memory repositoryからpure T-03分類する形へ修正した。100 userでもapp repository callが
  1回であることをmock testで固定した。
- **検証済み:** `backend/tests/admin_attendance_report_api.rs`は4 passed。System Admin、
  manager配下scope、scope外403、pagination、severity順、invalid queryを固定した。
- reportはread-onlyでprojectionを生成しない契約へ変更し、projection不足を
  `unresolved_days`としてfail-closedにした。
- 全scopeを分類してからglobal severity順を確定するため、単純な先行paginationは行わない。
  大規模組織で実測上の問題が出た場合はmaterialized read-modelを別タスクで導入する。
