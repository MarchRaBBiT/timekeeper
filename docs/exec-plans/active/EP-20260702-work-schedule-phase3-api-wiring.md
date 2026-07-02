# EP-20260702-work-schedule-phase3-api-wiring

## Goal

- Phase3で追加したflex/core-time/清算期間のdomain・contract型を、実際のWork Schedule Version API（`crates/contract`のrequest/response DTO、`backend`のhandler/repository/migration）に配線し、`schedule_type = Flex`のバージョンをAPI経由で作成・取得できるようにする
- [`EP-20260702-work-schedule-phase3-flex-core-time.md`](./EP-20260702-work-schedule-phase3-flex-core-time.md) Constraints節の申し送り事項（未知フィールドとして黙って無視される状態を作らない）に対応する

## Scope

- In:
  - `crates/contract`: `CreateWorkScheduleVersionRequest` / `ReplaceWorkScheduleVersionRequest` / `WorkScheduleVersionResponse` へ `schedule_type`（デフォルト`Fixed`、後方互換）・`flex_policy`（`Option`）を追加
  - `backend/src/handlers/admin/work_schedules.rs`: `validate_version_definition` を実際のpayload値（`schedule_type`/`flex_policy`）で domain `ScheduleDefinition` を構築するよう変更（現状ハードコードされている `Fixed`/`None` を廃止）
  - `backend/migrations`: `work_schedule_versions.schedule_type` カラム追加、`work_schedule_settlement_periods` / `work_schedule_core_time_windows` テーブル追加（新規migrationファイル）
  - `backend/src/repositories/work_schedule/versions.rs` + `rows.rs`: create/replace/find でflex policyの永続化・読み出しを実装
  - `docs/design-docs/backend-api-catalog.md` のWork Schedule Version該当行を更新
  - contract round-trip test、backend handler unit test、backend integration test（flex versionの作成→取得）
- Out:
  - `crates/app` の `ResolveWorkday` flex対応出力（清算期間残高計算含む）— Phase3後続EPで扱う
  - frontend
  - 変形労働・複数勤務区間（split shift）、シフト一括作成・交換

## Constraints / Non-goals

- 既存Fixedスケジュールの挙動・既存テスト（`admin_work_schedules_api.rs`等）を壊さない。`schedule_type`未送信時は`Fixed`として扱う
- domain不変条件チェックは `crates/domain::ScheduleDefinition::validate()` に委譲する。handlerは変換のみ行う
- SQLx migrationは既存ファイル編集でなく新規追加（`048_...`）
- Published versionの不変性トリガー（`work_schedule_versions_immutable`）の挙動は変えない

## Task Breakdown

1. [x] contract: `WorkScheduleType` に `Default`（`Fixed`）実装、`CreateWorkScheduleVersionRequest`/`ReplaceWorkScheduleVersionRequest`/`WorkScheduleVersionResponse` へフィールド追加 + round-tripテスト
2. [x] backend handler: `validate_version_definition` の引数拡張、contract→domain flex変換ヘルパー追加、既存ユニットテストの回帰確認
3. [x] migration: `048_add_work_schedule_flex_policy.sql` 新規作成
4. [x] repository: `versions.rs`/`rows.rs` でflex policyのinsert/update/select配線
5. [x] `docs/design-docs/backend-api-catalog.md` 更新
6. [x] backend integration test: flexバージョン作成→取得のハッピーパス、Fixed×flex_policy送信時の422、既存fixedフローの回帰なし確認
7. [x] `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test -p timekeeper-contract` / `cargo test -p timekeeper-backend --lib` / `cargo test -p timekeeper-backend --test admin_work_schedules_api`
8. [x] Codexへadversarial reviewを依頼
9. [ ] git commit、Progress Notes更新

## Validation Plan

