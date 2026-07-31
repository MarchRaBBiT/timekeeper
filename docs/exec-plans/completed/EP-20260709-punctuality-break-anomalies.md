# EP-20260709-punctuality-break-anomalies

## Goal

- T-09/T-10: detect late, early leave, absent, and insufficient break anomalies.

## Scope

- In: fixed-schedule punctuality anomalies, past-day absence, configured break minimum checks.
- Out: hard rejection of punches or closes.

## Done Criteria

- [x] `late`, `early_leave`, `absent`, and `insufficient_break` anomaly kinds are in the contract.
- [x] Approved leave days are excluded from absence detection.
- [x] Boundary behavior is covered by focused integration tests.
- [x] API catalog/OpenAPI updated.

## Validation

- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api anomaly_list_detects_overtime_punctuality_absence_and_break_warnings -- --nocapture` — passed.
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 18 passed.

## Follow-up fix (2026-07-10, MEDIUM #2: absent 判定が UTC 日付基準)

`list_anomalies`（`backend/src/repositories/work_schedule/operations.rs`）内の
`None if work_date < Utc::now().date_naive()` が UTC 日付を「今日」の基準にしていた欠陥を
修正した。業務タイムゾーン（既定 Asia/Tokyo, UTC+9）では、UTC 日付が変わる 09:00 (JST) まで
前日が「過去日」と判定されず、`absent`（打刻なし欠勤）の検出が最大9時間遅延していた
（例: JST 7/10 00:00〜08:59 の間、UTC は依然 7/9 のため、7/9 が過去日と判定されず
`missing_clock_in` のまま欠勤として検出されない）。

修正方針: repository 層が `Utc::now()` を直接参照する設計をやめ、`list_anomalies` の
シグネチャに `today: NaiveDate` を追加した。呼び出し元（`backend/src/handlers/admin/
work_schedules.rs` の `list_work_schedule_anomalies` と `get_work_schedule_calendar`
の両方。grep で全呼び出し元を確認し、2箇所とも修正）で
`crate::utils::time::today_local(&state.config.time_zone)` を計算して渡すようにした。
これにより repository はテストから `today` を固定して境界を検証できるようになり、
判定基準も業務タイムゾーンで決定的になった。

### Validation (実測, 2026-07-10)

- [x] `cargo check -p timekeeper-backend` / `--tests` — 成功（`list_anomalies` の呼び出し元
      2箇所ともコンパイルエラーで検出・修正済み）
- [x] `cargo fmt --all --check` — 差分なし
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` — 26 passed; 0 failed
      （新規: `anomaly_list_uses_caller_supplied_today_for_absent_boundary` —
      `today` の前日で打刻なし → `absent`、`today` 当日で打刻なし → `missing_clock_in`
      の境界を、repository 関数を直接呼び出して固定 `today` で検証）
- [x] `cargo test -p timekeeper-backend --lib` — 403 passed; 0 failed
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` — warnings なし
- [x] `bash scripts/harness.sh docs-check` — green
- [x] `docs/design-docs/backend-api-catalog.md` の `/api/admin/work-schedule-anomalies` 行を更新
      （`absent`/`missing_clock_in` の境界が業務タイムゾーンの「今日」基準である旨を明記。
      route/method/DTO は不変）

