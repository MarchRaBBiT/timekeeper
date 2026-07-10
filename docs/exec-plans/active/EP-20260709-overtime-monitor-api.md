# EP-20260709-overtime-monitor-api

## Goal

- T-08: expose a 36-agreement overtime monitoring read API.

## Scope

- In: effective-dated overtime monitor settings, admin read API, threshold status calculation.
- Out: frontend dashboard integration.

## Done Criteria

- [x] `overtime_monitor_settings` migration exists with default thresholds.
- [x] `GET /api/admin/overtime-monitor` returns monthly/yearly/rolling-average statuses.
- [x] System admin can get/upsert monitor settings.
- [x] API catalog/OpenAPI updated.

## Validation

- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api overtime_monitor_reports_threshold_statuses -- --nocapture` — passed.
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 18 passed.

## Follow-up fix (2026-07-10)

レビューで検出された HIGH 欠陥2件を `backend/src/repositories/work_schedule/operations.rs` の
`list_overtime_monitor` に修正した。

- 休憩控除漏れ: 36協定監視の時間外分数が `clock_out - clock_in`（休憩込みの生スパン）から
  `expected_work_minutes`（休憩控除後の所定時間）を引いていたため、休憩を正しく取った定時勤務者に
  休憩分の残業が誤検出されていた。`break_records`（`break_end_time IS NOT NULL`）を
  `attendance_id` で LEFT JOIN 集計し、実働時間から控除するよう修正
- 年度累計の欠落: attendance の取得範囲が `rolling_from`（直近6ヶ月窓の開始月）〜月末に限定されており、
  年度開始月が6ヶ月以上前になる照会（例: fiscal_year_start_month=4、month=12）で年度累計から
  4〜6月分が欠落していた。取得開始日を `min(fiscal_start, rolling_from)` に変更
- flex 誤適用: `schedule_type != 'fixed'` の日は `expected_work_minutes` がflex band幅であり契約
  所定時間ではないため、監視対象から除外するフィルタ (`rw.schedule_type = 'fixed'`) を追加

### Validation (実測, 2026-07-10)

- [x] `cargo fmt --all --check` — 差分なし
- [x] `cargo build -p timekeeper-backend --tests` — 成功
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 22 passed; 0 failed
  （新規: `overtime_calculation_deducts_break_minutes`,
  `flex_schedule_is_excluded_from_overtime_anomalies_and_monitor`,
  `overtime_monitor_fiscal_year_total_includes_months_before_rolling_window`）
- [x] `cargo test -p timekeeper-backend --lib` — 403 passed; 0 failed
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` — warnings なし
- [x] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings` — warnings なし
- [x] `bash scripts/harness.sh docs-check` — green
- [x] `docs/design-docs/backend-api-catalog.md` の `/api/admin/overtime-monitor` 行を更新
      （休憩控除・fixed限定・年度累計の意味論を明記。route/method/DTO は不変）

## Follow-up fix (2026-07-10, MEDIUM #1: settings i64→i32 overflow → 500)

`validate_overtime_monitor_settings`（`backend/src/handlers/admin/work_schedules.rs`）が
「正の値」チェックのみで上限を持たなかった欠陥を修正した。contract 側 DTO
（`OvertimeMonitorSettingsRequest`）の分単位フィールドは `i64` だが DB カラムは
`INTEGER`（`i32`）のため、`i32::MAX` を超える値を送ると repository の
`i32::try_from` が `WorkScheduleRepositoryError::CorruptData` を返し、
`AppError::InternalServerError`（500）としてクライアントに漏れていた。単なる入力ミスが
500 になり監視を誤らせる。

修正: `monthly_limit_minutes` / `yearly_limit_minutes` /
`rolling_average_limit_minutes` / `single_month_absolute_limit_minutes` /
`overtime_request_tolerance_minutes` に上限チェックを追加し、`400 INVALID_WORK_SCHEDULE`
で弾くようにした。

### 上限値の選定根拠

`i32::MAX`（2,147,483,647 分 ≈ 4085 年）そのものを上限にすることもできたが、それは
「オーバーフローしない」ことしか保証しない技術的な値であり、業務的に意味のある値ではない。
そこで以下を採用した。

- `monthly_limit_minutes` / `yearly_limit_minutes` / `rolling_average_limit_minutes` /
  `single_month_absolute_limit_minutes`: `MAX_OVERTIME_LIMIT_MINUTES = 527,040 * 10 =
  5,270,400`分（1年 = 527,040分の10倍、約10年分）。36協定の監視閾値としてこれを超える設定は
  現実的にあり得ないが、多年度の運用や将来のテストデータ生成にも十分な余裕を残した
- `overtime_request_tolerance_minutes`: `MAX_OVERTIME_TOLERANCE_MINUTES = 44,640`分
  （31日分）。この値は残業申請と実績の突合の「許容差」であり、他の上限フィールドと違って
  性質上小さいはずなので、より狭い上限とした
- どちらも `i32::MAX` から大きく距離があるため、`i32::try_from` が対象フィールドで
  オーバーフローすることは構造的に起こらない

### Validation (実測, 2026-07-10)

- [x] `cargo check -p timekeeper-backend` / `--tests` — 成功
- [x] `cargo fmt --all --check` — 差分なし
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 26 passed; 0 failed
      （新規: `upsert_overtime_monitor_settings_rejects_minutes_beyond_i32_range`）
- [x] `cargo test -p timekeeper-backend --lib` — 403 passed; 0 failed
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` — warnings なし
- [x] `bash scripts/harness.sh docs-check` — green
- [x] `docs/design-docs/backend-api-catalog.md` の `/api/admin/overtime-monitor/settings` 行を更新
      （上限超過で `400 INVALID_WORK_SCHEDULE` を返す旨と具体的な上限値を明記。route/method/DTO は不変）

