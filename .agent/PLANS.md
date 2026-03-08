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

# EP-20260309-requests-application-create-request

## Goal
- user request handler の create 系ロジックを requests application 層へ移し、HTTP 層を adapter に近づける

## Scope
- In: `backend/src/handlers/requests.rs`, `backend/src/requests/application/user_requests.rs`, `.agent/PLANS.md`
- Out: request API path/method/payload 変更、DB schema 変更、admin request flow の再設計

## Done Criteria (Observable)
- [x] leave/overtime request 作成時の repository 呼び出しが `requests::application::user_requests` に移っている
- [x] handler は payload 検証と domain model 構築、HTTP response 変換に集中している
- [x] requests 関連ユニットテストと app router build テストが成功する

## Constraints / Non-goals
- 既存 API の JSON 形状は維持する
- `BREAKING_CHANGES.md` は API 互換が崩れた場合のみ更新する

## Task Breakdown
1. [x] create leave/overtime request の repository bridge を application 層へ抽出
2. [x] handler から直接 `RequestRepository` を触る責務を除去
3. [x] fmt + clippy + 対象テストで回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib planned_hours_validation`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`
- [ ] `cargo clippy --all-targets -- -D warnings`

## JJ Snapshot Log
- [x] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(requests): move create request into application service"`

## Progress Notes
- 2026-03-09: `create_leave_request` / `create_overtime_request` の永続化橋渡しを application 層へ移し、handler 側は validation と model 構築に限定。
- 2026-03-09: `cargo clippy --all-targets -- -D warnings` は `utoipa-swagger-ui` の build artifact path 不整合で失敗。今回の差分由来の lint 警告は未検出。

# EP-20260309-admin-request-read-application

## Goal
- admin request の read 系ユースケースを requests application 層へ移し、admin handler を権限確認と HTTP adapter に寄せる

## Scope
- In: `backend/src/handlers/admin/requests.rs`, `backend/src/requests/application/admin_requests.rs`, `backend/src/requests/application/mod.rs`, `.agent/PLANS.md`
- Out: approve/reject 更新系の再設計、API path/method/payload 変更、DB schema 変更

## Done Criteria (Observable)
- [x] admin request 一覧取得の filter 解釈と response 組み立てが application 層に移っている
- [x] admin request 詳細取得の kind/data 判定が application 層に移っている
- [x] 既存 admin request API の JSON 形状を維持したまま関連ユニットテストが成功する

## Constraints / Non-goals
- OpenAPI で参照している request/query 型は handler 側に残す
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests::application::admin_requests` を追加し、一覧/詳細の query 解釈と response 変換を移設
2. [x] `handlers::admin::requests` から read 系 repository 呼び出しを除去
3. [x] fmt + 対象テスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::admin_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(requests): extract admin request read application service"`

## Progress Notes
- 2026-03-09: admin request の一覧/詳細取得と filter/page 解釈を `requests::application::admin_requests` へ移動開始。handler 側は auth と query 受け渡しに集中させる。

# EP-20260309-admin-request-decision-application

## Goal
- admin request の approve/reject と自己承認防止を requests application 層へ移し、admin handler を更新系でも adapter に寄せる

## Scope
- In: `backend/src/handlers/admin/requests.rs`, `backend/src/requests/application/admin_requests.rs`, `.agent/PLANS.md`
- Out: request API path/method/payload 変更、DB schema 変更、監査ミドルウェア仕様変更

## Done Criteria (Observable)
- [x] approve/reject の status update 呼び出しが application 層に移っている
- [x] 自己承認防止と comment validation の業務ルールが application/handler 境界で明確になっている
- [x] 既存 admin request 更新系 API の JSON 形状を維持したまま関連テストが成功する

## Constraints / Non-goals
- admin 権限チェック自体は handler に残す
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests::application::admin_requests` に decision use case と自己承認防止ロジックを追加
2. [x] `handlers::admin::requests` から直接 repository 更新を除去
3. [x] fmt + 関連ユニットテスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::admin_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::admin::requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(requests): move admin request decisions into application service"`

## Progress Notes
- 2026-03-09: approve/reject の repository 更新と自己承認防止を application 層へ移し、handler は auth・body 受け取り・HTTP 変換に限定。

# EP-20260213-attendance-correction-request

## Goal
- フロントエンドに勤怠修正依頼機能を追加し、承認後に補正値を正として集計・表示・CSVへ反映する

## Scope
- In: `backend`（migration/model/repository/handler/router）、`frontend`（API/types/requests画面）、関連テスト
- Out: メール通知、既存過去データ移行、上長ロール新設

