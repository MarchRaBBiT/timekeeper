# EP-20260703-work-schedule-phase3-resolve-workday-flex

## Goal

- Phase3の後続増分として、`ResolveWorkday` use caseをflex対応にする。flexスケジュールのpublished versionが割り当てられたユーザーの日別projectionに `schedule_type` と該当曜日のコアタイムsnapshotを含め、resolved workday読み取りAPI（一覧・カレンダー）から参照可能にする
- [`EP-20260702-work-schedule-phase3-api-wiring.md`](./EP-20260702-work-schedule-phase3-api-wiring.md) Out節の申し送り「`crates/app` の `ResolveWorkday` flex対応出力」のうち、**projection生成と読み出しまで**を扱う（清算期間残高計算は含めない。下記Out参照）

## Scope

- In:
  - `crates/app/src/work_schedules.rs`:
    - `ScheduleVersion` へ `schedule_type: ScheduleType` を追加（`find_published_version` の読み出し拡張）
    - `WorkdayResolutionRepository` port に該当曜日のコアタイム読み出しを追加（`find_day_rule` の戻り値拡張、または `find_core_time_windows(version_id, weekday)` の新設。実装時にシンプルな方を選ぶ）
    - `NewResolvedWorkday` / `ResolvedWorkday` へ `schedule_type` / `core_time_windows: Vec<CoreTimeWindow>` を追加
    - resolverロジック: `schedule_type = Flex` かつ勤務日のときのみコアタイムをsnapshotする。非勤務日（override / 祝日 / non-working rule）は `work_intervals` 同様に空にする
  - `backend/migrations/049_add_resolved_workday_flex.sql`（新規）:
    - `resolved_workdays.schedule_type` カラム（`TEXT NOT NULL DEFAULT 'fixed'`、`CHECK (schedule_type IN ('fixed', 'flex'))`）
    - `resolved_workday_core_time_windows` テーブル: `(resolved_workday_id, weekday)` 主キー（domainの「曜日1件」制約・048の `(version_id, weekday)` 主キーと整合）、`weekday SMALLINT CHECK (BETWEEN 1 AND 7)`、`start_day_offset = 0` / `end_day_offset BETWEEN 0 AND 1` のCHECK、`ON DELETE CASCADE`（044の子テーブルパターンを踏襲）
    - 既存 `prevent_locked_resolved_workday_mutation` と同等のlocked不変トリガーを新子テーブルへ適用
  - `crates/infra-postgres/src/work_schedules/`: 上記port変更に対応するSQL配線
    - published version読み出しへの `schedule_type` 追加、コアタイム取得
    - **projection保存（`save_projection`）の子テーブル置換**: 既存 `resolved_workday_intervals` / `resolved_workday_breaks` と同様に、未locked行の再解決時は同一transaction内で `resolved_workday_core_time_windows` をDELETEしてから再INSERTする（flex勤務日→非勤務日/fixed/コアタイムなし曜日への再解決で古いsnapshotが残らないこと）
    - 読み出し時のassemble: `schedule_type = fixed` なのにコアタイム子行が存在する、または `day_kind != scheduled_workday` なのに子行が存在する不整合データは `CorruptData` エラーで検出する（api-wiring EPのMedium#1対応と同じ方針）
  - `backend` のresolved workday読み取り経路（`/api/admin/users/{user_id}/resolved-workdays`、work-schedule-calendar、**従業員向け `/api/work-schedules/me`** — `get_my_workdays` は同じ `ResolvedWorkdayResponse` を返す共通経路）で使うrepository/rowの拡張
  - `crates/contract`: `ResolvedWorkdayResponse` へ `schedule_type: WorkScheduleType` / `core_time_windows: Vec<CoreTimeWindowResponse>` を追加（response-only追加のため後方互換）+ round-tripテスト
  - `docs/design-docs/backend-api-catalog.md` の該当行、`docs/design-docs/work-schedule-master.md` のStatus/Implementation Status更新
  - テスト: app unit（RED→GREEN）、infra-postgres integration、backend integration（flex version作成→publish→assign→resolve→read APIで検証）、contract round-trip
