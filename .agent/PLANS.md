# Exec Plans

複雑タスクのみ、このファイルに実行計画を作成する。  
小規模修正では計画作成は任意。

## 運用ルール
- 複数レイヤー横断（`backend` + `frontend` + `e2e` など）の変更は原則 `ExecPlan` を作成する
- 完了条件は「確認可能な挙動」で定義する（例: APIレスポンス、画面表示、テスト成功）
- 実装中はチェックボックスを更新し、未完了の作業を残す
- テスト成功の節目ごとに `git status` を確認し、必要な差分だけを conventional commit で記録する

# EP-20260705-leave-entitlement-ledger

**実装完了・検証済み** — 詳細: [`docs/exec-plans/active/EP-20260705-leave-entitlement-ledger.md`](../docs/exec-plans/active/EP-20260705-leave-entitlement-ledger.md)。T-04（G2 実装 前半）: 付与ルールマスタ + append-only `leave_ledger_entries`（migration 052–054）、`crates/domain` の FIFO・時効・年5日義務純ロジック、付与実行 API（dry-run）・残高 read API 2 本・初期投入 `adjust` API。消化引当は T-05

# EP-20260708-leave-request-ledger-consumption

**実装完了・検証済み** — 詳細: [`docs/exec-plans/active/EP-20260708-leave-request-ledger-consumption.md`](../docs/exec-plans/active/EP-20260708-leave-request-ledger-consumption.md)。T-05（G2 実装 後半）: annual 申請の残高不足 reject、承認時 FIFO consume、承認済み annual 取消時 release。pending は引当せず承認時に同一 DB transaction で再検証する。annual 以外は残高非連動を維持

# EP-20260709-leave-attendance-integration

**実装完了・検証済み** — 詳細: [`docs/exec-plans/active/EP-20260709-leave-attendance-integration.md`](../docs/exec-plans/active/EP-20260709-leave-attendance-integration.md)。T-06（G3 前半）: 承認済み休暇を attendance read path / summary / CSV / work-schedule calendar へ read-time join で反映し、休暇日打刻は `leave_conflict` anomaly として検出する。resolved workday への休暇状態書き込みと打刻 reject はしない

# EP-20260709-overtime-request-reconciliation

**実装完了・検証済み** — 詳細: [`docs/exec-plans/active/EP-20260709-overtime-request-reconciliation.md`](../docs/exec-plans/active/EP-20260709-overtime-request-reconciliation.md)。T-07（G3 後半）: `unapproved_overtime` / `overtime_exceeds_request` anomaly を追加し、admin anomaly list / calendar 経由で露出する

# EP-20260709-overtime-monitor-api

**実装完了・検証済み** — 詳細: [`docs/exec-plans/active/EP-20260709-overtime-monitor-api.md`](../docs/exec-plans/active/EP-20260709-overtime-monitor-api.md)。T-08（G4）: `overtime_monitor_settings` と `GET /api/admin/overtime-monitor` / settings API を追加し、月次・年度・直近6ヶ月平均の `ok` / `warning` / `exceeded` を返す

# EP-20260709-punctuality-break-anomalies

**実装完了・検証済み** — 詳細: [`docs/exec-plans/active/EP-20260709-punctuality-break-anomalies.md`](../docs/exec-plans/active/EP-20260709-punctuality-break-anomalies.md)。T-09/T-10（G5/G6）: `late` / `early_leave` / `absent` / `insufficient_break` anomaly を追加

# EP-20260709-monthly-closing-workflow

**実装完了・検証済み** — 詳細: [`docs/exec-plans/active/EP-20260709-monthly-closing-workflow.md`](../docs/exec-plans/active/EP-20260709-monthly-closing-workflow.md)。T-12（G8）: 月次締め design doc、workflow tables、self-confirm / approve / close / reopen API を追加。`closed` は既存 monthly lock と統合

# EP-20260709-request-notification-events