# EP-20260308-backend-architecture-foundation

## Goal
- バックエンド全面再構築に向けた最初の土台として、起動/bootstrap とルーティング構成を `platform` 層へ分離し、`main.rs` を薄くする

## Scope
- In: `backend/src/main.rs`, 新規 `backend/src/platform/*`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: handler の業務ロジック再設計、DB schema 再設計、API 互換性変更

## Done Criteria (Observable)
- [x] `main.rs` がアプリ組み立ての委譲のみを行う
- [x] 既存の public/user/admin/system-admin ルーティング組み立てが `platform` モジュールに移動している
- [x] CORS/cleanup/bootstrap の既存挙動をユニットテストで維持確認できる

## Constraints / Non-goals
- 既存 handler / middleware / state の外部挙動は変えない
- 今回は新規アーキテクチャの受け皿作成までに留め、業務モジュール分解は別タスクとする

## Task Breakdown
1. [x] `platform` モジュール追加と `main.rs` の薄型化
2. [x] ルーティング・共通 layer・cleanup 起動処理の移設
3. [x] 既存 `main.rs` テストの移設と回帰確認
4. [x] fmt + 対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib platform::`
- [x] `cargo test -p timekeeper-backend --bin timekeeper-backend`

## JJ Snapshot Log
- [x] `jj status`
- [x] `cargo test -p timekeeper-backend --lib platform::` pass
- [x] `cargo test -p timekeeper-backend --bin timekeeper-backend` pass
- [ ] `jj commit -m "refactor(backend): move app bootstrap into platform module"`

## Progress Notes
- 2026-03-08: 専用ブランチ `topic/backend-rebuild-architecture` を作成。
- 2026-03-08: `platform::{app,runtime}` を追加し、`main.rs` の起動・ルーティング・cleanup 初期化を移設。platform ユニットテストと bin テストビルド成功。

# EP-20260308-identity-http-boundary

## Goal
- 認証・セッション関連の HTTP 入口を `identity` モジュールへ移し、`platform` は業務モジュール合成に集中させる

## Scope
- In: 新規 `backend/src/identity/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: auth/sessions handler 本体のリライト、API contract 変更、DB 変更

## Done Criteria (Observable)
- [x] `/api/auth/*` と関連 public config route のルーティング定義が `identity/interface/http.rs` に集約されている
- [x] `platform::app::build_app` が identity ルータを合成している
- [x] identity と platform の対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `identity` モジュール追加
2. [x] auth/sessions/config の route 定義を `identity/interface/http.rs` へ移設
3. [x] `platform::app` から identity ルータを合成するよう変更
4. [x] identity / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib user_identity_routes_require_auth`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`
- [x] `cargo test -p timekeeper-backend --lib test_domain_route_groups_require_auth`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(identity): extract auth http routes into identity module"`

## Progress Notes
- 2026-03-08: `identity/interface/http.rs` を追加し、認証・セッション系ルートを `platform` から切り離した。既存 API 変更はなし。

# EP-20260308-attendance-http-boundary

## Goal
- 勤怠・勤怠修正申請の HTTP 入口を `attendance` モジュールへ移し、`platform` はモジュール合成責務へ寄せる

## Scope
- In: 新規 `backend/src/attendance/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: attendance handler 本体の再実装、holiday ルート分離、API contract 変更

## Done Criteria (Observable)
- [x] `/api/attendance*` と `/api/admin/attendance*` の route 定義が `attendance/interface/http.rs` に集約されている
- [x] admin と system-admin の勤怠系権限制御が従来どおり分離されている
- [x] attendance / platform の対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `attendance` モジュール追加
2. [x] attendance / attendance-correction admin route を `attendance/interface/http.rs` へ移設
3. [x] `platform::app` から attendance ルータを合成するよう変更
4. [x] attendance / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib attendance::interface::http::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(attendance): extract attendance http routes into module"`

## Progress Notes
- 2026-03-08: `attendance/interface/http.rs` を追加し、user/admin/system-admin の勤怠系 route 定義を `platform` から切り離した。既存 API 変更はなし。

# EP-20260308-requests-http-boundary

## Goal
- 申請・同意・本人開示請求の HTTP 入口を `requests` モジュールへ移し、`platform` はモジュール合成責務へ寄せる

