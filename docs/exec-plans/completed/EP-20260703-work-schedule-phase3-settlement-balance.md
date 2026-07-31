# EP-20260703-work-schedule-phase3-settlement-balance

## Goal

- Phase3 の最終増分として、flexスケジュールの清算期間残高（契約所定時間 vs 補正後実績）を計算・参照できるようにする
- [`EP-20260702-work-schedule-phase3-api-wiring.md`](./EP-20260702-work-schedule-phase3-api-wiring.md) / [`EP-20260703-work-schedule-phase3-resolve-workday-flex.md`](./EP-20260703-work-schedule-phase3-resolve-workday-flex.md) の Out 節で申し送られた「清算期間残高の実績突合」を解消する
- あわせて、残高計算の前提となる勤怠計算ポリシーのうち本増分に必要な最小限（実績時間の定義・休憩の扱い・丸めの扱い）を確定し、`docs/design-docs/work-schedule-master.md` へ節として追記する

## 前提（開始条件）

- Phase3 の先行2EP（domain/contract、API配線、`ResolveWorkday` flex対応）は完了済み（commit `8305f16` まで）
- `work_schedule_settlement_periods`（version単位、`unit='monthly'`、`contracted_minutes_per_period`）は migration 048 で永続化済み・published version は DB トリガーで不変
- `ResolvedWorkday` は `schedule_type` とコアタイム snapshot を持つが、**清算期間は projection に snapshot されず `work_schedule_version_id` 経由の version 参照**（EP-20260703 の設計判断）

## Design Decisions（本EPで確定する設計判断）

以下は実装前に確定した判断であり、実装タスクでこのまま `work-schedule-master.md` へ「清算期間残高（第一増分）」節として転記する。

1. **実績時間の定義 — 補正後 effective values の timestamp 差分から整数分で直接計算する**
   残高の実績分は、既存の勤怠一覧・月次サマリ・CSVと同じく `attendance_correction_effective_values` 適用後の clock/break を入力とする。ただし**保存済み `attendance.total_work_hours`（f64 時間値）は残高計算に使用しない**。f64 の時間値から分へ逆変換すると丸め誤差が入るため、effective な timestamp 差分から整数分（`actual_minutes`）を直接計算する。既存サマリとの一貫性は「同一の effective 入力から導出する」というレベルで保証し、テストは残高側の分値をテスト内で独立計算して assert する（f64 サマリとの数値突合はしない）。raw attendance を使うと既存 UI/CSV の実績と残高がズレるため不採用。
2. **休憩 — 実休憩打刻ベース、予定休憩の自動控除はしない**
   design doc の Time Semantics（実績労働時間は実際の休憩打刻から計算。予定休憩自動控除は別ポリシー）に従う。休憩打刻の欠落はそのまま実績に反映され、既存サマリと同じ値になる。予定休憩控除の導入は勤怠計算ポリシー本体（Follow-up Designs 1）の後続とする。
3. **丸め — 第一増分では丸めなし（分単位の生値）**
   残高 API は丸め前の分値を返す。所定内外・深夜・遅刻早退・丸めの各ポリシーは Follow-up Designs 1 の本体であり本EPのスコープ外。丸めが導入されても残高は派生値のため互換を壊さず再定義できる。
4. **契約所定分 — settlement period の月次契約値をそのまま使う**
   `contracted_minutes_per_period`（Monthly）を対象月の契約所定分とする。月の営業日数差による按分・日割りはしない（契約値は「期間あたり」として定義済みのため）。
5. **期間内の version 混在 — fail-closed（計算不可を明示）**
   version 混在判定の母集団は**対象月の全 resolved workdays（非勤務日・祝日を含む）**とする。`ResolvedWorkday` は非勤務日でも `work_schedule_version_id` と `schedule_type` を保持するため、勤務日だけに限定すると非勤務日でのみ version が切り替わるケースを見逃す。母集団の参照する published version が複数あり settlement period 値が一致しない場合、按分ルールは労務判断を要するため第一増分では計算せず、計算不可応答（`version_mixed`）を返す。version が単一、または複数でも settlement period 値が同一なら計算する。按分が必要になった時点で別EPとして設計する。
