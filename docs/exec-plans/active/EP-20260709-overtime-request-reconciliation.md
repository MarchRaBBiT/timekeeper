# EP-20260709-overtime-request-reconciliation

## Goal

- T-07: approved overtime requests and actual overtime are reconciled as work-schedule anomalies.

## Scope

- In: `unapproved_overtime` / `overtime_exceeds_request` anomaly kinds, admin anomaly list and calendar exposure.
- Out: blocking punches or monthly close.

## Done Criteria

- [x] Unapproved overtime is detected.
- [x] Actual overtime above approved request minutes is detected.
- [x] `backend/tests/work_schedule_phase2_api.rs` covers both paths.
- [x] API catalog/OpenAPI updated.

## Validation

- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 18 passed.

## Follow-up fix (2026-07-10)

レビューで検出された HIGH 欠陥2件を `backend/src/repositories/work_schedule/operations.rs` の
`add_overtime_anomalies`（`unapproved_overtime` / `overtime_exceeds_request` の検出ロジック）に修正した。

- 休憩控除漏れ: `raw_work_minutes`（clock_out − clock_in の生スパン）をそのまま
  `expected_work_minutes`（休憩控除後の所定時間）と比較していたため、休憩を正しく取った定時勤務者に
  休憩分の残業が誤検出されていた。既存の `list_break_totals` の結果を用いて実働時間
  （生スパン − 休憩実績）を算出してから比較するよう修正
- flex 誤適用: `add_punctuality_anomalies` にはあった `schedule_type != "fixed"` の early return が
  `add_overtime_anomalies` になく、flexユーザーの`expected_work_minutes`（flex band幅であり契約
  所定時間ではない）を残業判定に使ってしまっていた。同様の early return を追加し、flexの日は
  残業 anomaly 判定の対象外にした（清算期間ベースの正しいflex残業計算は今回のスコープ外）

引数個数が clippy `too_many_arguments` に抵触したため、`break_totals` / `overtime_requests` /
`settings` を `OvertimeAnomalyContext` 構造体にまとめて渡すようにリファクタした（`list_anomalies`
ループの外で1回だけ構築）。

### Validation (実測, 2026-07-10)

- [x] `cargo fmt --all --check` — 差分なし
- [x] `cargo build -p timekeeper-backend --tests` — 成功
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 22 passed; 0 failed
  （新規: `overtime_calculation_deducts_break_minutes`,
  `flex_schedule_is_excluded_from_overtime_anomalies_and_monitor`）
- [x] `cargo test -p timekeeper-backend --lib` — 403 passed; 0 failed
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` — warnings なし
- [x] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings` — warnings なし
- [x] `bash scripts/harness.sh docs-check` — green
- [x] `docs/design-docs/backend-api-catalog.md` の `/api/admin/work-schedule-anomalies` 行を更新
      （休憩控除・fixed限定の意味論を明記。route/method/DTO は不変）

