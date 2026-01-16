# Integration Test Structure & Conventions

**Generated:** 2026-01-16
**Commit:** (not tracked)

## OVERVIEW
pg-embed を使用した PostgreSQL 統合テスト環境。ctor による自動セットアップ、integration_guard による並列テスト制御。

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
| 並列制御 | `integration_guard()` | TRUNCATE 排他制御 |

## CONVENTIONS
### DB セットアップ
- `ctor` 使用: テスト実行前に `support/mod.rs` 初期化
- 環境変数: `TEST_DATABASE_URL` 自動設定（pg-embed 生成）
- マイグレーション: `sqlx::migrate!` 自動実行

### 並列制御
- `integration_guard()`: `tokio::sync::Mutex` ベース
- 用途: `TRUNCATE` 操作時の排他制御
- 使い方: `let _guard = integration_guard().await;`

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
- pg-embed は PostgreSQL v15 を一時起動
- テスト終了後に DB 自動破棄
- 大規模テストファイル（例: `admin_holiday_list.rs`）は複数テストケースを含む
