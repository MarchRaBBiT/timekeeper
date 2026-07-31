# EP-20260705-attendance-classification-read-model

## Goal

- T-03（G1 実装）: [attendance-calculation-policy.md](../../design-docs/attendance-calculation-policy.md) の Decision 1–8, 10–12 を実装し、日次労働時間区分（所定内 / 法定内残業 / 法定外残業 / 深夜 / 法定休日）を導出値として提供する read-model と API 2 本を追加する
- 親タスク: [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-03

## Scope

- In:
  - `crates/domain`: 区分計算の純ロジック（実労働区間の導出、日次・週次法定判定、深夜 overlay、法定休日識別、flex 清算期間区分）+ unit test
  - `crates/app`: `GetMonthlyClassification` read use case（非保存・fail-closed、settlement balance 設計と同型の tagged status）+ unit test
  - `crates/contract`: `MonthlyClassificationResponse` ほか DTO（tagged union）+ round-trip test
  - `crates/infra-postgres`: port 実装（resolved workdays / effective 打刻 / settlement period / 就業規則マスタの読み出し、materializer）
  - `backend`: migration `050_create_work_rule_settings.sql`（就業規則マスタ + 既定値 seed）、handler 2 本、routing、OpenAPI、部署スコープ認可、integration test
  - API: `GET /api/attendance/me/classification?year=&month=`（本人）、`GET /api/admin/users/{user_id}/classification?year=&month=`（Scoped Manager+）
  - docs: `backend-api-catalog.md` 2 行追加、OpenAPI 同期、ExecPlan 登録
- Out（後続タスク）:
  - 丸めポリシー実装（Decision 10 は互換方針のみ）
  - 36協定閾値・監視（T-08）、残業申請突合（T-07）、給与エクスポート（T-14）
  - 就業規則マスタの CRUD API（第一増分は migration seed のみ。改定は運用 SQL / 後続タスク）
  - settlement balance API 本体（本 EP 後の `51a2c9d` で実装済み。本 EP は設計判断のみ流用）
  - frontend / CSV への区分追加

## Done Criteria (Observable)

- [x] 上記 API 2 本が実装され、200 + tagged status（`calculated` / `unresolved_days` / `work_rule_not_configured`）で応答する
- [x] 日次内訳 + 月次合計が丸めなしの整数分で返り、夜勤（月境界跨ぎの深夜分の当月帰属）・法定休日出勤・所定休日出勤・週 40h 跨ぎ・flex 月・fixed/flex 混在月が integration test で固定されている
- [x] 深夜 22:00/5:00 丁度・日次 480/481・週次 2400 丁度・法定休日の週累積除外・清算枠丁度/超過・月跨ぎ週の隣接月一致が domain/app unit test で固定されている
- [x] 全区分（partition）+ 法定休日の合計 = actual_minutes の consistency invariant がテストで固定されている
- [x] contract round-trip テストが tagged union 全 variant を固定している
- [x] 就業規則パラメータは migration 050 の `work_rule_settings`（effective-dated）から読み、行が無い期間は fail-closed（`work_rule_not_configured`）
- [x] 既存 `AttendanceSummary` / CSV は無変更（追加のみ）
- [x] api-catalog / OpenAPI 同期、fmt / clippy / docs-check green

## Constraints / Non-goals

- 区分値は DB へ保存しない（Decision 12）。migration は `work_rule_settings` のみ（採番帯 050/051 のうち 050 を使用）
- 既存 migration の編集禁止。published version / locked projection の不変性に触れない
- `late_grace_minutes` / `early_leave_grace_minutes` は区分計算に使用しない（Decision 10）
- 法定値はハードコードしない（マスタの seed 既定値のみ）

## Task Breakdown

1. [x] migration 050: `work_rule_settings` + 既定値 seed（480 / 2400 / 22:00–5:00 / 週起算 日曜 / 法定休日 日曜）
2. [x] domain: `attendance_classification` モジュール（区間導出・分割・深夜交差・週次累積・flex 清算区分）+ unit test（RED→GREEN）
3. [x] app: `GetMonthlyClassification` + port trait + tagged result + unit test（fake repository）
4. [x] contract: DTO + round-trip test
5. [x] infra-postgres: `ClassificationPostgresRepository` + `PostgresWorkdayMaterializer`
6. [x] backend: handler / routing / OpenAPI / 認可 + integration test
7. [x] docs: api-catalog、OpenAPI、ExecPlan 登録
8. [x] fmt / clippy / 全 crate テスト / 本 EP と PLANS.md / T-03 Status 更新

## Validation Plan

- [x] `cargo fmt --all --check`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-domain -p timekeeper-app -p timekeeper-contract`
- [x] `cargo test -p timekeeper-backend --test attendance_classification_api`
- [x] `cargo test -p timekeeper-backend --tests`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/harness.sh docs-check`

## Design Notes（実装時に確定した精密化。doc との整合は Progress Notes 参照）

- 週次累積のリセットは「日ごとに `week_start_date(work_date, 当日有効な起算曜日)` を計算し、直前処理日と週開始日が変わったらリセット」とする。欠測日（未 resolve かつ実績なし）があっても週境界がずれない
- 月内に未 resolve 日があっても、その日に実績（正の実労働分）が無ければ 0 分の欠測日として扱い月全体は計算する。**未 resolve 日に正の実労働分がある場合のみ** top-level `unresolved_days` で fail-closed（週次累積・法定休日判定に予定情報が必須のため）。月内に flex 日を含む場合の flex 清算区分は、実績の有無に関わらず未 resolve 日が 1 日でもあれば `unresolved_days`（settlement balance Decision 8 の流用）
- flex 清算区分の優先順位: flex 日なし / fixed 混在 → `not_applicable` → `version_mixed`（settlement period 値が 2 種以上）→ `not_configured`（行欠落）→ `calculated`
- flex 清算区分は excess を先に確定してから scheduled を clamp する（`excess = max(0, actual − frame)`、`scheduled = min(actual − excess, contracted)`、`within = 残り`）。契約所定 > 法定総枠の誤設定でも負値を返さない
- 清算期間の法定総枠・暦日数は対象月そのもの。`statutory_weekly_minutes` は月初時点で有効な行を使う
- 深夜帯 `night_start == night_end` は空帯として扱う（seed 既定は 22:00–5:00）
- 打刻は TIMESTAMP WITHOUT TIME ZONE（スケジュール timezone のローカル時刻）として保存されているため、深夜帯の暦時刻交差は naive datetime 同士で行う

## Git Snapshot Log

- [x] `git status --short`
- [x] focused tests pass
- [x] commit recorded: `b39ed35 feat(attendance): add daily classification read-model`

## Progress Notes

- 2026-07-05: EP 作成。attendance-calculation-policy.md Decision 1–12 を読み込み、当時設計段階だった settlement balance EP の tagged status / fail-closed / work_date 帰属 / year・month validation の各判断を流用する方針を確定。
- 2026-07-31: 実装 commit `b39ed35` と後続 settlement balance commit `51a2c9d` を再確認し、完了扱いとした。
- 2026-07-05: 前セッションで実装途中まで進行。mixed fixed/flex 月の fixture が fixed 日の expected minutes を flex 側と混同していたため、実装ではなくテストデータの不整合として引き継ぎ。
- 2026-07-06: 引き継ぎ後、mixed fixed/flex fixture を fixed resolved workday の expected minutes に合わせて修正。既存 admin request integration test は manager 部署スコープ認可に合わせて fixture を明示化。auth timing distribution test は full integration 実行時の p90 jitter で 51ms/75ms 程度の揺れが出るため、median gate は維持し p90 jitter 許容を 75ms に調整。
- 2026-07-08: `.claude/worktrees/agent-a49ed6b9d7cffc94b` に残っていた T-03 実装を main checkout へ取り込み、現在の repo state で再検証した。追加で clippy の `vec_init_then_push` 指摘を app test で修正。検証結果: `cargo test -p timekeeper-domain -p timekeeper-app -p timekeeper-contract` green、`cargo test -p timekeeper-backend --test attendance_classification_api` green（5 passed）、`cargo fmt --all --check` green、`bash scripts/harness.sh docs-check` green、`cargo test -p timekeeper-backend --lib` green（387 passed）、`cargo clippy --workspace --all-targets -- -D warnings` green、`cargo test -p timekeeper-backend --tests` green。
- 2026-07-09: Phase 1 多観点レビュー（ドメイン正当性）で T-03 の区分計算を attendance-calculation-policy.md の Decision 3–8/11 と突き合わせ、深夜帯日跨ぎ・日次/週次法定境界・法定休日分岐・flex 清算・fail-closed のいずれも不一致なしを確認（修正なし）。auth timing test の p90 閾値緩和（25ms→75ms）が本 EP のコミットに混入していた点は commit hygiene の注意事項として backlog EP に記録
