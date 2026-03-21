# Exec Plans

複雑タスクのみ、このファイルに実行計画を作成する。  
小規模修正では計画作成は任意。

## 運用ルール
- 複数レイヤー横断（`backend` + `frontend` + `e2e` など）の変更は原則 `ExecPlan` を作成する
- 完了条件は「確認可能な挙動」で定義する（例: APIレスポンス、画面表示、テスト成功）
- 実装中はチェックボックスを更新し、未完了の作業を残す
- テスト成功の節目ごとに `jj` スナップショットを残す

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

## JJ Snapshot Log
- [ ] `jj status`
- [ ] <対象テスト> pass
- [ ] `jj commit -m "chore(test): snapshot after <test_target> pass"`

## Progress Notes
- YYYY-MM-DD: <実施内容>
```

# EP-20260320-timekeeper-backend-go-migration

## Goal
- `timekeeper-backend` を Rust/Axum から Go へ移行し、DB マイグレーション最終状態と API 仕様を 100% 満たす

## Scope
- In: `timekeeper-backend/cmd/timekeeper-backend`, `timekeeper-backend/internal/**`, `timekeeper-backend/docs/go-migration-roadmap.md`, `timekeeper-backend/migrations/**`
- In: backend 互換確認のための Go テスト、PostgreSQL マイグレーション検証、API contract 検証
- Out: frontend の実装変更、Rust frontend/backend の機能追加、DB スキーマの再設計

## Done Criteria (Observable)
- [x] Go の起動基盤、migration runner、route registry、OpenAPI scaffold が存在する
  - [x] `cmd/timekeeper-backend` が起動できる
  - [x] migration runner がある
  - [x] route registry と OpenAPI scaffold がある
- [x] Rust 版 migration 001-042 を Go で順番どおり適用し、最終 schema が一致する
  - [x] 001-042 を順番どおり適用できる
  - [x] final schema と checksum が Rust 版と一致する
- [x] Rust router の全 endpoint が Go にあり、path/method/status/body/header が API catalog と一致する
  - [x] API catalog と route registry が一致する
  - [x] path / method / status / body / header の parity が取れている
- [x] auth / session / MFA / CSRF / rate limit / audit log の挙動が Rust と一致する
  - [x] auth / session の lifecycle が一致する
  - [x] CSRF の cookie-auth mutation 挙動が一致する
  - [x] MFA の挙動が Rust と一致する
  - [x] rate limiting の window / burst が Rust と一致する
  - [x] audit log の write path と masking / admin trail が Rust と一致する
- [x] attendance / requests / consents / subject requests / holidays / admin の主要機能が Go で動作する
  - [x] attendance / attendance corrections
  - [x] requests / consents / subject requests
  - [x] holidays / admin workflows
- [x] `go test ./...` と DB/API parity 検証が green になる
  - [x] `go test ./...`
  - [x] PostgreSQL migration smoke / idempotency / checksum mismatch 検証
  - [x] frontend login/session smoke
  - [x] `go vet ./...` と追加 integration smoke

## Constraints / Non-goals
- Rust 版 migrations は canonical とし、原則として書き換えない
- route path、HTTP method、response envelope、cookie/CSRF semantics は壊さない
- 1 セクションずつ移植し、各節目で検証を通す
- frontend の変更は最小化し、backend 互換を優先する

## Task Breakdown
1. [x] Go module と server/migration/OpenAPI scaffold を追加する
2. [x] DB 接続プール、Redis、request id、logging、CORS、health/readiness を本番相当に仕上げる
3. [x] public auth, refresh, password reset, MFA, session 管理を移植する
4. [x] attendance, attendance corrections, requests, consents, subject requests, holidays を移植する
5. [x] admin 系 endpoint（users, departments, attendance, breaks, audit logs, exports, holidays, sessions）を移植する
6. [x] Rust 既存テストと API catalog に対する parity テストを Go 側へ移す
7. [x] cutover 準備と Rust runtime path の整理を行う

## Validation Plan
- [x] `podman run --rm -v /home/mrabbit/Documents/timekeeper/timekeeper-backend:/work -w /work golang:1.23 go test ./...`
- [x] PostgreSQL コンテナを使った migration smoke test
- [x] Go backend の contract test / parity test
- [x] frontend の login / session smoke が Go backend で通ることを確認
- [x] 必要に応じて `go vet ./...` と追加の integration test を実行
- [x] Go runtime container build via `podman build -t timekeeper-backend-cutover:test ...`

## JJ Snapshot Log
- [x] `git status`
- [x] `go test ./...` pass
- [x] `git commit -m "feat: scaffold go backend migration"`
- [x] `git commit -m "docs: add go migration roadmap"`
- [x] parity test と OpenAPI presence check を追加して `go test ./...` を green にした

## Progress Notes
- 2026-03-20: Go backend の scaffold を nested repo 側に追加し、`go test ./...` を container 上で green にした
- 2026-03-20: backend 移植のチェックリストを `timekeeper-backend/docs/go-migration-roadmap.md` に整理した
- 2026-03-20: そのチェックリストを ExecPlan 形式に合わせて `.agent/PLANS.md` へ反映した
- 2026-03-20: Task Breakdown 2 を実装し、`DB 接続プール / Redis / request id / logging / CORS / health-readiness` を Go 側へ追加した
- 2026-03-20: Task Breakdown 3 を実装し、`auth / refresh / password reset / MFA / sessions` を Go 側へ追加して `go test ./...` を green にした
- 2026-03-20: Task Breakdown 4 を実装し、`attendance / attendance corrections / requests / consents / subject requests / holidays` を Go 側へ追加して `go test ./...` を green にした
- 2026-03-20: Task Breakdown 5 を実装し、`admin users / departments / attendance / breaks / audit logs / export / holidays / sessions` の不足 helper と route parity を埋めて `go test ./...` を green にした
- 2026-03-21: Task Breakdown 6 を実装し、API catalog parity test と OpenAPI presence check を追加して `go test ./...` を green にした
- 2026-03-21: `go vet ./...` を Podman の Go 1.23 container で green 確認し、PostgreSQL 直結の migration smoke も `go run ./cmd/timekeeper-backend --migrate-only` で green 確認した
- 2026-03-21: PostgreSQL-backed verification で migration の idempotency と checksum mismatch detection も確認した
- 2026-03-21: Task Breakdown 7 を実装し、Go Dockerfile / README / ignore rules を追加して Rust runtime path を cutover 可能な状態にした。`podman build` で container image も確認した
- 2026-03-21: PostgreSQL container smoke test を実施し、fresh DB に 42 migrations を適用できることと `schema_migrations` の最終 version が 42 であることを確認した
- 2026-03-21: frontend login/session smoke を Go backend で実施し、wasm frontend は `/dashboard`、timekeeper-frontend TS frontend は `/admin` へ遷移することを確認した
- 2026-03-21: Go backend の live smoke で request-id echo/generation、auth/session/CSRF lifecycle、admin/user/system-admin boundary、409 conflict、download headers、PII masking を確認した。続けて PostgreSQL-backed smoke で `auth_login` / `session_create` / `mfa_reset` の audit log write を確認し、audit-log write path は完了した。rate limiting も Go 側で IP/user の window/burst parity と claims fallback、Redis fallback を unit test で確認した

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
4. [x] `jj` snapshot を作成し、必要なら stack/bookmark を整理

## Validation Plan
- [x] `cargo fmt --all --check`
- [x] `cargo test -p timekeeper-frontend locale -- --nocapture --test-threads=1`
- [x] `cargo test -p timekeeper-frontend confirm_dialog -- --nocapture --test-threads=1`
- [x] `cargo test -p timekeeper-frontend settings -- --nocapture --test-threads=1`
- [x] `cargo test -p timekeeper-backend --test password_api -- --nocapture`
- [x] `bash scripts/harness.sh lint`

## JJ Snapshot Log
- [x] `jj status`
- [x] focused tests pass
- [x] `jj commit -m "fix(i18n): address locale foundation review follow-ups"`
- [x] `jj commit -m "feat(i18n): localize shared and core frontend pages"`
- [x] `jj commit -m "fix(settings): map password change errors by code"`

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

## JJ Snapshot Log
- [x] `jj status`
- [x] `cargo test -p timekeeper-backend --lib encryption` pass
- [ ] `jj commit -m "fix(security): route pii encryption through kms provider abstraction"`

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

## JJ Snapshot Log
- [x] `jj status`
- [x] `cargo test -p timekeeper-backend --lib kms` pass
- [ ] `jj commit -m "feat(security): integrate aws and gcp kms api calls"`

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

## JJ Snapshot Log
- [x] `jj status`
- [x] frontend対象テスト pass
- [ ] `jj commit -m "feat(frontend): surface pii masking state in admin views"`

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

## JJ Snapshot Log
- [x] `jj status`
- [x] backend対象テスト pass
- [ ] `jj commit -m "feat(security): automate pii key rotation workflow"`

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

## JJ Snapshot Log
- [x] `jj status`
- [x] backend関連テスト pass
- [ ] `jj commit -m "fix(security): close issue150 pii plaintext gap"`

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

## JJ Snapshot Log
- [x] `jj status`
- [x] backend/frontend 対象テスト pass
- [ ] `jj commit -m "feat(requests): add attendance correction request workflow"`

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