6. **月次締め・再締めとの関係 — 残高は保存しない導出値とし、常に最新で計算する**
   残高は DB へ保存せず read-model として都度計算する。締め済み月は locked projection が日次データを固定するが、締め後に attendance correction が承認されれば残高は変わり得ることを許容する（残高は導出値であり、締め時点の固定が必要になるのは給与エクスポート契約の設計時。その固定は Follow-up Designs 2「月次締め・承認・再締め」/ 4「給与エクスポート契約」で扱う）。
7. **expected_work_minutes は使わない**
   projection の `expected_work_minutes` は flex band（最大稼働可能幅）であり契約所定時間ではない（API catalog 明記済み）。残高計算の契約側は settlement period のみを参照する。
8. **対象は flex のみ・月の完全性を前提とする**
   計算前に、既存の読み取り経路（`list_resolved_workdays` の `materialize_resolved_workdays`）と同じ方式で**対象月の全日を先に resolve（materialize）する**。そのうえで:
   - materialize 後も resolved workday が存在しない日（スケジュール未設定日）が1日でもある月 → `unresolved_days` で計算不可（欠損日を無視して部分月を計算しない fail-closed）
   - 全日 resolved かつ `schedule_type = fixed` の日を含む月（fixed のみ・fixed/flex 混在の両方）→ `not_applicable`。混在月の部分計算は労務判断を要するため第一増分では行わない
   - 全日 flex だが settlement period 行が取得できない（データ不整合）→ `not_configured`
   - 計算不可ステータスの優先順位は `unresolved_days` → `not_applicable` → `version_mixed` → `not_configured` の順で判定し、テストで固定する
9. **出力の形 — read-only API 2本のみ（DB保存・CSVなし）、計算不可は 200 + tagged union で返す**
   - `GET /api/work-schedules/me/settlement-balance?year=&month=`（従業員本人）
   - `GET /api/admin/users/{user_id}/settlement-balance?year=&month=`（Scoped Manager+。既存 resolved-workdays と同じ部署スコープ認可）
   - **HTTP 契約**: 計算不可（`unresolved_days` / `not_applicable` / `version_mixed` / `not_configured`）は**リクエスト不正ではなくリソースの正当な状態**であるため、`200 OK` + レスポンス DTO の tagged union（serde tag 付き `status` フィールド: `calculated` | `unresolved_days` | `not_applicable` | `version_mixed` | `not_configured`）で返す。4xx + 既存 `ErrorResponse` envelope（`{"error","code"}`）はリクエスト検証エラー（下記12）と認可（403）にのみ使う。tagged union は contract round-trip テストと OpenAPI schema に固定する
   - `calculated` 時のフィールド: 対象期間、`contracted_minutes`、`actual_minutes`（整数分）、`balance_minutes`（actual − contracted。負値=不足）、日次内訳（`work_date`、`actual_minutes`、`locked` 有無、`in_progress` 有無）
10. **実績の月帰属 — `work_date` 基準（暦時刻では分割しない）**
    実績分の集計キーは attendance が紐づく `work_date`（= resolved workday の日付）とする。`workday_boundary` 前の打刻が前日 `work_date` に帰属する既存挙動、および夜勤（`end_day_offset = 1`）が暦日を跨ぐケースはそのまま従い、**夜勤が月境界を跨いでも実績は `work_date` の属する月へ全量帰属させる**（Time Semantics「終了日だけで別勤務日に分割しない」と整合）。月末夜勤・月初 boundary 前打刻をテストで固定する。
11. **進行中 attendance と当月・未来月**
    `clock_out` 欠落の進行中 attendance は `actual_minutes = 0` として日次内訳に `in_progress: true` で含める（既存サマリが `total_work_hours = None` を集計外とするのと整合する保守的な扱い）。当月途中の要求は暫定残高として正当なので許可する。未来月も既存読み取り経路と同様に materialize のうえ実績 0 で計算する。
12. **リクエスト検証 — 既存月次締めと同一範囲**
    `year` は 1900..=9999、`month` は 1..=12 を範囲外 `400 INVALID_WORK_SCHEDULE` で拒否する（`close_work_schedule_month` の既存 validation と同一）。不正 `user_id` は既存 admin API と同じ扱い。
13. **アーキテクチャ配置 — 既存パターンに従う**
    use case は `crates/app`（`ListUserWorkdays` と同型の read use case。計算純ロジックは app unit テストで固定）、SQL は `crates/infra-postgres`、handler は `backend/src/handlers/`。DTO は `crates/contract`。

## Scope