- Out（後続EPまたは対象外）:
  - 清算期間残高の実績突合・flexの期待勤務時間の契約ベース計算 — `docs/design-docs/work-schedule-master.md` Follow-up Designsの「勤怠計算ポリシー」設計に依存するため明示的にOut。清算期間はpublished version（不変）への参照で辿れるため、projectionへのsnapshotは行わない
  - flex特有のanomaly検出（コアタイム欠勤・清算期間不足の検出）
  - 打刻→workday接続ロジック（`attendance_work_schedule_integration`）の挙動変更
  - frontend、変形労働・複数勤務区間（split shift）、シフト一括作成・交換

## Done Criteria (Observable)

- [x] flexのpublished versionが割り当てられたユーザーの `ResolveWorkday` は、`schedule_type = flex` と該当曜日のコアタイムsnapshotを持つprojectionを保存する（backend統合テスト: 作成→publish→assign→resolve→GETのエンドツーエンド）
- [x] fixedスケジュールのprojectionは `schedule_type = fixed`・コアタイムなしで、既存の全フィールドが従来と同一（回帰テスト）
- [x] flexの非勤務日（override non-working / 祝日 / non-working rule）のprojectionはコアタイムを持たない
- [x] **未lockedのflex勤務日projection（コアタイムあり）を override non-working / fixed schedule / コアタイムなし曜日として再解決すると、`resolved_workday_core_time_windows` が空に置換される**（Codexレビュー Highの指摘: 子テーブルDELETE+INSERTの置換をテストで固定）
- [x] コアタイムのない曜日・フルフレックスのflex勤務日は空 `core_time_windows` で正常にresolveされる
- [x] 日跨ぎコアタイム（`end_day_offset = 1`）がsnapshot・読み出しでround-tripする（**日曜の日跨ぎ（週境界）を含む**）
- [x] **`effective_from` 境界で異なるコアタイムを持つ2つのpublished versionがある場合、対象日ごとに正しいversionのコアタイムがsnapshotされる**
- [x] locked済みprojectionは再解決されず、**後続のpublished version追加・assignment変更にも追従せず元の `work_schedule_version_id` を保持し、そのversion参照でsettlement periodを取得できる**（清算期間をsnapshotしない設計判断の再現性根拠をテストで固定）
- [x] `resolved_workday_core_time_windows` へのINSERT/UPDATE/DELETEはlocked時にDBトリガーで拒否される
- [x] `schedule_type = fixed` なのにコアタイム子行がある、または非勤務日なのに子行がある不整合データは読み出し時に `CorruptData` エラーになる（silentな露出にならない）
- [x] migration適用後、既存の `resolved_workdays` 行は `schedule_type = fixed`・空コアタイムとして読み出せる（後方互換）
- [x] `GET /api/admin/users/{user_id}/resolved-workdays`、work-schedule-calendar、**`GET /api/work-schedules/me`** のレスポンスに `schedule_type` / `core_time_windows` が含まれ、既存fixedレスポンスは `fixed` + 空配列になる
- [x] `expected_work_minutes` の計算方式は不変（flexでは区間=flex bandの幅−休憩、すなわち最大稼働可能幅を表す）であることをテストで固定し、**契約所定時間ではないことを `work-schedule-master.md` と `backend-api-catalog.md` の両方に明記する**
- [x] **anomaly検出と打刻→workday接続がflexの `expected_work_minutes`（最大幅）を欠勤・不足判定に誤用しない**ことを回帰テストで確認する（現行は `day_kind` + attendanceのみ参照だが、flex projectionを与えても挙動が変わらないことを固定）
- [x] 既存テスト（`resolve_workday` / `workday_resolver_repository` / `workday_resolver` / `admin_work_schedules_api` / `work_schedule_read_api` / `attendance_work_schedule_integration` 等）に回帰がない

## Constraints / Non-goals

