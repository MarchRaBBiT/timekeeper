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
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
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