## Scope
- In: 新規 `backend/src/requests/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: request handler 本体の再実装、audit/holiday ルート分離、API contract 変更

## Done Criteria (Observable)
- [x] `/api/requests*`, `/api/consents*`, `/api/subject-requests*`, `/api/admin/requests*`, `/api/admin/subject-requests*` の route 定義が `requests/interface/http.rs` に集約されている
- [x] user/admin の申請系権限制御が従来どおり維持されている
- [x] requests / platform の対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests` モジュール追加
2. [x] request/consent/subject-request 系 route を `requests/interface/http.rs` へ移設
3. [x] `platform::app` から requests ルータを合成するよう変更
4. [x] requests / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::interface::http::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`
- [x] `cargo test -p timekeeper-backend --lib test_domain_route_groups_require_auth`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(requests): extract request http routes into module"`

## Progress Notes
- 2026-03-08: `requests/interface/http.rs` を追加し、申請・同意・本人開示請求系 route 定義を `platform` から切り離した。既存 API 変更はなし。

# EP-20260309-holiday-http-boundary

## Goal
- 祝日・週次休日・個別祝日例外の HTTP 入口を `holiday` モジュールへ移し、`platform` はモジュール合成責務へ寄せる

## Scope
- In: 新規 `backend/src/holiday/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: holiday handler 本体の再実装、audit/admin user ルート分離、API contract 変更

## Done Criteria (Observable)
- [x] `/api/holidays*`, `/api/admin/holidays*`, `/api/admin/users/{user_id}/holiday-exceptions*` の route 定義が `holiday/interface/http.rs` に集約されている
- [x] user/admin の holiday 系権限制御が従来どおり維持されている
- [x] holiday / platform の対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `holiday` モジュール追加
2. [x] holiday / holiday-exception 系 route を `holiday/interface/http.rs` へ移設
3. [x] `platform::app` から holiday ルータを合成するよう変更
4. [x] holiday / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib holiday::interface::http::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(holiday): extract holiday http routes into module"`

## Progress Notes
- 2026-03-09: `holiday/interface/http.rs` を追加し、祝日・週次休日・祝日例外の route 定義を `platform` から切り離した。既存 API 変更はなし。

# EP-20260309-admin-http-boundary

## Goal
- 管理画面向けユーザー管理・監査ログ・CSV export の HTTP 入口を `admin` モジュールへ移し、`platform` の route 定義を空に近づける

## Scope
- In: 新規 `backend/src/admin/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: admin handler 本体の再実装、application/use-case 層抽出、API contract 変更

## Done Criteria (Observable)
- [x] `/api/admin/users*`, `/api/admin/archived-users*`, `/api/admin/audit-logs*`, `/api/admin/export` の route 定義が `admin/interface/http.rs` に集約されている
- [x] admin / system-admin の権限制御が従来どおり維持されている
- [x] `platform` の route group 関数が空ルータになり、対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `admin` モジュール追加
2. [x] admin/audit/export 系 route を `admin/interface/http.rs` へ移設
3. [x] `platform::app` から admin ルータを合成するよう変更
4. [x] admin / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib admin::interface::http::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`
- [x] `cargo test -p timekeeper-backend --lib platform_route_groups_are_empty_after_extraction`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(admin): extract admin http routes into module"`

## Progress Notes
- 2026-03-09: `admin/interface/http.rs` を追加し、管理者向けユーザー管理・監査ログ・export route 定義を `platform` から切り離した。既存 API 変更はなし。

# EP-20260309-identity-application-sessions

## Goal
- `identity` モジュールに最初の application/use-case を導入し、session 一覧取得の整形責務を handler から外す

## Scope
- In: 新規 `backend/src/identity/application/*`, `backend/src/handlers/sessions.rs`, 関連ユニットテスト
- Out: session revoke のユースケース化、auth handler 全体の再設計、API contract 変更

## Done Criteria (Observable)
- [x] `list sessions` の repository 呼び出しと DTO 変換が `identity/application/sessions.rs` に移動している
- [x] `handlers/sessions.rs` は application 層の結果を HTTP レスポンスへ変換するだけになっている
- [x] 新旧対象テストが成功する

## Constraints / Non-goals
- 既存 API response 形状は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `identity/application/sessions.rs` を追加
2. [x] `list_sessions` の session view 生成を application 層へ移設
3. [x] `handlers/sessions.rs` を application 呼び出しへ差し替え
4. [x] application / handler / app build テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib identity::application::sessions::tests::`
- [x] `cargo test -p timekeeper-backend --lib extract_ip_`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(identity): add sessions application service"`