- SQLx migrationは既存ファイル編集でなく新規追加（`049_...`）
- locked projection不変性の方針を踏襲する: applicationレイヤ（`locked_at` チェック）とDBトリガーの両方で守り、新子テーブルにも同じトリガー関数を適用する
- `crates/domain` の `FlexPolicy` / `validate_flex_policy` は変更しない。resolverはdomain型（`CoreTimeWindow`）を再利用し、新しい妥当性ルールを追加しない（published versionは作成時に検証済み）
- `expected_work_minutes()` の意味（区間−休憩）は変更しない。flexにおける契約ベースの期待値は「勤怠計算ポリシー」設計後の後続EPで扱う
- 清算期間はprojectionへsnapshotしない。published versionはDBトリガーで不変のため、`work_schedule_version_id` 経由の参照で再現性が保たれる（コアタイムをsnapshotするのは、日別の実効値として `work_intervals` / `planned_breaks` と対称に扱うため）
- contract変更はresponseへのフィールド追加のみで、request DTOは変更しない（silent-ignoreリスクの新規発生なし）
- 既存published version・locked projectionには一切影響しない

## Task Breakdown

1. [x] app unit test（RED）: `crates/app/tests/resolve_workday.rs` へflexケース（勤務日snapshot、非勤務日は空、コアタイムなし曜日、**再解決でのsnapshot置換**、fixed回帰）を追加する
2. [x] app実装（GREEN）: port拡張・`NewResolvedWorkday`/`ResolvedWorkday` フィールド追加・resolverロジック
3. [x] migration `049_add_resolved_workday_flex.sql` 新規作成（カラム・`(resolved_workday_id, weekday)` 主キー子テーブル・CHECK制約・lockedトリガー）
4. [x] infra-postgres: SQL配線とintegration test（snapshot保存・**未locked再解決時の子テーブルDELETE+INSERT置換（同一transaction）**・読み出し・不整合データの `CorruptData` 検出・lockedトリガー・既存行後方互換）
5. [x] contract: `ResolvedWorkdayResponse` 拡張 + round-tripテスト
6. [x] backend read経路の配線とintegration test（flexエンドツーエンド、fixed回帰、カレンダー埋め込み、**`/api/work-schedules/me`**、**version境界・週境界ケース**、**locked projectionのversion参照再現性**）
7. [x] anomaly検出・打刻接続の回帰テスト: flex projection投入時に `expected_work_minutes`（最大幅）が欠勤・不足判定へ影響しないことを固定する
8. [x] `docs/design-docs/backend-api-catalog.md`（`expected_work_minutes` が契約所定時間でない旨を含む）/ `work-schedule-master.md` 更新、OpenAPI schema（`backend/src/docs.rs`）反映、`docs-check`
9. [x] `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / focused tests / 影響統合テスト全件
10. [x] Codexへadversarial reviewを依頼し、指摘を修正する
11. [x] git commit、Progress Notes更新、`.agent/PLANS.md` のポインタ更新

## Validation Plan

- [x] `cargo test -p timekeeper-app --test resolve_workday`（16 passed）
- [x] `cargo test -p timekeeper-infra-postgres --test workday_resolver_repository`（2 passed、trait実装検証のみ。実DB挙動は`backend/tests/workday_resolver.rs`のtestcontainers統合テストで検証）
- [x] `cargo test -p timekeeper-contract`（10 flex round-tripテスト含む）
- [x] `cargo test -p timekeeper-backend --test workday_resolver`（16 passed、flex snapshot・再解決置換2種・corrupt-data検出2種・locked trigger・version境界・週境界・locked版versionn参照維持を含む）
- [x] `cargo test -p timekeeper-backend --test work_schedule_read_api`（9 passed、`/api/work-schedules/me` のflex/fixed両方を含む）
- [x] `cargo test -p timekeeper-backend --test admin_work_schedules_api`（15 passed、回帰）
- [x] `cargo test -p timekeeper-backend --test work_schedule_phase2_api`（14 passed、anomaly回帰含む）
- [x] `cargo test -p timekeeper-backend --test attendance_work_schedule_integration`（8 passed、回帰 + flex誤用なし確認）
- [x] OpenAPI schema検証（`docs_api.rs` 3 passed。utoipa `ToSchema` deriveにより自動反映を確認）
- [x] `cargo test -p timekeeper-backend --lib`（373 passed、回帰）
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/harness.sh docs-check`
- [x] 新規/変更モジュールのline coverage計測（`work_schedules.rs`: region 87.38% / line 95.08%。目標80%達成）
- [x] Codex adversarial review（計画レビュー: Block→全反映後実装。実装レビュー: Approve with follow-ups→指摘3件を修正）

## Git Snapshot Log

- [x] `git status --short`
- [x] focused tests pass
- [x] `git commit`