- In:
  - `crates/app`: `CalculateSettlementBalance` read use case（port: 対象月の flex resolved workdays + settlement period + 補正後実績の読み出し）
  - `crates/infra-postgres`: port 実装（resolved workdays・`work_schedule_settlement_periods`・attendance effective values の読み出し。新規テーブルなし・migrationなし）
  - `crates/contract`: `SettlementBalanceResponse` / 日次内訳 DTO / エラー code（round-trip テスト付き）
  - `backend`: 上記 API 2本の handler・routing・OpenAPI 登録・部署スコープ認可
  - `docs/design-docs/work-schedule-master.md`: Design Decisions の転記（「清算期間残高（第一増分）」節）、Status/Implementation Status 更新、Follow-up Designs の該当項目更新
  - `docs/design-docs/backend-api-catalog.md`: API 2本の行を追加
  - テスト: app unit（RED→GREEN）、backend integration（flex月の残高計算・correction 反映・version混在・not-applicable・認可）、contract round-trip
- Out（後続EPまたは対象外）:
  - 残高の DB 保存・締め時点の残高固定・給与エクスポート（Follow-up Designs 2/4）
  - 丸め・所定内外・深夜・遅刻早退の勤怠計算ポリシー本体（Follow-up Designs 1）
  - version 混在月の按分計算
  - fixed/flex 混在月の部分計算
  - 予定休憩の自動控除
  - frontend、CSV 出力

## Done Criteria (Observable)

- [x] 全日 flex・単一 version の月について、`GET /api/work-schedules/me/settlement-balance` が `status = "calculated"` と `contracted_minutes`（settlement period 由来）・`actual_minutes`（整数分）・`balance_minutes` を返す（backend統合テスト: flex version作成→publish→assign→resolve→打刻→残高取得のエンドツーエンド）
- [x] レスポンスは tagged union（`status` フィールド）であり、計算不可4種を含む全 variant が contract round-trip テストと OpenAPI schema に固定されている。計算不可は 200 で返り、4xx はリクエスト検証・認可のみ
- [x] `actual_minutes` は effective values の timestamp 差分から整数分で直接計算され、**保存済み `attendance.total_work_hours`（f64）を計算入力に使用していない**ことをテストで固定する
- [x] attendance correction の**申請→承認の API フロー経由**（直接 DB insert ではない）で承認された補正が残高へ反映される
- [x] 休憩は実打刻ベースで控除され、予定休憩は控除されない。`clock_out` 欠落の進行中 attendance は `actual_minutes = 0`・`in_progress: true` で日次内訳に含まれる
- [x] 実績は `work_date` 基準で月へ帰属する: 月末の夜勤（`end_day_offset = 1` で翌暦月に跨る）の実績が当月へ全量帰属し、月初の `workday_boundary` 前打刻の実績が前月へ帰属することをテストで固定する
- [x] materialize 後も resolved workday が無い日を含む月は `unresolved_days` で計算不可となり、欠損日を無視した部分計算が行われない（resolved 0件の月・1日だけ欠損の月の両方をテスト）
- [x] 対象月の**全 resolved workdays（非勤務日・祝日含む）**が参照する version の settlement period 値が一致しない場合、`version_mixed` で計算不可が返る（値の按分をしない）。非勤務日でのみ version が切り替わるケースも検出されることをテストで固定する
- [x] fixed のみ・fixed/flex 混在の月は `not_applicable`、全日 flex だが settlement period 行が無い月は `not_configured` が返り、計算不可ステータスの優先順位（`unresolved_days` → `not_applicable` → `version_mixed` → `not_configured`）がテストで固定されている
- [x] 祝日・非勤務日は `actual_minutes = 0` の日次内訳として計算に含まれ（計算不可にならない）、当月途中・未来月の要求は暫定残高（実績 0 を含む）として成功する
- [x] `year`（1900..=9999）・`month`（1..=12）の範囲外は `400 INVALID_WORK_SCHEDULE`（`month=0/13`、`year=10000`、不正 `user_id` をテスト）
- [x] 月次締め済み（locked projection）の月でも残高が計算でき、締め後の correction 承認が残高へ反映される（残高が保存値でなく導出値であることの固定）
- [x] admin API は既存 resolved-workdays と同じ部署スコープ認可に従う（System Admin=全員、Manager=配下のみ、スコープ外は403）
- [x] 残高は丸めなしの分値であり、`expected_work_minutes` を計算に使用していない
- [x] `work-schedule-master.md` に Design Decisions が転記され、`backend-api-catalog.md` に API 2本が追記される（**混在月・未設定・計算不可の扱いと `status` tagged union を利用者向けに明記**）
- [x] 既存テスト（workday_resolver / work_schedule_read_api / admin_work_schedules_api / work_schedule_phase2_api / attendance_work_schedule_integration / backend --lib）に回帰がない