**実装完了・検証済み** — 詳細: [`docs/exec-plans/active/EP-20260709-request-notification-events.md`](../docs/exec-plans/active/EP-20260709-request-notification-events.md)。T-17（G12 後半）: 汎用 notification envelope に申請イベント variant を追加し、申請提出・承認・却下時に Redis queue 有効なら enqueue する

# EP-20260704-notification-service-generalization

**完了済み (92ca21a)** — 詳細: [`docs/exec-plans/active/EP-20260704-notification-service-generalization.md`](../docs/exec-plans/active/EP-20260704-notification-service-generalization.md)。汎用 notification queue（tagged envelope + legacy fallback）、`worker-once` harness stage、RUNBOOK worker 運用節（T-16、tech-debt #7 返済。次は T-17 の申請・打刻イベント配線）

# EP-20260704-attendance-calculation-policy-design

**完了済み (40e10b8)** — 詳細: [`docs/exec-plans/active/EP-20260704-attendance-calculation-policy-design.md`](../docs/exec-plans/active/EP-20260704-attendance-calculation-policy-design.md)。成果物: [`docs/design-docs/attendance-calculation-policy.md`](../docs/design-docs/attendance-calculation-policy.md)（T-01、実装は T-03）

# EP-20260704-leave-entitlement-design

**完了済み (5ec584b)** — 詳細: [`docs/exec-plans/active/EP-20260704-leave-entitlement-design.md`](../docs/exec-plans/active/EP-20260704-leave-entitlement-design.md)。成果物: [`docs/design-docs/leave-entitlement.md`](../docs/design-docs/leave-entitlement.md)（T-02、実装は T-04/T-05）

# EP-20260704-data-retention-policy

**完了済み (8d5101a)** — 詳細: [`docs/exec-plans/active/EP-20260704-data-retention-policy.md`](../docs/exec-plans/active/EP-20260704-data-retention-policy.md)。成果物: [`docs/design-docs/data-retention.md`](../docs/design-docs/data-retention.md)（T-19、purge 実装は方針確定済み・別 EP）

# EP-20260704-attendance-domain-gap-backlog

**バックログ登録済み（棚卸し完了・個別 EP 分解は未着手）** — 詳細: [`docs/exec-plans/active/EP-20260704-attendance-domain-gap-backlog.md`](../docs/exec-plans/active/EP-20260704-attendance-domain-gap-backlog.md)

勤怠管理システムとしての機能不足観点 G1–G15 を優先度付きで棚卸しした親 EP。P1 = 労働時間の法令区分集計（G1）、有給付与・残高台帳（G2）、承認済み申請と実績の連動（G3）、36協定上限監視（G4）。

# EP-20260705-attendance-classification-read-model

**実装完了・検証済み** — 詳細: [`docs/exec-plans/active/EP-20260705-attendance-classification-read-model.md`](../docs/exec-plans/active/EP-20260705-attendance-classification-read-model.md)。T-03（G1）の日次労働時間区分 read-model。API は `GET /api/attendance/me/classification` と `GET /api/admin/users/{user_id}/classification`。

# EP-20260311-frontend-rust-i18n-migration

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260311-frontend-rust-i18n-migration.md`](../docs/exec-plans/active/EP-20260311-frontend-rust-i18n-migration.md)

# EP-20260621-workday-override-and-schedule-read-api

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260621-workday-override-and-schedule-read-api.md`](../docs/exec-plans/active/EP-20260621-workday-override-and-schedule-read-api.md)

# EP-20260314-wildcard-cors-hardening

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260314-wildcard-cors-hardening.md`](../docs/exec-plans/active/EP-20260314-wildcard-cors-hardening.md)

# EP-20260703-work-schedule-phase3-settlement-balance

**計画段階（未着手）** — 詳細: [`docs/exec-plans/active/EP-20260703-work-schedule-phase3-settlement-balance.md`](../docs/exec-plans/active/EP-20260703-work-schedule-phase3-settlement-balance.md)

# EP-20260703-work-schedule-phase3-resolve-workday-flex

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260703-work-schedule-phase3-resolve-workday-flex.md`](../docs/exec-plans/active/EP-20260703-work-schedule-phase3-resolve-workday-flex.md)