## Progress Notes
- 2026-03-09: `identity/application/sessions.rs` を追加し、session 一覧取得の整形責務を handler から移設した。既存 API 変更はなし。

# EP-20260309-identity-application-revoke-session

## Goal
- `identity` の session revoke 業務ロジックを application/use-case 化し、handler から削除・整合性チェック責務を外す

## Scope
- In: `backend/src/identity/application/sessions.rs`, `backend/src/handlers/sessions.rs`, 関連ユニットテスト
- Out: 監査ログ組み立ての完全移設、admin session revoke の共通化、API contract 変更

## Done Criteria (Observable)
- [x] session revoke のバリデーション・所有者確認・トークン削除が `identity/application/sessions.rs` に移動している
- [x] `handlers/sessions.rs` は application 層の結果を使って監査イベントと HTTP レスポンスを返すだけになっている
- [x] 新旧対象テストが成功する

## Constraints / Non-goals
- 既存 API response 形状は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] revoke 用の application 関数と戻り値型を追加
2. [x] token/session 削除処理を handler から application 層へ移設
3. [x] `handlers/sessions.rs` を application 呼び出しへ差し替え
4. [x] application / handler / app build テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib identity::application::sessions::tests::`
- [x] `cargo test -p timekeeper-backend --lib extract_ip_`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(identity): move session revoke into application service"`

## Progress Notes
- 2026-03-09: session revoke の整合性チェックと削除処理を `identity/application/sessions.rs` に移し、handler には監査と HTTP 応答だけを残した。既存 API 変更はなし。

# EP-20260309-requests-application-my-requests

## Goal
- `requests` の `get_my_requests` を application/use-case 化し、handler から repository 集約とレスポンス組み立て責務を外す

## Scope
- In: 新規 `backend/src/requests/application/*`, `backend/src/handlers/requests.rs`, 関連ユニットテスト
- Out: create/update/cancel request のユースケース化、admin request handler の再設計、API contract 変更

## Done Criteria (Observable)
- [x] `get_my_requests` の leave/overtime/correction 集約と view 組み立てが `requests/application/user_requests.rs` に移動している
- [x] `handlers/requests.rs` は application 層の結果を JSON 応答へ変換するだけになっている
- [x] 新旧対象テストが成功する

## Constraints / Non-goals
- 既存 API response 形状は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `requests/application/user_requests.rs` を追加
2. [x] `get_my_requests` の集約・整形を application 層へ移設
3. [x] `handlers/requests.rs` を application 呼び出しへ差し替え
4. [x] application / app build テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib get_my_requests`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(requests): add my-requests application service"`

## Progress Notes
- 2026-03-09: `requests/application/user_requests.rs` を追加し、`get_my_requests` の request/correction 集約とレスポンス組み立てを handler から移設した。既存 API 変更はなし。

# EP-20260309-requests-application-cancel-request

## Goal
- `requests` の `cancel_request` を application/use-case 化し、handler から cancel 分岐と状態遷移責務を外す

## Scope
- In: `backend/src/requests/application/user_requests.rs`, `backend/src/handlers/requests.rs`, 関連ユニットテスト
- Out: `update_request` のユースケース化、admin request 操作の再設計、API contract 変更

## Done Criteria (Observable)
- [x] `cancel_request` の leave/overtime/attendance-correction 分岐と cancel 実行が `requests/application/user_requests.rs` に移動している
- [x] `handlers/requests.rs` は application 層の結果を JSON 応答へ変換するだけになっている
- [x] 新旧対象テストが成功する

## Constraints / Non-goals
- 既存 API response 形状は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] cancel 用の application 関数と戻り値型を追加
2. [x] request/correction cancel 分岐を handler から application 層へ移設
3. [x] `handlers/requests.rs` を application 呼び出しへ差し替え
4. [x] application / handler / app build テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib planned_hours_validation`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(requests): move cancel request into application service"`

## Progress Notes
- 2026-03-09: `cancel_request` の leave/overtime/attendance-correction cancel 分岐を `requests/application/user_requests.rs` に移し、handler には HTTP 応答だけを残した。既存 API 変更はなし。

# EP-20260309-requests-application-update-request

## Goal
- `requests` の `update_request` を application/use-case 化し、handler から update 分岐とバリデーション責務を外す

## Scope
- In: `backend/src/requests/application/user_requests.rs`, `backend/src/handlers/requests.rs`, 関連ユニットテスト
- Out: create request のユースケース化、admin request 操作の再設計、API contract 変更

