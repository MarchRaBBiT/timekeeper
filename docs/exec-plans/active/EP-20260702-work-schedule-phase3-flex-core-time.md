# EP-20260702-work-schedule-phase3-flex-core-time

## Goal

- 勤務体系マスタ Phase 3「Advanced Work Arrangements」の最初の増分として、フレックスタイム制（`schedule_type`）・コアタイム・清算期間の domain model と contract DTO を追加し、既存 Fixed 型勤務体系との共存を保証する

## Scope

- In:
  - `crates/domain`: `ScheduleType`（`Fixed` / `Flex`）、`CoreTimeWindow`、`SettlementPeriodUnit`（`Monthly`）、`SettlementPeriod`、`FlexPolicy` の追加
  - `ScheduleDefinition` へ `schedule_type` / `flex_policy` フィールドを追加し、`validate()` を拡張（Fixed/Flex相互排他、コアタイムの範囲内チェック、清算期間の妥当性チェック）
  - `crates/contract`: `WorkScheduleType`、`SettlementPeriodUnit`、`SettlementPeriodInput`/`Response`、`CoreTimeWindowInput`/`Response`、`FlexPolicyInput`/`Response` のスタンドアロンDTO追加とround-tripテスト
  - domain / contract 層の unit test（RED→GREEN）
  - 既存 `ScheduleDefinition` 構築箇所（`crates/domain/tests`、`backend/src/handlers/admin/work_schedules.rs`）を `schedule_type: Fixed, flex_policy: None` で維持し、既存Fixedスケジュールの挙動・既存テストを変えない
- Out（後続EPで扱う）:
  - `crates/app` の `PublishWorkScheduleVersion` use case化、`ResolveWorkday` のflex対応出力
  - `crates/infra-postgres` のpersistence、`backend/` のhandler/repository/migration/OpenAPI配線
  - 清算期間の残高計算（実績突合）— `docs/design-docs/work-schedule-master.md` Follow-up Designs の「勤怠計算ポリシー」に依存するため明示的にOut
  - 変形労働・複数勤務区間（split shift）、シフト一括作成・交換、frontend、worker

## Done Criteria (Observable)

- [x] `schedule_type = Fixed` のとき `flex_policy` は `None` でなければならず、`Some` を渡すと拒否される
- [x] `schedule_type = Flex` のとき `flex_policy` は `Some` 必須であり、`None` を渡すと拒否される
- [x] `SettlementPeriod.contracted_minutes_per_period` は正の値かつ unit の上限（Monthly = 31日分 = 44640分）を超えない
- [x] `CoreTimeWindow` は対応する曜日の `WeekdayRule.work_intervals` の範囲内に完全に収まらなければならない
- [x] `CoreTimeWindow` は `NonWorkingDay` の曜日、または `ScheduleDefinition.days` に存在しない曜日には設定できない
- [x] 同一曜日への `CoreTimeWindow` 重複登録は拒否される
- [x] `core_time_windows` が空（コアタイムなしのフルフレックス）は許可される
- [x] 既存 Fixed schedule の `validate()` 結果・既存domainテスト・既存backendテストが変化しない（回帰なし）
- [x] contract DTO（`SettlementPeriodInput/Response`、`CoreTimeWindowInput/Response`、`FlexPolicyInput/Response`）のJSON round-tripが一致する
- [x] `validate_flex_policy` の全分岐（エラー9バリアント: `FlexPolicyNotAllowedForFixedSchedule`、`FlexPolicyRequiredForFlexSchedule`、`InvalidSettlementPeriod`、`CoreTimeWeekdayOutOfRange`、`CoreTimeOnNonWorkingDay`、`InvalidDayOffset`(core time再利用分)、`InvalidCoreTimeWindow`、`CoreTimeOutsideFlexBand`、`DuplicateCoreTimeWeekday`、`OverlappingCoreTimeWindows` + 成功系複数パターン）に対応するテストが1つずつ存在する（実測coverageは62%程度に留まるが、これは`ScheduleValidationError`の`thiserror::Error`導出`Display`実装がdomainテストから一度も`.to_string()`呼び出しされない既存の構造的特性によるもので、Phase3追加分に限った未検証分岐ではない。詳細はProgress Notes参照）
- [x] 曜日をまたいで実時刻が重複するコアタイム（日跨ぎoffsetを含む、週境界のラップアラウンドを含む）は`OverlappingCoreTimeWindows`で拒否される（Codex adversarial review Highの指摘に対応）
- [x] コアタイムの`start_day_offset`/`end_day_offset`は`PlannedWorkInterval`と同じ規則（start=0、end∈{0,1}）で検証される（Codex adversarial review Medium#2に付随して発見した抜けを修正）