# EP-20260702-work-schedule-phase3-flex-core-time

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260702-work-schedule-phase3-flex-core-time.md`](../docs/exec-plans/active/EP-20260702-work-schedule-phase3-flex-core-time.md)

# EP-20260702-work-schedule-phase3-api-wiring

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260702-work-schedule-phase3-api-wiring.md`](../docs/exec-plans/active/EP-20260702-work-schedule-phase3-api-wiring.md)

# EP-20260620-work-schedule-master-design

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260620-work-schedule-master-design.md`](../docs/exec-plans/active/EP-20260620-work-schedule-master-design.md)

# EP-20260620-work-schedule-master-api

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260620-work-schedule-master-api.md`](../docs/exec-plans/active/EP-20260620-work-schedule-master-api.md)

# EP-20260621-resolve-workday

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260621-resolve-workday.md`](../docs/exec-plans/active/EP-20260621-resolve-workday.md)

# EP-20260621-connect-punches-to-workdays

**完了済み** — 詳細: [`docs/exec-plans/active/EP-20260621-connect-punches-to-workdays.md`](../docs/exec-plans/active/EP-20260621-connect-punches-to-workdays.md)

# EP-20260610-rebuild-architecture-harness

**完了済み (b958562)** — 詳細: [`docs/exec-plans/active/EP-20260610-rebuild-architecture-harness.md`](../docs/exec-plans/active/EP-20260610-rebuild-architecture-harness.md)

# EP-20260612-rebuild-target-workspace

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-rebuild-target-workspace.md`](../docs/exec-plans/active/EP-20260612-rebuild-target-workspace.md)

# EP-20260612-attendance-contract-dtos

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-attendance-contract-dtos.md`](../docs/exec-plans/active/EP-20260612-attendance-contract-dtos.md)

# EP-20260612-clock-in-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-clock-in-use-case.md`](../docs/exec-plans/active/EP-20260612-clock-in-use-case.md)

# EP-20260612-clock-out-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-clock-out-use-case.md`](../docs/exec-plans/active/EP-20260612-clock-out-use-case.md)

# EP-20260612-start-break-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-start-break-use-case.md`](../docs/exec-plans/active/EP-20260612-start-break-use-case.md)

# EP-20260612-end-break-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-end-break-use-case.md`](../docs/exec-plans/active/EP-20260612-end-break-use-case.md)

# EP-20260612-attendance-infra-postgres-repository

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-attendance-infra-postgres-repository.md`](../docs/exec-plans/active/EP-20260612-attendance-infra-postgres-repository.md)

# EP-20260612-attendance-status-read-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-attendance-status-read-use-case.md`](../docs/exec-plans/active/EP-20260612-attendance-status-read-use-case.md)

# EP-20260612-breaks-by-attendance-read-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-breaks-by-attendance-read-use-case.md`](../docs/exec-plans/active/EP-20260612-breaks-by-attendance-read-use-case.md)

# EP-20260612-admin-force-end-break-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-admin-force-end-break-use-case.md`](../docs/exec-plans/active/EP-20260612-admin-force-end-break-use-case.md)

# EP-20260612-admin-active-breaks-read-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-admin-active-breaks-read-use-case.md`](../docs/exec-plans/active/EP-20260612-admin-active-breaks-read-use-case.md)

# EP-20260612-admin-attendance-page-read-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-admin-attendance-page-read-use-case.md`](../docs/exec-plans/active/EP-20260612-admin-attendance-page-read-use-case.md)

# EP-20260612-admin-attendance-upsert-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260612-admin-attendance-upsert-use-case.md`](../docs/exec-plans/active/EP-20260612-admin-attendance-upsert-use-case.md)

# EP-20260613-user-attendance-range-read-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-user-attendance-range-read-use-case.md`](../docs/exec-plans/active/EP-20260613-user-attendance-range-read-use-case.md)

