# EP-20260730-work-interval-anomaly

**Status:** 実装中  
**Parent task:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-18

## Goal

補正後の直前退勤から次の出勤までの実時間差が設定閾値を下回る場合に`insufficient_rest` anomalyを表示する。打刻はブロックしない。

## Scope And Decisions

- migration `064` で既存のeffective-dated `overtime_monitor_settings` に勤務間インターバル設定を追加し、既定660分とする。
- 判定は暦日やschedule時刻ではなくeffective clock-out/in timestampの差で行う。
- `elapsed < threshold`だけをanomalyとし、660分ちょうどは正常とする。
- 検索範囲先頭の判定に必要な直前勤務を範囲外から1件取得する。欠落したclock-out/inからは推測しない。
- 夜勤、`end_day_offset = 1`、月境界、補正後打刻を同じtimestamp差ロジックで扱う。
- anomaly contract、sort rank、calendar/list、OpenAPI、API catalogへ同じkindを追加する。

## TDD And Implementation

1. 660/659分、通常、夜勤、連続勤務、月境界、補正後値、直前勤務欠落をREDにする。
2. 既存のSystem Admin向けovertime-monitor settings read/update APIへ設定を追加する。
3. anomaly queryへ範囲外直前勤務を含む時系列材料化を追加する。
4. `insufficient_rest`生成を既存anomaly list/calendarへ接続する。
5. 打刻処理が影響を受けないことをintegration testで固定する。
6. OpenAPI、API catalog、contract testを同期する。

## Observable Acceptance

- 659分は検出、660分は非検出になる。
- 夜勤・月境界・補正済み時刻で誤検知しない。
- range先頭日も直前勤務があれば判定され、情報不足時は生成しない。
- anomalyがlist/calendarへ現れる一方、clock-inは拒否されない。
- contract、integration、lintがgreen。

## Validation

- [x] anomaly RED → GREEN
- [x] setting API integrationを追加
- [x] normal/night/correction/range-boundary integrationを追加
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`

## Progress

- 2026-07-30: `insufficient_rest` contract、migration 064、effective打刻の専用read、検索範囲直前の完了勤務取得、既存settings APIへの閾値配線を実装した。659分、660分ちょうど、補正後退勤、月境界、夜勤のintegration testを追加。focused 2件と`work_schedule_phase2_api`全29件、`cargo check -p timekeeper-backend --lib`、backend all-target clippy、`docs-check`はgreen。
- 2026-07-30: adversarial reviewのBlock指摘を反映。effective row存在時はcorrected NULLをraw値へ戻さないCASE式へ修正し、前勤務clock-out NULL補正と次勤務clock-in NULL補正を回帰test化した。range末尾の設定を全期間へ遡及適用せず、各next work_date時点のeffective-dated閾値を選択するよう変更し、range途中の660→720分改定をtest化した。直前勤務検索用に`attendance (user_id, date DESC)` indexをmigration 064へ追加。新規3testはgreenを確認後、fixture間の設定履歴干渉を隔離した。隔離後の最終再実行はTestcontainers起動失敗、all-target clippy再実行は並行T-11のcompile errorで停止したため、統合時に再実測する。
