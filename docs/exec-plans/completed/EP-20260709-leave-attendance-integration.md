# EP-20260709-leave-attendance-integration

**親タスク:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-06（G3 前半）
**前提:** [EP-20260704-attendance-calculation-policy-design](./EP-20260704-attendance-calculation-policy-design.md) / [docs/design-docs/attendance-calculation-policy.md](../../design-docs/attendance-calculation-policy.md)

## Goal

- 承認済み休暇を勤怠の read path に導出値として反映し、打刻欠測と休暇を区別できるようにする
- 休暇日に打刻がある場合は実態を保存したまま `leave_conflict` anomaly として検出する

## Scope

- In:
  - `GET /api/attendance/me` に休暇日を `status = "on_leave"` と `leave { leave_request_id, leave_type }` 付きで返す
  - 月次サマリに `leave_days` を追加し、承認済み休暇日を集計する
  - 本人/管理者 CSV export に `Leave Type` 列を追加し、休暇のみの日も導出行として出力する
  - 管理者 work schedule calendar に `days[].leave` を追加する
  - `GET /api/admin/work-schedule-anomalies` に `leave_conflict` を追加する
  - 既存レスポンスの後方互換を保つ contract test と integration test
- Out:
  - resolved workday への休暇状態の書き込み
  - 打刻の reject
  - 半休・時間単位休暇の同日分割表示（T-11）
  - 欠勤判定・遅刻早退判定（T-07 / T-09）

## Done Criteria

- [x] 承認済み休暇が本人勤怠一覧・月次サマリ・本人CSV・管理者CSV・work schedule calendar に表示される
- [x] 承認済み休暇の取消後、各 read path から休暇表示が消える
- [x] 休暇日の打刻は拒否せず、管理者 anomaly に `leave_conflict` として表示される
- [x] 休暇のみの日は打刻漏れ anomaly として扱われない
- [x] `cargo fmt --all --check`、focused integration、backend tests、clippy、docs-check が green
- [x] conventional commit 作成済み、本 EP に実測値を記録済み

## Task Breakdown

1. [x] T-01 の休暇日ポリシーと既存 attendance/work schedule/export 経路を確認する
2. [x] app/contract の failing tests を追加する
3. [x] app use case に承認済み休暇の read-time merge を追加する
4. [x] infra-postgres に承認済み休暇日 range query を追加する
5. [x] backend handler / CSV / work schedule repository を接続する
6. [x] backend integration test を追加する
7. [x] docs/catalog/OpenAPI/タスク表を同期する
8. [x] validation ladder を実行する

## Validation Plan

- [x] `cargo test -p timekeeper-app --test list_user_attendance`
- [x] `cargo test -p timekeeper-app --test export_admin_attendance`
- [x] `cargo test -p timekeeper-contract --test attendance_contract`
- [x] `cargo test -p timekeeper-contract --test work_schedule_contract`
- [x] `cargo test -p timekeeper-backend --test attendance_api approved_leave_surfaces_in_attendance_reads_and_cancel_reverts -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test admin_export_api admin_export_includes_approved_leave_rows_and_leave_type_column -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api approved_leave_surfaces_in_calendar_and_clock_in_conflict_anomaly -- --nocapture`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --tests`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/harness.sh fmt-check`
- [x] `bash scripts/harness.sh docs-check`

## Progress Notes

- 2026-07-09: T-06 着手。T-01 Decision 9 に従い、休暇は resolved workday へ書き込まず read-time join で扱う方針を確認。
- 2026-07-09: app/contract/backend の実装と focused tests を追加。休暇のみの日は `on_leave` 導出行、打刻済み休暇日は実打刻を維持したまま `leave` を付与し、anomaly は `leave_conflict` とする。
- 2026-07-09: 検証実測: `cargo test -p timekeeper-app --test list_user_attendance`、`cargo test -p timekeeper-app --test export_admin_attendance`、`cargo test -p timekeeper-contract --test attendance_contract`、`cargo test -p timekeeper-contract --test work_schedule_contract`、T-06 focused integration 3 本、`cargo test -p timekeeper-backend --lib`、`cargo test -p timekeeper-backend --tests`、`cargo clippy --workspace --all-targets -- -D warnings`、`bash scripts/harness.sh fmt-check`、`bash scripts/harness.sh docs-check` は green。
- 2026-07-09: Phase 1 多観点レビューを反映。read 表示（leave surfacing）は暦日ベースを維持し、消化計算のみ稼働日ベースとする非対称を leave-entitlement.md に決定として明文化（144ec32）。admin export は from/to 省略時に当月（片方指定時はその月）へフォールバックし 366 日超を 400 で拒否、leave マージキーを username から (user_id, date) へ変更（c7116c5）。`/api/admin/attendance` の `leave` が常に null である旨を api-catalog に追記。検証: admin_export_api 7 件 green