# EP-20260613-user-attendance-summary-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-user-attendance-summary-use-case.md`](../docs/exec-plans/active/EP-20260613-user-attendance-summary-use-case.md)

# EP-20260613-user-attendance-export-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-user-attendance-export-use-case.md`](../docs/exec-plans/active/EP-20260613-user-attendance-export-use-case.md)

# EP-20260613-admin-attendance-export-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-admin-attendance-export-use-case.md`](../docs/exec-plans/active/EP-20260613-admin-attendance-export-use-case.md)

# EP-20260613-create-attendance-correction-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-create-attendance-correction-use-case.md`](../docs/exec-plans/active/EP-20260613-create-attendance-correction-use-case.md)

# EP-20260613-manage-attendance-correction-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-manage-attendance-correction-use-case.md`](../docs/exec-plans/active/EP-20260613-manage-attendance-correction-use-case.md)

# EP-20260613-admin-attendance-correction-decision-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-admin-attendance-correction-decision-use-case.md`](../docs/exec-plans/active/EP-20260613-admin-attendance-correction-decision-use-case.md)

# EP-20260613-admin-attendance-correction-read-use-case

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-admin-attendance-correction-read-use-case.md`](../docs/exec-plans/active/EP-20260613-admin-attendance-correction-read-use-case.md)

# EP-20260613-attendance-correction-infra-postgres-repository

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-attendance-correction-infra-postgres-repository.md`](../docs/exec-plans/active/EP-20260613-attendance-correction-infra-postgres-repository.md)

# EP-20260613-attendance-correction-contract-dtos

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-attendance-correction-contract-dtos.md`](../docs/exec-plans/active/EP-20260613-attendance-correction-contract-dtos.md)

# EP-20260613-admin-attendance-contract-dtos

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-admin-attendance-contract-dtos.md`](../docs/exec-plans/active/EP-20260613-admin-attendance-contract-dtos.md)

# EP-20260613-leave-overtime-request-contract-dtos

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-leave-overtime-request-contract-dtos.md`](../docs/exec-plans/active/EP-20260613-leave-overtime-request-contract-dtos.md)

# EP-20260613-subject-request-contract-dtos

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-subject-request-contract-dtos.md`](../docs/exec-plans/active/EP-20260613-subject-request-contract-dtos.md)

# EP-20260613-department-contract-dtos

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-department-contract-dtos.md`](../docs/exec-plans/active/EP-20260613-department-contract-dtos.md)

# EP-20260613-holiday-contract-dtos

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-holiday-contract-dtos.md`](../docs/exec-plans/active/EP-20260613-holiday-contract-dtos.md)

# EP-20260613-auth-password-reset-contract-dtos

**完了済み (c9b64b3)** — 詳細: [`docs/exec-plans/active/EP-20260613-auth-password-reset-contract-dtos.md`](../docs/exec-plans/active/EP-20260613-auth-password-reset-contract-dtos.md)

## テンプレート

```md
# EP-YYYYMMDD-<short-slug>

## Goal
- <このタスクで達成すること>

## Scope
- In: <対象>
- Out: <対象外>

## Done Criteria (Observable)
- [ ] <確認可能な完了条件1>
- [ ] <確認可能な完了条件2>

## Constraints / Non-goals
- <制約や今回やらないこと>

## Task Breakdown
1. [ ] <実装タスク1>
2. [ ] <実装タスク2>
3. [ ] <実装タスク3>

## Validation Plan
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `bash scripts/harness.sh lint`
- [ ] `./scripts/test_backend_integrated.sh` または `cargo test --test <target>`
- [ ] `pwsh -File .\scripts\test_backend.ps1`（必要時）
- [ ] `cd frontend; wasm-pack test --headless --firefox`（必要時）
- [ ] `cd e2e; node run.mjs`（必要時）

## Git Snapshot Log
- [ ] `git status --short`
- [ ] <対象テスト> pass
- [ ] `git commit -m "<type>: <description>"`

## Progress Notes
- YYYY-MM-DD: <実施内容>
```