## Progress Notes

- 2026-07-03: EP作成。Phase3の残増分を「resolver flex対応出力（本EP）」と「清算期間残高・勤怠計算ポリシー（後続EP、design doc Follow-up Designs依存）」に分割し、本EPはprojection生成〜読み取りAPIまでに限定した。清算期間はsnapshotせずversion参照とする設計判断を明記（published versionのDBレベル不変性が根拠）。
- 2026-07-03: Codex（`codex:codex-rescue`エージェント経由）に計画段階のadversarial reviewを依頼した。結果は **Block**（High 1件・Medium 6件・Low 2件）。指摘と対応は以下（すべて本EPの計画に反映済み。実装は未着手）:
  - **High**: 未locked projectionの再解決時にコアタイム子テーブルの置換（DELETE+INSERT）手順が計画に無く、flex勤務日→非勤務日/fixed/コアタイムなし曜日への再解決で古いsnapshotがAPIへ露出する経路があった。→ Scopeの`save_projection`に同一transaction内での置換を明記し、Done Criteria・Task Breakdownへ置換の回帰テストを追加した。
  - **Medium#1**: `schedule_type` とコアタイム子行の整合性検出方針が無かった。→ api-wiring EPと同じ方針（repository assemble時に `CorruptData`）をScope・Done Criteriaへ追加した。
  - **Medium#2**: 従業員向け `/api/work-schedules/me`（`get_my_workdays`、同じ`ResolvedWorkdayResponse`を返す共通経路）が影響範囲から漏れていた。→ Scope・Done Criteria・Validation Planへ追加した。
  - **Medium#3**: 子テーブル主キーが`(resolved_workday_id, sequence)`で、domainの「曜日1件」制約・048の`(version_id, weekday)`と不整合だった。→ `(resolved_workday_id, weekday)` 主キーへ修正した。
  - **Medium#4**: 清算期間をsnapshotしない判断の再現性根拠（locked projectionが後続version/assignment変更後も元version_idを保持しsettlementへ辿れること）を固定するテストが無かった。→ Done Criteriaへ追加した。
  - **Medium#5**: `expected_work_minutes`（flexでは最大幅）の誤解釈対策がdocs中心で下流回帰テストが不足していた。→ anomaly・打刻接続の回帰テストをDone Criteria・Task Breakdownへ追加し、API catalogへの「契約所定時間ではない」明記もタスク化した。
  - **Medium#6**: 週境界（日曜の日跨ぎコアタイム）・published version切替境界（`effective_from`前後で異なるコアタイム）・locked済みprojectionが新versionへ追従しないことのテストが計画に無かった。→ Done Criteriaへ3ケースとも追加した。
  - **Low#1**: migration 049のDDL詳細（CHECK制約・CASCADE）が不足。→ Scopeへ044/048パターン踏襲の具体制約を明記した。
  - **Low#2**: `ResolvedWorkdayResponse`拡張のOpenAPI影響確認が弱い。→ `backend/src/docs.rs` のschema反映確認をValidation Planへ追加した。
