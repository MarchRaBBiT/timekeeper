# EP-20260702-work-schedule-phase2-operational-controls

## Goal

- 勤務体系マスタ Phase 2 として、未来projection生成、anomaly検出、管理者カレンダー、一括割当、月次締めlockを backend/API 境界で提供する

## Scope

- In:
  - 未来 `resolved_workdays` を指定範囲・指定ユーザー集合へ生成する管理API
  - 未設定、予定外勤務、打刻漏れの anomaly 検出・一覧
  - 管理者がユーザー別カレンダーとして resolved workday / attendance / anomaly を範囲取得するAPI
  - 複数targetへ同じ勤務体系割当を作る bulk assignment API
  - 月次締めとして対象月の resolved workday をlockし、後続override変更をDB triggerで拒否するAPI
  - contract DTO、migration、repository、handler、route、OpenAPI/API catalog更新
  - unit / integration tests と coverage計測
- Out:
  - frontend画面
  - cron/常駐worker daemon
  - anomaly解決workflow
  - flex/変形労働/シフト交換

## Done Criteria

- [x] system admin が指定範囲の未来projectionを生成でき、未設定日は件数として返る
- [x] manager+ がカレンダーAPIで配下/任意ユーザーの予定・勤怠・anomalyを範囲取得できる
- [x] anomaly API が未設定、予定外勤務、打刻漏れを決定的に返す
- [x] system admin が複数targetへ一括割当でき、重複/不正参照を個別結果として確認できる
- [x] system admin が月次締めを実行すると対象月のresolved workdayがlockされ、override upsert/deleteは409になる
- [x] API catalogとOpenAPIが実装に同期している
- [x] Phase2 SQL/repository line coverageが80%以上である — `operations.rs` 209/251 = 83.27%（review修正後再計測）

## Constraints

- 既存migrationは変更しない。DB変更は新規migrationで追加する
- lockのsource of truthはDBの `resolved_workdays.locked_at` と既存triggerに置く
- SQL入力はすべてbind parameter
- system admin mutationは既存CSRF/rate limit/auth_system_admin配下に置く
- manager readは既存部署スコープ認可を再利用する

## Validation Plan

- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api` — 13 passed（review修正分の回帰テスト込み）
- [x] `cargo test -p timekeeper-backend --test docs_api` — 3 passed
- [x] `cargo llvm-cov --no-clean -p timekeeper-backend --test work_schedule_phase2_api --summary-only` — `operations.rs` 83.27%
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`

## Progress Notes

- 2026-07-02: Phase1 coverage保留解消後、Phase2 Operational Controls を backend/API のMVPとして開始。
- 2026-07-02: Phase2 backend MVPを実装。`047_create_work_schedule_monthly_closures.sql`、contract DTO、projection生成、anomaly一覧、管理者カレンダー、一括割当、月次締めlock routeを追加。frontend画面と常駐worker daemonはOutのまま。
- 2026-07-02: `work_schedule_phase2_api` で projection生成、calendar表示、monthly close後のoverride 409、schedule_not_configured / missing_clock_in / missing_clock_out / unscheduled_work、bulk assignmentを検証。OpenAPI `docs_api`、docs-check、fmt、workspace clippyがgreen。
- 2026-07-02: coverageは新規Phase2 repository `backend/src/repositories/work_schedule/operations.rs` が 191/226 = 84.51%。`backend/src/handlers/admin/work_schedules.rs` は既存Phase1管理handlerを含む巨大ファイルのため、Phase2 integrationのみではファイル全体31.46%。
- 2026-07-02: rust-reviewer/database-reviewer/security-reviewer並列レビュー（`docs/reviews/2026-07-02-work-schedule-phase2-review.md`）でCritical 1件・High 4件を検出、全件修正。
  - C1: manager が `user_id` 省略時に全社員anomalyを閲覧できる認可バイパスを修正。`list_subordinate_user_ids` で配下ユーザーに限定。
  - H1: `generate_work_schedule_projections` / `bulk_create_work_schedule_assignments` に上限500件のバリデーションを追加（N+1・無制限クエリ発行の緩和）。
  - H2: `close_month` を `pool.begin()` でトランザクション化し、UPDATE成功後のINSERT失敗時に監査ログ欠落しないよう修正。
  - H3: `repository_error_code_message` / projection生成のエラー経路でDB内部エラー文字列を返さないよう汎用メッセージへマップし、詳細は `tracing::error!` に限定。
  - H4: `close_month` が対象0件の場合に監査レコードを作成しないよう変更し、再実行時の重複蓄積を防止（存在しない `user_id` の事前検証も追加）。
  - 修正内容は `work_schedule_phase2_api` に回帰テスト13件（5→13）として反映し、`operations.rs` coverageは209/251 = 83.27%を維持。`cargo fmt --all --check` / `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` はgreen。