# EP-20260311-pr430-431-review-followup

## Goal
- PR #430 / #431 に残っているレビューコメントを、stack 構造を壊さずに反映する

## Scope
- In: `frontend/src/state/locale.rs`, `frontend/src/components/confirm_dialog.rs`, `frontend/locales/*.yml`, `frontend/src/pages/settings/panel.rs`, 必要最小限の backend error response
- Out: `rust-i18n` preview 依存の見直し、locale リロード方式そのものの再設計

## Done Criteria (Observable)
- [x] locale context 初期化直後から期待 locale で翻訳されることを test で確認できる
- [x] dialog close label と `common.labels.code` 翻訳の指摘が修正されている
- [x] settings のパスワード変更エラーが backend 文言ではなく error code でローカライズされる
- [x] 関連する frontend/backend の focused test が green

## Constraints / Non-goals
- stacked PR のため、PR #430 相当の修正と PR #431 相当の修正を意識して差分を分ける
- 不要な広範囲 i18n 置換や UI リファクタは行わない

## Task Breakdown
1. [x] PR #430 レイヤーの未対応コメントを test 追加込みで修正
2. [x] PR #431 レイヤーの password change error mapping を error code ベースへ移行
3. [x] focused validation 実施
4. [x] commit snapshot を作成し、必要なら stack/bookmark を整理

## Validation Plan
- [x] `cargo fmt --all --check`
- [x] `cargo test -p timekeeper-frontend locale -- --nocapture --test-threads=1`
- [x] `cargo test -p timekeeper-frontend confirm_dialog -- --nocapture --test-threads=1`
- [x] `cargo test -p timekeeper-frontend settings -- --nocapture --test-threads=1`
- [x] `cargo test -p timekeeper-backend --test password_api -- --nocapture`
- [x] `bash scripts/harness.sh lint`

## Git Snapshot Log
- [x] `git status --short`
- [x] focused tests pass
- [x] commit recorded: `fix(i18n): address locale foundation review follow-ups`
- [x] commit recorded: `feat(i18n): localize shared and core frontend pages`
- [x] commit recorded: `fix(settings): map password change errors by code`

## Progress Notes
- 2026-03-11: PR #430 / #431 review threads を確認し、残差分を locale 初期化・dialog a11y・ja 翻訳・password error code に絞り込んだ。
- 2026-03-11: `cargo test -p timekeeper-frontend locale -- --nocapture --test-threads=1`、`confirm_dialog`、`settings`、`cargo test -p timekeeper-backend --test password_api -- --nocapture`、`bash scripts/harness.sh lint` を green 確認。
- 2026-03-11: `push-qqmxntqlymrw` を `20b8cbb8`、`push-wvlvrxqtwqlx` を `39fdca67` へ更新し、PR 430/431 の stack を clean な chain に再構成。

# EP-20260212-kms-provider-abstraction

## Goal
- PII暗号化を `KmsProvider` 抽象経由に切り替え、疑似KMS実装をその1実装として提供する

## Scope
- In: `backend/src/utils/encryption.rs`, 新規 `backend/src/utils/kms.rs`, 関連ユーティリティテスト
- Out: AWS/GCP実KMS API接続、DB migration追加、frontend変更

## Done Criteria (Observable)
- [x] `encrypt_pii` / `decrypt_pii` が `KmsProvider` を経由して動作する
- [x] 既存 `kms:v1:<nonce>:<cipher>` を復号できる後方互換が維持される
- [x] 既存関連テストが成功する

## Constraints / Non-goals
- 疑似KMSは現行の鍵導出ロジックを維持する
- 実KMSプロバイダはこの変更では未実装（インターフェースのみ）