## Constraints / Non-goals

- migration は追加しない（既存テーブルのみで計算可能）。やむを得ず必要になった場合は新規ファイルとして追加し、既存 migration は変更しない
- published version・locked projection の不変性には一切触れない
- `ResolveWorkday` / `ResolvedWorkday` の構造・挙動を変更しない（読み出し専用の追加）
- domain の `SettlementPeriod` / `FlexPolicy` / `validate_flex_policy` を変更しない
- エラー応答は内部情報（SQL・スタックトレース・他ユーザーのデータ有無）を漏らさない
- 認可は既存の Scoped Manager+ パターンを再利用し、新しい認可モデルを導入しない

## Task Breakdown

1. [x] app unit test（RED）: `crates/app/tests/settlement_balance.rs` を新規作成し、**計算純ロジック**（単一version計算・整数分計算・version混在fail-closed（非勤務日のみの切替含む）・unresolved_days・not_applicable（fixed混在含む）・not_configured・ステータス優先順位・進行中attendanceの0分扱い・丸めなし）を失敗テストで固定する
2. [x] app 実装（GREEN）: `CalculateSettlementBalance` use case・port trait・結果型（`status` tagged union に対応する enum）
3. [x] infra-postgres: port 実装（月次 resolved workdays 読み出し、version→settlement period 解決、effective values 適用済み実績の timestamp 読み出し）と integration test
4. [x] contract: tagged union DTO（`status` 全 variant）+ round-trip テスト
5. [x] backend: handler 2本・routing・year/month validation・OpenAPI 登録・部署スコープ認可 + integration test（**materialize を含むエンドツーエンド・correction 申請→承認 API フロー経由の反映・月境界の夜勤/boundary前打刻・resolved 0件月/1日欠損月・当月途中/未来月・認可403・validation 400**。HTTP 契約と境界日付は統合テスト側で固定する）
6. [x] docs: `work-schedule-master.md` へ Design Decisions 転記と Status 更新、`backend-api-catalog.md` へ API 2行追加（計算不可の扱い・tagged union を利用者向けに明記）、`docs-check`
7. [x] `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / focused tests / 影響統合テスト全件 / coverage 計測（目標80%）
8. [x] Codex へ実装の adversarial review を依頼し、指摘を修正する
9. [x] git commit、Progress Notes 更新、`.agent/PLANS.md` のポインタ更新

## Validation Plan

- [x] `cargo test -p timekeeper-app --test settlement_balance`
- [x] `cargo test -p timekeeper-contract`
- [x] `cargo test -p timekeeper-backend --test settlement_balance_api`（新規）
- [x] `cargo test -p timekeeper-backend --test workday_resolver`（回帰）
- [x] `cargo test -p timekeeper-backend --test work_schedule_read_api`（回帰）
- [x] `cargo test -p timekeeper-backend --test admin_work_schedules_api`（回帰）
- [x] `cargo test -p timekeeper-backend --test attendance_work_schedule_integration`（回帰）
- [x] `cargo test -p timekeeper-backend --lib`（回帰）
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/harness.sh docs-check`
- [x] 新規モジュールの line coverage 計測（目標80%）
- [x] Codex adversarial review

## Git Snapshot Log

- [x] `git status --short`
- [x] focused tests pass
- [x] `git commit` — `51a2c9d feat: add flex settlement balance read model`

## Progress Notes

- 2026-07-30: 実装完了。`CalculateSettlementBalance` use case、5 variant の `status`
  tagged union、既存 effective attendance read を再利用する PostgreSQL adapter、本人・scoped
  manager 向け API 2 本、OpenAPI/API catalog/design doc を追加した。HTTP 統合テストで
  materialize、全計算不可 status、version/type 混在、補正申請→別 manager 承認→locked 月再計算、
  日跨ぎ夜勤と boundary 前打刻の `work_date` 帰属、認可・入力境界を固定した。
  実測: app 6 passed、contract 全 suite green（settlement 2 passed）、settlement API 8 passed、
  admin work schedules 15 passed、attendance/work-schedule integration 8 passed、
  work-schedule read 9 passed、workday resolver 16 passed、Phase 2 API 27 passed、
  backend lib 406 passed。`cargo clippy --workspace --all-targets -- -D warnings`、
  `docs-check`、`fmt-check`、`git diff --check` は green。新規 app module の line coverage は
  100.00%（region 96.51%）で80%目標を達成した。実装レビューは Block 2回の指摘を修正後、
  最終 **Approve**。security review は Critical/High なしで **Approve**。