- [x] `cargo test -p timekeeper-contract`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --test admin_work_schedules_api`（testcontainers/podman経由）
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] Codex adversarial review

## Progress Notes

- 2026-07-02: EP作成。Phase3 domain/contract layerの後続として、API配線（DTO・handler・migration・repository）をスコープとする。
- 2026-07-02: `crates/contract`の`CreateWorkScheduleVersionRequest`/`ReplaceWorkScheduleVersionRequest`に`schedule_type`（`#[serde(default)]`で省略時`Fixed`、後方互換）・`flex_policy: Option<FlexPolicyInput>`を追加し、`WorkScheduleVersionResponse`に`schedule_type`/`flex_policy: Option<FlexPolicyResponse>`を追加した。round-tripテスト2件を`work_schedule_flex_contract.rs`に追加。既存`work_schedule_contract.rs`の構築箇所を更新。
- 2026-07-02: `backend/src/handlers/admin/work_schedules.rs`の`validate_version_definition`をhardcoded `Fixed`/`None`から実際のpayload値（`schedule_type`/`flex_policy`）を使うよう変更。引数が9個になりclippy `too_many_arguments`に抵触したため`VersionDefinitionInput`構造体へリファクタ。contract→domain変換ヘルパー（`domain_schedule_type`/`domain_flex_policy`）を追加。domain不変条件チェックは引き続き`ScheduleDefinition::validate()`に委譲。
- 2026-07-02: migration `048_add_work_schedule_flex_policy.sql`で`work_schedule_versions.schedule_type`カラム（デフォルト`'fixed'`）、`work_schedule_settlement_periods`（`version_id`主キー）、`work_schedule_core_time_windows`（`(version_id, weekday)`複合主キー）を追加。
- 2026-07-02: `repositories/work_schedule/versions.rs`の`create_version`/`replace_version`にschedule_type挿入・flex_policy挿入（`insert_flex_policy`/`insert_core_time_window`）を追加。`replace_version`は既存の`work_schedule_day_rules`削除と同様に`work_schedule_settlement_periods`/`work_schedule_core_time_windows`を削除してから再insertする。`find_version`はsettlement_period（`fetch_optional`）とcore_time_windows（`fetch_all`）を追加取得し、`rows.rs`の`assemble_version`でFlexPolicyResponseを組み立てる。
- 2026-07-02: `docs/design-docs/backend-api-catalog.md`のWork Schedule Version該当3行、`docs/design-docs/work-schedule-master.md`のStatus/Implementation Status/Phase3ロードマップを更新し、API配線完了を反映した。
- 2026-07-02: backend統合テスト`admin_work_schedules_api.rs`に3件追加（flexバージョン作成→取得のハッピーパス、Fixed×flex_policy送信時の`422 INVALID_SCHEDULE_INTERVALS`、既存fixedペイロードで`schedule_type`が`fixed`にデフォルトされ`flex_policy`が`null`になることの回帰確認）。全14件green。既存`work_schedule_phase2_api.rs`/`work_schedule_read_api.rs`/`attendance_work_schedule_integration.rs`（計27件）も回帰なし。
- 2026-07-02: `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test -p timekeeper-contract` / `cargo test -p timekeeper-backend --lib`（373 passed）green。
- 2026-07-02: Codex（`codex:codex-rescue`エージェント経由）にadversarial reviewを依頼した。結果はHigh 1件・Medium 2件（並行性/トランザクション・エラーハンドリング内部情報漏洩・domain変換の正確性は問題なし）。指摘は以下:
  - **High**: `ReplaceWorkScheduleVersionRequest`の`schedule_type`/`flex_policy`が両方`#[serde(default)]`だったため、既存flex draftバージョンを`schedule_type`未送信のクライアントが`PUT`すると、silentにFixedへdowngradeされflex policyが失われる経路が存在した。
  - **Medium#1**: DBスキーマが`work_schedule_versions.schedule_type`と子テーブル（`work_schedule_settlement_periods`/`work_schedule_core_time_windows`）の相互整合性を保証しておらず、将来のrepository変更・保守SQLで不整合データが書き込まれても検出されない。
  - **Medium#2**: published version不変性トリガー（`work_schedule_versions_immutable`）が本体テーブルのみを対象とし、新規flex子テーブルにはguardが無いため、DBレベルでは公開済みflex policyを直接改変できる余地が残っていた。
  - 対応: (1) `ReplaceWorkScheduleVersionRequest.schedule_type`から`#[serde(default)]`を除去し必須化。full-replace（PUT）セマンティクスでは`days`同様に明示必須とするのが正しいという理由をコード上のコメントで明記した。バックエンドはこの機能追加前からfrontendがwork schedule master APIを未使用（`docs/design-docs/work-schedule-master.md`のPhase2 Statusで frontend未実装と明記済み）のため、この必須化による実クライアントへの破壊的影響は無い。contractのround-tripテスト（`replace_version_request_requires_explicit_schedule_type`）とbackend統合テスト（`replace_without_schedule_type_is_rejected_instead_of_silently_downgrading_flex`）を追加して固定した。テスト実装中に、axumの`Json<T>`extractorはmissing field相当のdeserialize失敗を422（plain textボディ）で返す（malformed JSON構文エラーのみ400）ことが判明したため、`request_json`ヘルパーを非JSONレスポンスボディも扱えるよう拡張し、テストの期待値・アサーションをステータスだけでなくボディ形状（構造化`code`フィールドの有無）でも判別するよう修正した。(2) `rows.rs`の`assemble_version`に`schedule_type`と`settlement_period_row`存在有無の整合性チェックを追加し、不整合時は`WorkScheduleRepositoryError::CorruptData`を返すようにした。(3) migration `048_...`に`work_schedule_settlement_periods`/`work_schedule_core_time_windows`向けの`prevent_published_work_schedule_flex_mutation`トリガー（親versionの`status`を参照してpublished時のINSERT/UPDATE/DELETEを拒否）を追加した。
  - 修正後: `cargo test -p timekeeper-contract`（8 passed、新規1件）、`cargo test -p timekeeper-backend --lib`（373 passed）、`cargo test -p timekeeper-backend --test admin_work_schedules_api`（15 passed、新規1件）、既存3統合テストファイル（27件）回帰なし、`cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `docs-check` 継続green。