## Task Breakdown
1. [x] `KmsProvider` 抽象と envelope パーサを新規追加
2. [x] 疑似KMS暗号化ロジックを `KmsProvider` 実装へ移設し `encryption.rs` を差し替え
3. [x] 互換ケース（旧 envelope 形式）を含む単体テストを追加/更新
4. [x] fmt と対象テストで回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib encryption`

## Git Snapshot Log
- [x] `git status --short`
- [x] `cargo test -p timekeeper-backend --lib encryption` pass
- [x] commit recorded: `fix(security): route pii encryption through kms provider abstraction`

## Progress Notes
- 2026-02-12: 計画作成
- 2026-02-12: `utils/kms.rs` を追加し、疑似KMSを `KmsProvider` 実装へ移設。`encryption` / `mfa` 関連ユニットテスト成功を確認。

# EP-20260212-kms-api-integration

## Goal
- `AwsKmsProvider` と `GcpKmsProvider` に実際のKMS API呼び出しを実装する

## Scope
- In: `backend/src/utils/kms.rs`, `backend/Cargo.toml`
- Out: 本番資格情報配布、インフラ構築、E2Eクラウド疎通試験

## Done Criteria (Observable)
- [x] AWS provider が KMS Encrypt/Decrypt API を呼び出すコードになっている
- [x] GCP provider が Cloud KMS Encrypt/Decrypt API を呼び出すコードになっている
- [x] 既存暗号化ユニットテストが成功する

## Constraints / Non-goals
- 実環境疎通はこのPR内では行わない
- 既存 envelope 形式互換は維持する

## Task Breakdown
1. [x] AWS SDK 呼び出しの実装（encrypt/decrypt）
2. [x] GCP Cloud KMS REST 呼び出しの実装（encrypt/decrypt）
3. [x] nonce 埋め込み方式で既存 envelope 互換を維持
4. [x] fmt + ユニットテストで回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib kms`
- [x] `cargo test -p timekeeper-backend --lib encryption`

## Git Snapshot Log
- [x] `git status --short`
- [x] `cargo test -p timekeeper-backend --lib kms` pass
- [x] commit recorded: `feat(security): integrate aws and gcp kms api calls`

## Progress Notes
- 2026-02-12: 実KMS API呼び出しコードの実装開始
- 2026-02-12: AWS SDK (`aws-sdk-kms`) と GCP Cloud KMS REST 呼び出しを `KmsProvider` 実装に反映。暗号化関連ユニットテスト成功。

# EP-20260212-issue150-frontend-pii-mask-ui

## Goal
- issue #150 の未実装差分として、PIIマスキング状態をフロントエンドで明示表示する

## Scope
- In: `frontend/src/api/*`, `frontend/src/pages/admin_users/*`, `frontend/src/pages/admin_export/*`, `frontend/src/pages/admin_audit_logs/*`
- Out: 新規バックエンドAPI追加、DBマイグレーション変更

## Done Criteria (Observable)
- [x] `X-PII-Masked` ヘッダをAPIクライアントで受け取れる
- [x] 管理画面でマスキング適用中のバナー/注記が表示される
- [x] 既存関連テストが成功する

## Constraints / Non-goals
- 既存 API レスポンスの JSON 互換性は壊さない
- 権限判定ロジックはバックエンド仕様に追従し、フロントは表示のみ追加