- 2026-07-03: EP作成（計画のみ、実装未着手）。Phase3 先行2EPの申し送り「清算期間残高の実績突合」を対象とし、実装前に確定すべき設計判断（補正後 effective values の採用、実休憩ベース・予定休憩控除なし、丸めなし、契約値の按分なし、version混在は fail-closed、残高は非保存の導出値、expected_work_minutes 不使用、flexのみ対象、read-only API 2本、既存アーキテクチャ配置）を Design Decisions 節として明文化した。丸め・締め時点固定・給与エクスポートは design doc の Follow-up Designs 1/2/4 へ明示的に委譲した。
- 2026-07-03: Codex（`codex:codex-rescue`エージェント経由、backgroundタスク`task-mr4px414-r21aav`、所要3分40秒）に計画段階のadversarial reviewを依頼した。結果は **Block**（High 2件・Medium 5件・Low 2件）。指摘と対応は以下（すべて本EPへ反映済み。実装は未着手）:
  - **High#1（API契約未定義）**: 計算不可を `200+成功DTO内code` で返すのか `4xx+ErrorResponse` で返すのか未定義で、実装者ごとに挙動が割れる。→ Design Decision 9 で「計算不可はリソースの正当な状態として 200 + tagged union（`status` フィールド）、4xx はリクエスト検証・認可のみ」と確定し、contract round-trip / OpenAPI への固定を Done Criteria 化した。
  - **High#2（月の完全性未定義）**: 既存 read API は materialize 後に読むが、本EPの port は未resolve日の扱いが未定義で、欠損日を無視した部分月の誤計算が可能だった。→ Design Decision 8 で「materialize を先行し、欠損日が残る月は `unresolved_days` で fail-closed」と確定し、resolved 0件月・1日欠損月のテストを Done Criteria 化した。
  - **Medium#1（f64サマリとの突合が曖昧）**: 既存サマリは `total_work_hours: f64` で、「同一の分値」の検証方法が未定義。→ Design Decision 1 を「effective timestamp 差分から整数分を直接計算、保存済み f64 は入力に使わない、テストは分値を独立計算して assert」へ具体化した。
  - **Medium#2（version混在判定の母集団）**: 非勤務日も version 参照を持つため、勤務日限定の判定では非勤務日のみの切替を見逃す。→ 母集団を「対象月の全 resolved workdays」と確定し、ステータス優先順位（unresolved_days → not_applicable → version_mixed → not_configured）も定義した。
  - **Medium#3（月帰属未定義）**: 夜勤の月境界跨ぎ・boundary前打刻の帰属が未定義。→ Design Decision 10 で「`work_date` 基準、暦時刻で分割しない」と確定し、月末夜勤・月初boundary前打刻を Done Criteria 化した。
  - **Medium#4（進行中attendance未定義）**: → Design Decision 11 で「`actual_minutes=0`・`in_progress:true` で内訳に含める。当月途中・未来月は許可」と確定した。
  - **Medium#5（year/month validation）**: → Design Decision 12 で既存月次締めと同一範囲（year 1900..=9999、month 1..=12、範囲外 400 INVALID_WORK_SCHEDULE）と確定し、境界値テストを Done Criteria 化した。
  - **Low#1**: 締め後 correction 反映のテストは直接 DB insert でなく correction 申請→承認 API フロー経由で行うことを Done Criteria に明記した。
  - **Low#2**: `backend-api-catalog.md` の新規2行に混在月・未設定・計算不可の扱いと tagged union を利用者向けに明記することを Done Criteria に追加した。
  - Codex は Design Decisions のうち 2/3/4/6/7/13 を既存実装・design doc と矛盾なしと確認済み（特に判断6「締め後 correction 反映」は、月次締めが `resolved_workdays.locked_at` のみ更新し correction 承認が `attendance_correction_effective_values` へ upsert する現行実装から成立すると裏付けられた）。
