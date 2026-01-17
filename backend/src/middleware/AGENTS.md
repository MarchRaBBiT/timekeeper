# Middleware Architecture Guide

**Generated:** 2026-01-16
**Commit:** (not tracked)

## OVERVIEW
Axum ミドルウェア層。JWT 認証、監査ログ、リクエストID、ロギング、レート制限を実装。リクエストライフサイクルとセキュリティを分離。

## STRUCTURE
```
backend/src/middleware/
├── mod.rs              # ミドルウェア公開
├── auth.rs              # JWT 認証、Extension<User> 抽出
├── audit_log.rs         # 監査ログ（848 lines - 要リファクタ）
├── logging.rs           # HTTP レスポンスロギング
├── request_id.rs        # リクエストID 生成・伝播
└── rate_limit.rs         # API レート制限
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| JWT 認証ミドルウェア | `auth.rs` | Extension<User> 抽出、トークン失効チェック |
| 監査ログミドルウェア | `audit_log.rs` | イベント分類・メタデータ構築 |
| HTTP ロギング | `logging.rs` | 4xx/5x レスポンスバッファリング |
| リクエストID | `request_id.rs` | UUID 生成・ヘッダー伝播 |
| レート制限 | `rate_limit.rs` | IP ベースレート制限 |

## CONVENTIONS
### ミドルウェアチェーン
- 順序：CORS → Trace → RequestId → Auth → Handler
- 各ミドルウェアは独立モジュール、State で共有
- 次のミドルウェアへ request を渡す（next.run(request).await）

### 認証フロー
- JWT 検証：`Authorization` ヘッダーまたは `access_token` Cookie
- トークン失効：`active_access_tokens` テーブルでチェック
- ユーザー抽出：`Extension<User>` エクストラクター
- 権限階層：`auth_admin`/`auth_system_admin` ミドルウェア

### 監査ログ
- イベント分類：`classify_event` 関数でパスとメソッドをマッピング
- メタデータ構築：`build_metadata` でリクエストボディから抽出
- 非同期記録：バックグラウンドタスクで DB に記録
- 自動適用：ルート定義でミドルウェアスタック適用

## ANTI-PATTERNS
- 生トークン検証をミドルウェアに直書き禁止（必ず `auth.rs` 経由）
- ハンドラー内で認証状態を手動設定禁止（必ず Extension<User> 使用）
- 監査対象外の機密操作を記録禁止（自動分類前提）

## COMPLEXITY HOTSPOTS (要リファクタ)

### audit_log.rs (848 lines)
- **問題点**：
  - `classify_event`: 170行の巨大な match 文
  - `build_metadata`: 200行の複雑なメタデータ構築
  - 複数のドメインモデルと密結合

- **リファクタ方針**：
  - イベント分類をドメイン別モジュールに分割（例: `attendance_audit.rs`）
  - メタデータ構築を各ドメインのヘルパーに移動
  - 監査ログ設定を外部ファイルに分離

## NOTES
- 監査ログは `audit_logs` テーブルに記録
- ミドルウェアは State 経由で設定と DB プールを共有
- 全ミドルウェアは非同期実行可能（Handler は非ブロッキング）
- リクエスト ID はトレーシングとデバッグで使用