## Task Breakdown
1. [x] API型に `pii_masked` 付きレスポンス型を追加
2. [x] Users/Export/Audit API にヘッダ読み取りメソッドを追加
3. [x] 各 ViewModel に `pii_masked` 状態を追加
4. [x] 各パネルに注意表示を追加
5. [x] fmt + frontend対象テストで回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-frontend --lib admin_users`
- [x] `cargo test -p timekeeper-frontend --lib admin_export`
- [x] `cargo test -p timekeeper-frontend --lib admin_audit_logs`

## Git Snapshot Log
- [x] `git status --short`
- [x] frontend対象テスト pass
- [x] commit recorded: `feat(frontend): surface pii masking state in admin views`

## Progress Notes
- 2026-02-12: 実装開始
- 2026-02-12: APIクライアントで `X-PII-Masked` を取り込み、admin users/export/audit logs でマスキング表示バナーを実装。frontend関連テスト成功。

# EP-20260212-key-rotation-automation

## Goal
- PII暗号鍵のローテーション運用をコード化し、再暗号化を自動実行できるようにする

## Scope
- In: `backend/src/utils/kms.rs`, `backend/src/utils/encryption.rs`, `backend/src/bin/pii_backfill.rs`, 新規 `backend/src/bin/pii_rotate_keys.rs`
- Out: インフラ側の鍵作成自動化（Terraform等）

## Done Criteria (Observable)
- [x] 暗号化 envelope に鍵バージョンが格納される
- [x] 復号時に envelope の鍵バージョンから適切な鍵設定を解決できる
- [x] ローテーションCLIで users/archived_users の PII を再暗号化できる
- [x] 対象ユニットテストが成功する

## Constraints / Non-goals
- 既存データ（旧 envelope / 平文）との後方互換を壊さない
- ローテーション対象は PII 列の再暗号化に限定する

## Task Breakdown
1. [x] KMS envelope を `provider + key_version` 形式へ拡張
2. [x] versioned key id/name 解決ロジックを provider に実装
3. [x] `encrypt_pii` / `decrypt_pii` を鍵バージョン対応
4. [x] `pii_rotate_keys` CLI を追加
5. [x] fmt + backend関連テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib encryption`
- [x] `cargo test -p timekeeper-backend --lib kms`
- [x] `cargo test -p timekeeper-backend --lib mfa`

## Git Snapshot Log
- [x] `git status --short`
- [x] backend対象テスト pass
- [x] commit recorded: `feat(security): automate pii key rotation workflow`

## Progress Notes
- 2026-02-12: 実装開始
- 2026-02-12: envelopeへ鍵バージョンを導入し、versioned key設定解決・`pii_rotate_keys` CLIを追加。関連ユニットテストとbinコンパイル確認済み。

# EP-20260212-issue150-gap-closure

## Goal
- issue #150 の残差分（平文フォールバック除去・平文列廃止・関連テスト整備）を完了する

## Scope
- In: `backend/src/repositories/*`, `backend/src/middleware/auth.rs`, `backend/src/handlers/*`, `backend/migrations/*`, `backend/tests/*`
- Out: インフラ資格情報、本番データの実移行作業

## Done Criteria (Observable)
- [x] DBクエリから平文PIIカラム参照/fallbackが除去されている
- [x] `users`/`archived_users` の平文PIIカラム削除migrationが追加されている
- [x] 影響範囲の統合テスト/ユニットテストが成功する

## Constraints / Non-goals
- 既存API仕様は維持（JSON項目名は互換維持）
- SQLx migrationは新規追加のみで対応

