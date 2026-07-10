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

## Follow-up fix (2026-07-10, MEDIUM #2: `list_overtime_monitor` O(U²×M) 再走査 + 日次行の全転送)

レビューで指摘された DB 性能欠陥。`backend/src/repositories/work_schedule/operations.rs` の
`list_overtime_monitor` が抱えていた2つの問題を修正した。

1. **日次行の全転送**: 最大13ヶ月(fiscal/rolling window の広い方)分の attendance を
   `user_id, date` 単位の生 row のまま fetch し、Rust 側 `by_user_month` に月次集計していた。
   全ユーザー×全日分のネットワーク転送・行構築コストが不要にかかっていた。
2. **O(U²×M) 再走査**: `for user_id in user_ids` ループの中で、fiscal 合計・rolling 合計の
   両方について `by_user_month.iter().filter(|((id, ...))| id == user_id ...)` と
   **全ユーザー×全月の HashMap を毎回スキャン**していた。意味論自体は user_id でフィルタして
   いたため結果は正しかったが（欠陥はパフォーマンスのみで正当性のバグではない）、ユーザー数が
   増えるほど二乗で悪化する。

### 修正内容

- SQL を `GROUP BY a.user_id, date_trunc('month', a.date)` の月次集計に変更。ここで
  ef24020 で入った休憩控除（`br` サブクエリの LEFT JOIN + `COALESCE(br.break_minutes, 0)`)と
  `rw.schedule_type = 'fixed'` フィルタは現行のまま維持し、`GREATEST(..., 0)` を `SUM(...)` で
  包む形にした（日次で 0 未満を切り捨ててから月次合計する意味論は変えていない）。
- Rust 側の中間構造を `HashMap<(String, i32, u32), i64>`（全ユーザー×全月フラット）から
  `HashMap<String, HashMap<(i32, u32), i64>>`（ユーザーごとに月マップを分離）に変更。
  fiscal/rolling の合計はそのユーザー自身の月マップ（フェッチ窓に収まる最大 ~13 件）だけを
  走査するため、ループ全体で O(U×M) になる。
- レスポンス契約（フィールド・ソート順）は不変。DTO/OpenAPI/catalog の変更なし。
- 呼び出し元 `handlers/admin/work_schedules.rs::list_overtime_monitor` の
  `SELECT id FROM users ORDER BY id`（system admin 時の全ユーザー取得、LIMIT なし）は
  **今回はそのまま**とした。ページネーション導入はレスポンス契約（`items` の形・ページング
  パラメータ）変更になり本タスクのスコープ外（呼び出し元の契約は不変という指示）。ユーザー数が
  数千規模に達した場合はこの一覧取得がボトルネックになり得るため、別チケットでの対応候補として
  記録しておく。

### Tests added (`backend/tests/work_schedule_phase2_api.rs`)

- `overtime_monitor_keeps_per_user_month_totals_independent_across_users`: 2ユーザー
  (employee_a, employee_b) × 2ヶ月(2026-04, 2026-08) で異なる残業分数を作り、SQL 側の
  `GROUP BY a.user_id, date_trunc('month', a.date)` と Rust 側のユーザー単位マップ構築が
  ユーザー間で値を混同しないことを固定する回帰テスト。fiscal_year_start_month=4 で
  year=2026,month=8 を問い合わせ、employee_a: month=90分/fiscal=210分、employee_b:
  month=0分/fiscal=120分（互いの分数が一切混入していない）ことを検証。

### Validation (実測, 2026-07-10)

- [x] `cargo fmt --all --check`（backend/ 配下） — 差分なし
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` —
      27 passed; 0 failed（新規: `overtime_monitor_keeps_per_user_month_totals_independent_across_users`。
      `sqlx::migrate!` により新規 migration `059_add_phase2_indexes.sql` もこの実行で自動適用・検証済み）
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` — warnings なし
- API 契約不変のため `docs/design-docs/backend-api-catalog.md` の更新は不要（変更なし）