## Done Criteria (Observable)
- [x] `update_request` の leave/overtime/attendance-correction 分岐と更新バリデーションが `requests/application/user_requests.rs` に移動している
- [x] `handlers/requests.rs` は application 層の結果を JSON 応答へ変換するだけになっている
- [x] 新旧対象テストが成功する

## Constraints / Non-goals
- 既存 API response 形状は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] update 用の application 関数と戻り値型を追加
2. [x] request/correction update 分岐とバリデーションを handler から application 層へ移設
3. [x] `handlers/requests.rs` を application 呼び出しへ差し替え
4. [x] application / handler / app build テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_requests::tests::`
- [x] `cargo test -p timekeeper-backend --lib planned_hours_validation`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(requests): move update request into application service"`

## Progress Notes
- 2026-03-09: `update_request` の leave/overtime/attendance-correction 更新分岐とバリデーションを `requests/application/user_requests.rs` に移し、handler には HTTP 応答だけを残した。既存 API 変更はなし。

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

# EP-20260306-auth-security-topics

## Goal
- backend 認証フローのレビュー指摘 4 件を、それぞれ独立した topic branch で修正する

## Scope
- In: `backend/src/handlers/auth.rs`, `backend/src/repositories/password_reset.rs`, `backend/src/models/user.rs`, `backend/src/utils/security.rs`, `backend/src/middleware/auth.rs`, `backend/tests/*`, `.agent/PLANS.md`
- Out: frontend 変更、既存 `.takt/.gitignore` 変更、インフラ設定の本番反映

## Done Criteria (Observable)
- [x] パスワードリセット token が単回利用になり、旧 token を無効化できる
- [x] メール変更時に再認証が必要になる
- [x] Cookie ベース認証の状態変更 API で CSRF 防御が統一される
- [x] ログインで未知ユーザーと既知ユーザーの処理差が緩和される
- [x] 各修正が独立した `jj` topic branch / snapshot として分離されている

## Constraints / Non-goals
- 既存の [`.takt/.gitignore`](/home/mrabbit/Documents/timekeeper/.takt/.gitignore) には触れない
- 各修正は `@-` を起点にした別 workspace で行い、ユーザーの未コミット変更を混ぜない
- SQLx migration 変更は必要な場合のみ新規追加で対応する

## Task Breakdown
1. [x] `jj workspace` を 4 つ作成し、各 topic branch の起点を分離する
2. [x] Finding 1: password reset の単回利用保証と旧 token 無効化を実装する
3. [x] Finding 2: メール変更に current password 再認証を導入する
4. [x] Finding 3: refresh/logout/session 操作に Origin 検証を統一適用する
5. [x] Finding 4: ダミーハッシュでログインのタイミング差を緩和する
6. [x] 各 workspace で関連テストを実行し、成功ごとに `jj commit` する

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --test password_reset_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test auth_api -- --nocapture`
- [x] 必要に応じて追加の unit/integration test を実行する

## JJ Snapshot Log
- [x] `jj status`
- [x] topic 1 tests pass
- [x] `jj commit -m "fix(auth): harden password reset token lifecycle"`
- [x] topic 2 tests pass
- [x] `jj commit -m "fix(auth): require re-authentication for email changes"`
- [x] topic 3 tests pass
- [x] `jj commit -m "fix(auth): enforce origin checks on cookie auth actions"`
- [x] topic 4 tests pass
- [x] `jj commit -m "fix(auth): reduce login timing side-channel"`

## Progress Notes
- 2026-03-06: 計画作成。既存 `.takt/.gitignore` 変更を避けるため、`jj workspace` で 4 件を分離して対応する方針に決定。
- 2026-03-06: `topic/auth-reset-token-lifecycle` を `75b3060d` に配置。`password_reset_api` は 9 passed。未使用 token の事前失効と、token 消費の原子的更新で単回利用を担保。
- 2026-03-06: `topic/auth-email-reauth` を `7e10dea6` に配置。`auth_flow_api` と `user_update_api` が成功。メール変更時のみ `current_password` を必須化。
- 2026-03-06: `topic/auth-origin-checks` を `f96bdcea` に配置。`verify_origin_if_cookie_present` の unit test、`session_api`、`auth_flow_api` が成功。Cookie 認証の状態変更 API に Origin 検証を統一適用。
- 2026-03-06: `topic/auth-login-timing` を `422b56aa` に配置。`auth_flow_api` 17 passed、`auth_api` 5 passed、`verify_missing_user_login_returns_unauthorized` も成功。未知ユーザー時もダミー Argon2 hash を検証してタイミング差を緩和。