## Task Breakdown
1. [x] repository/handler/middleware の平文fallbackクエリを削除
2. [x] 平文PIIカラム削除migrationを追加
3. [x] テストデータ投入を `*_enc` 前提に更新
4. [x] 旧データ由来の `*_enc IS NULL` を吸収するmigrationを追加
5. [x] 影響テストを再実行して回帰なしを確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --test admin_users_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test password_reset_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test transaction_repository -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test user_update_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test user_repository -- --nocapture`

## Git Snapshot Log
- [x] `git status --short`
- [x] backend関連テスト pass
- [x] commit recorded: `fix(security): close issue150 pii plaintext gap`

## Progress Notes
- 2026-02-12: `*_enc` への一本化と平文列削除migration（032）を追加。
- 2026-02-12: 既存seed由来の `NULL` を補正するmigration（033）を追加し、統合テスト群のデータ投入を暗号化前提に更新。

# EP-20260213-attendance-correction-request

## Goal
- フロントエンドに勤怠修正依頼機能を追加し、承認後に補正値を正として集計・表示・CSVへ反映する

## Scope
- In: `backend`（migration/model/repository/handler/router）、`frontend`（API/types/requests画面）、関連テスト
- Out: メール通知、既存過去データ移行、上長ロール新設

## Done Criteria (Observable)
- [x] 従業員が1日単位の勤怠修正依頼（出勤/退勤/休憩明細+理由）を作成/更新/取消できる
- [x] 管理者が勤怠修正依頼を一覧/詳細確認し承認/却下できる
- [x] 承認時に原本差分を検知して衝突エラーを返せる
- [x] 承認済み補正値が勤怠履歴・月次サマリ・CSVに反映される
- [x] requests画面から勤怠修正依頼を操作できる

## Constraints / Non-goals
- 原本打刻（attendance / break_records）は上書きしない
- 値クリア（未設定化）は許可しない
- SQLx migrationは新規追加のみ

## Task Breakdown
1. [x] DBマイグレーション追加（修正依頼・補正値テーブル）
2. [x] backend モデル/リポジトリ/ハンドラー/ルーティング実装
3. [x] 勤怠集計・履歴・CSVへ補正優先ロジック導入
4. [x] frontend API/types/requests画面に勤怠修正依頼UIを追加
5. [x] backend/frontend テスト追加と回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --test requests_api`
- [x] `cargo test -p timekeeper-frontend --lib requests`

## Git Snapshot Log
- [x] `git status --short`
- [x] backend/frontend 対象テスト pass
- [x] commit recorded: `feat(requests): add attendance correction request workflow`

## Progress Notes
- 2026-02-13: 計画作成
- 2026-02-13: migration/handler/repository/frontend requests UI まで実装し、`cargo fmt --all`、`cargo test -p timekeeper-backend --lib`、`cargo test -p timekeeper-backend --test requests_api -- --nocapture`、`cargo test -p timekeeper-frontend requests:: -- --nocapture` を通過。

# EP-20260316-frontend-invite-department

## Goal
- 新規ユーザー招待フォームに部署選択ドロップダウンを追加し、backend の `department_id` フィールドと整合させる

## Scope
- In: `frontend/src/api/types.rs`, `frontend/src/pages/admin/components/department_select.rs` (新規), `frontend/src/pages/admin/components/mod.rs`, `frontend/src/pages/admin_users/` (repository/utils/view_model/components/panel), `frontend/locales/ja.yml`, `frontend/locales/en.yml`
- Out: backend 変更, ユーザー一覧での部署表示, ユーザー編集フォームの追加

## Done Criteria (Observable)
- [x] 招待フォームに部署ドロップダウンが表示される
- [x] 部署を選択してユーザー招待すると `POST /admin/users` に `department_id` が送信される
- [x] 部署未選択の場合は `department_id` がリクエストに含まれない（または `null`）
- [x] `cargo clippy --all-targets -- -D warnings` が通る
- [x] 関連 frontend テストが green

## Constraints / Non-goals
- ユーザー一覧・詳細画面への部署表示は今回のスコープ外
- backend に変更なし
- 部署階層のツリー表示は今回対象外（フラットリスト）

## Task Breakdown
1. [x] `api/types.rs` に `department_id` 追加 + struct literal 全箇所更新
2. [x] `department_select.rs` 新規作成 + `mod.rs` 登録
3. [x] `repository.rs` に `fetch_departments()` 追加
4. [x] `utils.rs` の `InviteFormState` 更新
5. [x] `view_model.rs` に `departments_resource` 追加
6. [x] `invite_form.rs` に `AdminDepartmentSelect` 組み込み
7. [x] `panel.rs` に prop 追加
8. [x] locale ファイル更新
9. [x] lint + frontend テスト green 確認

## Validation Plan
- [x] `bash scripts/harness.sh fmt-check`
- [x] `cargo test -p timekeeper-frontend --lib admin_users`
- [x] `cargo test -p timekeeper-frontend --lib department_select`
- [x] `bash scripts/harness.sh lint`

## Progress Notes
- 2026-03-16: 計画作成・実装・全テスト green 確認