- 2026-07-03: 計画どおり実装した。`crates/app/src/work_schedules.rs`に`ScheduleVersion.schedule_type`・`WorkdayResolutionRepository::find_core_time_window`・`NewResolvedWorkday`/`ResolvedWorkday`の`schedule_type`/`core_time_windows`を追加し、resolverロジックで`schedule_type=Flex`かつ勤務日のときのみコアタイムをsnapshotするようにした。migration `049_add_resolved_workday_flex.sql`で`resolved_workdays.schedule_type`カラムと`resolved_workday_core_time_windows`子テーブル（`(resolved_workday_id, weekday)`主キー、044の`prevent_locked_resolved_workday_child_mutation`トリガーを再利用）を追加。`crates/infra-postgres`の`persistence.rs::replace_children`で未locked再解決時にコアタイムも同一transaction内でDELETE+INSERT置換し、`rows.rs::assemble_resolved_workday`で`schedule_type=fixed`×コアタイムあり、非勤務日×コアタイムありの不整合を`InvalidScheduleData`で検出するようにした。`crates/contract`の`ResolvedWorkdayResponse`へ`schedule_type`/`core_time_windows`をresponse-only追加し、`backend/src/handlers/work_schedules.rs`の`resolved_workday_to_response`（`/api/work-schedules/me`・admin resolved-workdays・calendarの共通経路）で変換した。
- 2026-07-03: テスト追加。app unit 16件（既存10件+flex新規6件）、contract 10件（flex round-trip 2件新規）、backend統合: `workday_resolver.rs`にflex snapshot・非勤務日空・フルフレックス・週境界（日曜日跨ぎ）・version境界の5件、`work_schedule_read_api.rs`に`/api/work-schedules/me`のflex/fixed各1件、`work_schedule_phase2_api.rs`にanomaly誤用なし回帰1件、`attendance_work_schedule_integration.rs`にclock-in flex回帰1件を追加。docsは`backend-api-catalog.md`（該当3行、`expected_work_minutes`が契約所定時間でない旨含む）・`work-schedule-master.md`（Status/Implementation Status/Phase3ロードマップ）を更新。`cargo fmt --all --check`/`cargo clippy --workspace --all-targets -- -D warnings`/`bash scripts/harness.sh docs-check`すべてgreen。`work_schedules.rs`のline coverageはregion 87.38%/line 95.08%で目標80%を達成。
- 2026-07-03: Codex（`codex:codex-rescue`エージェント経由、backgroundタスク`task-mr4cgdq5-bibj7e`、所要16分12秒）に実装のadversarial reviewを依頼した。結果は **Approve with follow-ups**（実行時のブロッカーなし。Medium 1件・Low 2件）。指摘と対応は以下（すべて反映済み）:
  - **Medium**: `docs/design-docs/backend-api-catalog.md`の`ReplaceWorkScheduleVersionRequest`行が`schedule_type? = "fixed"`（省略可能）と誤記していたが、実際のcontractは`#[serde(default)]`無しで必須（既存`replace_without_schedule_type_is_rejected_instead_of_silently_downgrading_flex`テストが固定済み）。このEPの変更範囲外の既存ドリフトだったが、レビュー中に発見されたため合わせて修正し、`必須。省略不可`と明記した。
  - **Low#1**: corrupt-data検出（`schedule_type=fixed`×コアタイムあり、非勤務日×コアタイムあり）を直接exerciseするテストが無かった。→ `backend/tests/workday_resolver.rs`に`corrupt_fixed_projection_with_core_time_rows_errors_on_read`・`corrupt_non_workday_projection_with_core_time_rows_errors_on_read`を追加し、手動でコアタイム行を挿入して`ResolveWorkdayError::InvalidScheduleData`が返ることを固定した。また、再解決置換のテストがoverride-to-non-workingの1パターンのみだったため、`re_resolution_from_flex_to_fixed_schedule_clears_core_time_window`（flex→fixed、ユーザーレベル優先割当による切替）・`re_resolution_to_a_different_flex_schedule_without_core_time_clears_stale_window`（flex→別flex（コアタイムなし）、published versionの不変性のため同一version内での書き換えは不可能と判明し割当スワップ方式に設計変更）を追加した。
  - **Low#2**: `work-schedule-master.md`の旧エントリ（2026-07-02付、Phase3 flex-core-time EPとapi-wiring EP時点）が「`ResolveWorkday`のflex対応出力は未実装」と書いたままで、直後の新エントリ（2026-07-03、実装済み）と矛盾して見えた。→ 旧エントリへ「当時未実装」+後続エントリで実装済みである旨の注記を追加。また`## Persistence Design`のテーブル一覧にPhase3で追加した3テーブル（`work_schedule_settlement_periods`/`work_schedule_core_time_windows`/`resolved_workday_core_time_windows`）が漏れていたため追加した。
  - 修正後: `cargo test -p timekeeper-backend --test workday_resolver`（16 passed、corrupt-data 2件・re-resolution variant 2件を追加）、`cargo fmt --all --check`/`cargo clippy --workspace --all-targets -- -D warnings`/`bash scripts/harness.sh docs-check`継続green。全対象テスト（app 16 + contract 10 + backend lib 373 + workday_resolver 16 + work_schedule_read_api 9 + admin_work_schedules_api 15 + work_schedule_phase2_api 14 + attendance_work_schedule_integration 8 + docs_api 3 = 464件）green。
