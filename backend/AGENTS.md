# Backend Architecture Guide

**Generated:** 2026-01-16
**Commit:** (not tracked)

## OVERVIEW
Axum + SQLx + PostgreSQL の API サーバー。JWT 認証、勤怠打刻、申請承認、管理者機能を提供。testcontainers を使用した統合テスト環境完備。

## STRUCTURE
```
backend/src/
├── main.rs           # エントリーポイント、ルーティング、マイグレーション実行
├── lib.rs            # ライブラリ（bin/token_cleanup.rs で共有）
├── handlers/         # API ハンドラー（詳細: handlers/AGENTS.md）
│   ├── admin/        # 管理者機能（詳細: handlers/admin/AGENTS.md）
│   ├── auth.rs       # 認証・MFA・パスワードリセット (642 lines - 監視要)
│   └── attendance.rs # 勤怠打刻・CSV エクスポート (610 lines)
├── repositories/     # Trait-based リポジトリ層 (password_reset.rs 含む)
├── models/           # データモデル（sqlx::FromRow + validator）
├── middleware/       # JWT認証・監査ログ・ロギング
│   ├── auth.rs       # Extension<User> 抽出、トークン検証
│   └── audit_log.rs # 監査ログ分類・メタデータ構築 (848 lines - 監視要)
├── services/         # ビジネスロジック（祝日計算など）
├── utils/            # JWT 生成、CSV エクスポート、セキュリティ、email
└── error/mod.rs      # 統一 AppError enum + IntoResponse

migrations/           # SQLx マイグレーション（001-024）
tests/               # testcontainers 統合テスト（詳細: tests/AGENTS.md）
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| 新規エンドポイント追加 | `handlers/*.rs` + `main.rs` | ルート定義追加 |
| ハンドラー規約 | `handlers/AGENTS.md` | 認証・検証・エラーハンドリング |
| 管理者機能 | `handlers/admin/` + `handlers/admin/AGENTS.md` | 権限・監査ログ |
| リポジトリ実装 | `repositories/*.rs` | Repository<T> トレイト |
| パスワードリセット | `models/password_reset.rs` + `repositories/password_reset.rs` | トークン管理・有効期限 |
| マイグレーション追加 | `migrations/` | NNN_description.sql |
| 統合テスト | `tests/*.rs` + `tests/support/mod.rs` | testcontainers + シードヘルパー |

## CONVENTIONS
### ハンドラー
- 認証: `Extension<User>` エクストラクターを使用
- 検証: ペイロード構造体に `#[derive(Validate)]` + `payload.validate()?`
- エラー: `AppError` を返す（`Result<T, AppError>` または `HandlerResult<T>`）
- 所有権チェック: `ensure_authorized_access(resource, user.id)?`
- 原則: スリムに保つ、重い DB ロジックは repository または `_repo.rs` に委譲

### リポジトリ
- パターン1: `repositories/*.rs` で `Repository<T>` トレイト実装（標準）
- パターン2: `handlers/*_repo.rs` で関数ベース実装（ハンドラー固有ロジック）
- 複雑度が高い場合、重い DB ロジックは `repositories/` 優先

### ミドルウェア
- `auth.rs`: JWT 検証、トークン失効チェック（active_access_tokens テーブル）
- `audit_log.rs`: 監査ログ分類（`classify_event` - 大規模マッチ文）
- `logging.rs`: 4xx/5xx レスポンスのバッファリング

### マイグレーション
- 命名: `NNN_description.sql`（連番）
- 実行: `main.rs` で `sqlx::migrate!("./migrations").run(&pool).await`
- 新規追加: 連番の `.sql` ファイル作成（バックエンド起動時に自動適用）

## ANTI-PATTERNS
- `unwrap()` を本番コードで禁止（`main.rs` の例外は `expect!` に書き換え予定）
- ハンドラーでの生 SQL 発行禁止（必ず repository/service 経由）
- 祝日ロジックをハンドラーに直書き禁止（必ず `services::holiday` 経由）
- 認証なしのエンドポイント追加禁止（public エンドポイントは `/auth/login` 等に限定）
- SQLxマイグレーションの変更禁止（必ず新しいファイル追加でDB操作）

## COMPLEXITY HOTSPOTS (要リファクタ)
- `middleware/audit_log.rs` (848 lines): 分類ロジックのモジュール化
- `handlers/auth.rs` (642 lines): MFA ロジックの `handlers/mfa.rs` へ分離
- `main.rs` (603 lines): ルート定義のモジュール化（`handlers::routes()`）
- `handlers/attendance.rs` (610 lines): CSV エクスポートの utils へ分離

## COMMANDS
```bash
# 開発サーバー起動
cargo run

# テスト（統合テスト含む）
cargo test

# クリニック
cargo clippy --all-targets -- -D warnings
cargo fmt --all

# トークンクリーンアップバックグラウンドタスク
cargo run --bin token_cleanup
```

## NOTES
- 環境変数 `.env` から `DATABASE_URL` 読み込み（SQLite/PostgreSQL 対応）
- 複数バイナリ: `timekeeper-backend` (main) + `token_cleanup` (bin/)
- OpenAPI ドキュメント: `/swagger-ui` 端点（utoipa 使用）
- 監査ログ: `audit_logs` テーブルに記録（主要 CRUD 操作）
