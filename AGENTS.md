# リポジトリ ガイドライン

**Generated:** 2026-01-16
**Commit:** (not tracked)
**Project:** Timekeeper - 勤怠管理システム

## OVERVIEW
Rust/Axum バックエンド + Leptos/WASM フロントエンド + Playwright E2E の勤怠管理システム。小〜中規模チーム向けの認証・勤怠打刻・申請承認ワークフローを提供。

## STRUCTURE
```
timekeeper/
├── backend/          # Axum API + SQLx + PostgreSQL (118 Rust files)
│   ├── src/
│   │   ├── handlers/       # API ハンドラー（認証・勤怠・申請）
│   │   ├── repositories/   # リポジトリ層（Trait-based）
│   │   ├── models/         # データモデル
│   │   ├── middleware/     # JWT/認証/ロギング
│   │   ├── services/       # ビジネスロジック（祝日）
│   │   └── bin/           # ユーティリティ（token_cleanup）
│   ├── migrations/     # SQLx マイグレーション
│   └── tests/          # 統合テスト（testcontainers）
├── frontend/         # Leptos WASM + TailwindCSS (77 Rust files)
│   ├── src/
│   │   ├── pages/         # ルーティング画面（MVVM pattern）
│   │   ├── components/    # 再利用コンポーネント
│   │   ├── api/           # API クライアント（reqwest-wasm）
│   │   └── state/         # グローバル状態管理
├── e2e/             # Playwright スモークテスト
├── scripts/          # PowerShell 自動化（起動/ビルド/テスト）
└── docs/            # コンポーネント/API ドキュメント
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| バックエンド API 追加 | `backend/src/handlers/` | 参考: `backend/AGENTS.md` |
| リポジトリ実装 | `backend/src/repositories/` | Trait-based pattern |
| マイグレーション | `backend/migrations/` | SQLx, auto-run on start |
| フロントエンド画面追加 | `frontend/src/pages/` | MVVM: panel/view_model/repository |
| API 呼び出し | `frontend/src/api/client.rs` | Centralized client |
| 統合テスト | `backend/tests/` | testcontainers + ctor |
| E2E スモーク | `e2e/*.mjs`, `scripts/test_backend.ps1` | Playwright + PowerShell |

## CONVENTIONS
### 共通
- エンコーディング: UTF-8
- 改行コード: LF
- 言語: 英語で思考、日本語でコミュニケーション
- コミット: Conventional Commits (`feat:`, `fix:`, `chore:`)
- バージョン管理: `git` コマンドは使わず `jj` (jujutsu) を使用

### Rust 全般
- インデント: 4 スペース
- 命名: モジュールは `snake_case`、型は `PascalCase`
- 事前確認: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`

### バックエンド (Axum)
- 認証: `Extension<User>` エクストラクター
- 検証: `validator` crate, `payload.validate()?`
- エラー: 統一 `AppError` enum (`IntoResponse` 実装)
- テスト: 統合テストは `testcontainers` で実 DB 使用

### フロントエンド (Leptos)
- 状態: `*_signal` または `use_*` ヘルパー
- コンポーネント: `PascalCase`
- アーキテクチャ: MVVM pattern (`panel.rs` + `view_model.rs` + `repository.rs`)

## ANTI-PATTERNS (THIS PROJECT)
- 型エラー抑制を禁止（`as any`, `@ts-ignore`, `@ts-expect-error`）
- 空の catch ブロックを禁止
- 既存ロジックを空関数で置き換えを禁止（必ず実装まで完了）
- ユーザーの明示的許可なしに挙動/仕様を簡略化または削除を禁止
- ハンドラーの肥大化を回避（重い DB ロジックは repository/service に委譲）
- フロントエンドの大幅なリファクタリングは UI/UX 変更を含む場合 `frontend-ui-ux-engineer` に委譲
- SQLxマイグレーションの変更禁止（必ず新しいファイル追加でDB操作）

## COMMANDS
```bash
# バックエンド起動
cd backend; cargo run
# バックエンド（Podman経由）
pwsh -File .\scripts\backend.ps1 start

# フロントエンドビルド＆起動
pwsh -File .\scripts\frontend.ps1 start

# バックエンドテスト
./scripts/test_backend_integrated.sh
# API スモークテスト
pwsh -File .\scripts\test_backend.ps1

# フロントエンドテスト（WASM）
cd frontend; wasm-pack test --headless --firefox

# E2E スモーク
cd e2e; node run.mjs
```

## NOTES
- 環境変数: `.env` に設定（`env.example` をコピー）
- データベース: `DATABASE_URL` で PostgreSQL or SQLite を指定
- PID ファイル: `.backend.pid`, `.frontend.pid` でプロセス管理（`.gitignore` に追加済み）
- マイグレーション: バックエンド起動時に自動実行
- 詳細ガイド: `CODING_STANDARD.md`（関数設計・リファクタリング）

## SUBDIRECTORIES
- `backend/AGENTS.md` - バックエンド詳細（ハンドラー・リポジトリ・テスト）
- `backend/src/middleware/AGENTS.md` - ミドルウェア層のアーキテクチャ（監査ログ・認証・ロギング）
- `backend/src/handlers/AGENTS.md` - ハンドラー層のパターンと規約
- `backend/src/handlers/admin/AGENTS.md` - 管理者機能の実装ガイド
- `backend/tests/AGENTS.md` - 統合テストの構造と規約
- `frontend/AGENTS.md` - フロントエンド詳細（MVVM・API・状態管理）
- `frontend/src/api/AGENTS.md` - API クライアントのアーキテクチャ（トークン管理・自動リフレッシュ）
- `frontend/src/pages/admin/components/AGENTS.md` - 管理者コンポーネントのアーキテクチャ（祝日管理・UI ロジック）

## RUST SKILLS (GENERAL)
The following skills are available for general Rust development tasks:
- `rust_router`: Question routing for Rust concepts.
- `coding_guidelines`: Project-wide settings and style enforcement.
- `unsafe_checker`: Safety validation for `unsafe` blocks.
