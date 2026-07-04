# Integration Test Structure & Conventions

**Generated:** 2026-01-16
**Commit:** (not tracked)

## OVERVIEW
testcontainers を使用した PostgreSQL 統合テスト環境。ctor による自動セットアップ、integration_guard による並列テスト制御。

## STRUCTURE
```
tests/
├── support/
│   └── mod.rs          # テストインフラ・シードヘルパー（大規模）
├── admin_holiday_list.rs  # 祝日一覧テスト (770 lines - 大規模)
├── audit_log_middleware.rs # 監査ログテスト (568 lines)
├── *.rs                 # 各機能テスト（auth/attendance/requests 等）
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| テストインフラ | `support/mod.rs` | DB セットアップ・シード |
| ユーザーシード | `support::seed_user` | ロール・権限設定 |
| 休暇シード | `support::seed_leave_request` | ステータス・期間 |
| 残業シード | `support::seed_overtime_request` | 承認済・却下済 |
| 祝日シード | `support::seed_holiday` | 固定・法定祝日 |
| 並列制御 | `support::integration_guard()` | TRUNCATE 排他制御（同一バイナリ内） |
| env mutation | `support::EnvVarGuard` | RAII で snapshot/restore を保証する env var 変更 |
| fixture profile | `support::profile::{db_only, db_and_redis, db_and_smtp_skip, db_and_smtp_failure}` | 外部依存を明示するテストセットアップ |

## CONVENTIONS
### DB セットアップ
- `ctor` 使用: テスト実行前に `support/mod.rs` 初期化
- 環境変数: `TEST_DATABASE_URL` 自動設定（testcontainers 起動）
- マイグレーション: `sqlx::migrate!` 自動実行

### 並列制御
- `support::integration_guard()`: `tokio::sync::Mutex` ベースの共有関数（`backend/tests/support/mod.rs` に集約済み。旧来は各ファイルに同一実装がコピーされていたが 2026-07-04 に統合）
- 用途: 同一 test binary（= 同一ファイル）内の `#[tokio::test]` 間での `TRUNCATE` 排他制御
- 使い方: `use support::integration_guard;` の上で `let _guard = integration_guard().await;`
- 適用範囲の限界: `cargo test` は test binary を逐次実行するため単一 invocation 内では cross-file 競合は起きないが、共有 Postgres（`scripts/test_backend_integrated.sh` 経由）に対して**複数の `cargo test` invocation を同時に**走らせるとこの mutex では防げない。cross-invocation の直列化は `scripts/harness.sh` の `flock` ベース lock（`BACKEND_INTEGRATION_LOCK`）が担う。詳細は [docs/manual/HARNESS.md](../../docs/manual/HARNESS.md) の "Suite Execution Model" を参照
- `admin_holiday_list.rs` のみ、`#[cfg(feature = "test-utils")]` 配下の同期版 `integration_guard()` を独自に持つ（今回の集約対象外。follow-up）

### env mutation
- `support::EnvVarGuard::new(&[...])`: 指定した env var キーの現在値を snapshot し、drop 時に自動 restore する RAII guard
- 直接 `env::set_var` / `env::remove_var` を書くのではなく、この guard 経由でテスト用の env 変更を行う
- SMTP 系キーは `support::profile::SMTP_ENV_KEYS` を再利用する

### fixture profile
- `support::profile::db_only()`: Postgres のみ（既定。多くのテストは `support::test_pool()` を直接使い、暗黙にこの profile に従う）
- `support::profile::db_and_redis()`: Postgres + ephemeral Redis 7 testcontainer（`DbRedisFixture { pool, redis_url, .. }`）
- `support::profile::db_and_smtp_skip()`: Postgres + `SMTP_SKIP_SEND=true`
- `support::profile::db_and_smtp_failure()`: Postgres + 到達不能な `SMTP_HOST`/`SMTP_PORT`（送信失敗を強制）
- 全テストをこれらの profile へ強制移行してはいない。新規の DB+Redis / DB+SMTP テストを書くときはこの profile を使う

### シードヘルパー
- 命名: `support::seed_*` (例: `seed_user`, `seed_leave_request`)
- 引数: `&PgPool` + 必須フィールド（例: `role`）
- 戻り値: 作成されたモデル（例: `User`）

### テスト構造
```rust
#[tokio::test]
async fn test_feature() {
    let pool = get_test_pool().await;
    let _guard = integration_guard().await;

    // シード
    let user = support::seed_user(&pool, Role::Admin).await;

    // 実行
    let response = client.post("/api/...").json(&payload).send().await;

    // アサート
    assert_eq!(response.status(), StatusCode::OK);
}
```

## ANTI-PATTERNS
- `integration_guard()` なしで `TRUNCATE` 使用禁止
- ハードコードされた `DATABASE_URL` 使用禁止（必ず `get_test_pool()`）
- シードヘルパーを再実装禁止（必ず `support::` 使用）

## NOTES
- testcontainers は PostgreSQL v15 を一時起動
- Podman 利用時は `DOCKER_HOST=unix:///run/podman/podman.sock` または `unix:///run/user/$UID/podman/podman.sock` を設定
- テスト終了後に DB 自動破棄
- 大規模テストファイル（例: `admin_holiday_list.rs`）は複数テストケースを含む