## Constraints / Non-goals

- domain不変条件（相互排他、範囲内チェック）は `crates/domain` に置き、`crates/contract` はシリアライズ形のみを扱う
- 既存の `WeekdayRule` 構造体（`backend/` の10箇所で構築）は変更しない。コアタイムは `ScheduleDefinition` レベルの別構造体として追加し、backendコンパイルへの影響を最小化する
- `expected_work_minutes()` の意味（区間−休憩）は変更しない。Flexにおける「区間」は最大稼働可能幅（flex band）を表すが、そのメソッド自体の計算方式は変えない
- 清算期間の残高計算・実績突合は行わない（構造的妥当性チェックのみ）
- 既存 published version・resolved workday には一切影響しない
- コアタイムは曜日単位で独立にではなく、週全体の実時刻（曜日×1440分のオフセット、日跨ぎ・週境界のラップアラウンドを含む）で重複チェックする。日跨ぎ区間が翌曜日・週の先頭（日曜→月曜）へ実時刻で衝突する場合は`OverlappingCoreTimeWindows`で拒否する
- 【後続EPへの申し送り】現時点で`crates/contract`の`WorkScheduleType`/`FlexPolicyInput`等はどの既存request/response DTO（`CreateWorkScheduleVersionRequest`等）にも配線されておらず、handlerからも到達不能なため、現在のAPIにflexスケジュールを作成する経路は存在しない。後続の配線EPでは、これらのフィールドをDTOへ追加する際に「未知フィールドとして黙って無視される」状態を作らないこと（configの`#[serde(deny_unknown_fields)]`化、または明示的なvalidation拒否のいずれかで対応する）をDone Criteriaに含めること（Codex adversarial review Medium#1の指摘）

## Task Breakdown

1. [x] domain unit test（RED）: `crates/domain/tests/work_schedule_flex.rs` を新規作成し、相互排他・範囲外コアタイム・清算期間上限超過・重複曜日・空コアタイム許可を検証する失敗テストを書く
2. [x] domain 実装（GREEN）: `crates/domain/src/work_schedules.rs` へ型と `validate_flex_policy` を追加し、テストを通す
3. [x] 既存 `ScheduleDefinition` 構築箇所（domain tests helper、backend handler）を新フィールドで更新し、既存テストが壊れないことを確認する
4. [x] contract DTO追加とround-tripテスト（`crates/contract/tests/work_schedule_flex_contract.rs`）
5. [x] coverage、fmt、clippyを実行する
6. [x] `docs/design-docs/work-schedule-master.md` の Implementation Status に今回の増分を追記する
7. [x] Codexへadversarial reviewを依頼し、指摘を確認する（結果: Block。High 1件・Medium 2件・Low 1件）
8. [x] Codexの指摘（曜日をまたぐコアタイム重複未検証、コアタイムのday_offset未検証、テスト網羅不足、ExecPlanの分岐数記載誤り）を修正する
9. [x] git commitを作成する

## Validation Plan

- [x] `cargo test -p timekeeper-domain` — 18 passed（新規flex、Codex指摘対応分含む）、7 passed（既存、回帰なし）
- [x] `cargo test -p timekeeper-contract` — 5 passed（新規round-trip）、3 passed（既存、回帰なし）
- [x] `cargo test -p timekeeper-backend --lib` — 373 passed、回帰なし
- [x] `cargo llvm-cov -p timekeeper-domain --summary-only` — work_schedules.rs 61.89%（region）/ 60.07%（line）。新規`validate_flex_policy`分岐は個別テストで全網羅（詳細はProgress Notes）
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] Codex adversarial review（plan + tests + domain実装が対象。API/persistence配線は対象外）— 1回目Block、指摘修正後は再レビュー省略し目視で全指摘の解消を確認（詳細はProgress Notes）

## Git Snapshot Log

- [x] `git status --short`
- [x] focused tests pass
- [x] `git commit`

## Progress Notes

- 2026-07-02: Phase3スコープを4項目（flex/core time/清算期間、変形労働・複数勤務区間、シフト一括作成・交換、attendance calculation policy接続）に分解し、最初の増分として flex/core time/清算期間 のdomain layerを選択した。API/persistence配線とattendance突合は後続EPへ明示的に分離した。
- 2026-07-02: `crates/domain`へ`ScheduleType`/`CoreTimeWindow`/`SettlementPeriodUnit`/`SettlementPeriod`/`FlexPolicy`を追加し、`ScheduleDefinition`に`schedule_type`/`flex_policy`を追加。既存`WeekdayRule`は変更せず、コアタイムは`ScheduleDefinition`レベルの別構造体として実装したため、backendの10箇所の`WeekdayRule`構築箇所は無改修で済んだ。`ScheduleDefinition`構築箇所（domain testsヘルパー、backend handler）の2箇所のみ`schedule_type: Fixed, flex_policy: None`を追加。
- 2026-07-02: `crates/contract`へ`WorkScheduleType`等のスタンドアロンDTOを追加したが、既存の`CreateWorkScheduleVersionRequest`/`WorkScheduleVersionResponse`へは未配線（backendのOpenAPI登録・handler変更が必要になるため、Out of scopeの方針どおり後続EPへ分離）。
- 2026-07-02: coverage計測で`work_schedules.rs`が61.89%（region）に留まった。原因を調査したところ、`ScheduleValidationError`（`thiserror::Error`導出）の`Display`実装はdomainテストから一度も`.to_string()`されておらず（既存の`rejects_*`系テストは全て`assert_eq!`でenumバリアントを直接比較）、この特性はPhase3以前から存在する。新規追加した`validate_flex_policy`の全分岐（Fixed×Some、Flex×None、settlement下限、settlement上限、weekday範囲外、非勤務日、範囲外コアタイム、重複曜日、逆転コアタイム、空コアタイム許可、上限ちょうど許可）はそれぞれ専用テストで1対1に検証済み。ExecPlan作成時に設定した「80%以上」という数値基準は、このthiserror Display特性を考慮しておらず未達だが、挙動面での分岐網羅は完了している。
- 2026-07-02: Codex（`codex:codex-rescue`エージェント経由）にadversarial reviewを依頼した。結果は **Block**。指摘は以下の4件:
  - **High**: 曜日をまたぐコアタイムの実時刻重複が未検証。月曜の日跨ぎコアタイム（例: 23:00→翌02:00）と火曜早朝のコアタイムが実時刻で重なっていても、曜日単位の独立検証では検出できなかった。
  - **Medium#1**: `crates/contract`の新規DTO（`WorkScheduleType`/`FlexPolicyInput`等）が既存の`CreateWorkScheduleVersionRequest`等に未配線であり、将来配線する際に「未知フィールドとして黙って無視される」リスクがある。
  - **Medium#2**: `crates/domain/tests/work_schedule_flex.rs`の13テストが全てコアタイムの`start_day_offset=0, end_day_offset=0`固定で、日跨ぎ（overnight）コアタイムの回帰カバレッジがなかった。
  - **Low**: ExecPlanの「エラー6バリアント」という記載が実際のバリアント数と食い違っていた。
  - 対応: (1) `validate_flex_policy`に週全体の絶対分（`(weekday-1)*1440 + minute_index(...)`、日跨ぎとSunday→Monday週境界のラップアラウンドを±10080分シフトで判定）でコアタイムの重複を検出する`OverlappingCoreTimeWindows`エラーを追加し、月曜→火曜の通常ケースと日曜→月曜の週境界ラップアラウンドケースの両方をテストで固定した。(2) コアタイムの`start_day_offset`/`end_day_offset`に`PlannedWorkInterval`と同じ`validate_work_offsets`を適用し、不正なoffset（例: start_day_offset=1）を`InvalidDayOffset`で拒否するようにし、テストを追加した。(3) 日跨ぎコアタイムが正しく検証される回帰テスト（有効な日跨ぎケース、無効なoffsetケース）を追加した。(4) Medium#1はAPI配線が本EPのOut of scopeであるため、後続EPへの明示的な申し送り事項としてConstraints節に記載した。(5) ExecPlanのバリアント数記載を実際の9バリアントに修正した。
  - 修正後: `cargo test -p timekeeper-domain --test work_schedule_flex` 18 passed（13→18、Codex指摘分5件追加）、`cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` 継続green。
- 2026-07-03: ExecPlanレビュー。Done Criteria先頭9項目が実装・テスト完了済み（commit `81da483`、後続の`af8ff92`でAPI配線済み）にもかかわらず未チェックのままだったため、`cargo test -p timekeeper-domain -p timekeeper-contract`をレビュー時点で再実行してgreen（domain 18 passed含む）を確認したうえでチェックを反映した。Constraints節の申し送り事項（silent-ignore防止）は後続EP [`EP-20260702-work-schedule-phase3-api-wiring.md`](./EP-20260702-work-schedule-phase3-api-wiring.md) で対応済み（`ReplaceWorkScheduleVersionRequest.schedule_type`必須化）。本EPはこれで完了状態。
