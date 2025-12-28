# Timekeeper 環境構築ガイド

このドキュメントは `SETUP_GUIDE.md` や `README.md`、`scripts/` 内の手順を要約し、Timekeeper のローカル環境を素早く立ち上げるための手引きです。macOS / Linux / Windows (PowerShell) のいずれでも同じ流れで進められます。

## 1. 必要要件

| 用途 | ツール |
| --- | --- |
| バックエンド | [Rust 1.70+](https://www.rust-lang.org/tools/install)（`cargo` 付き） |
| フロントエンド（WASM） | `wasm-pack`（`cargo install wasm-pack`）と Python 3（静的サーバ用） |
| データベース | ローカル実行は SQLite（同梱）、PostgreSQL も `DATABASE_URL` で利用可 |
| 自動化（任意） | Podman Desktop + Podman Compose |
| テスト | Firefox（`wasm-pack test --headless --firefox`）、Node.js 18+（Playwright/E2E） |

> 補足: Windows では `scripts/backend.ps1` / `scripts/frontend.ps1` を使えば start/stop/status/logs を一括操作できます。

## 2. リポジトリの取得

```bash
git clone <repository-url>
cd timekeeper
```

## 3. 環境変数

1. テンプレートをコピー  
   ```bash
   cp env.example .env
   ```
2. `.env` を編集し、使用する DB とシークレットを設定  
   ```env
   DATABASE_URL=postgres://timekeeper:timekeeper@localhost:5432/timekeeper
   JWT_SECRET=change-me-for-local
   JWT_EXPIRATION_HOURS=1
   REFRESH_TOKEN_EXPIRATION_DAYS=7
   AUDIT_LOG_RETENTION_DAYS=365
   AUDIT_LOG_RETENTION_FOREVER=false
   ```
3. ローカル PostgreSQL / ステージングで利用する場合は `DATABASE_URL` を DSN に置き換え、`JWT_SECRET` を十分ランダムな値に更新してください（Podman Compose でも `.env` を直接読むため）。

> SQLite を利用する場合は `DATABASE_URL=sqlite:./timekeeper.db` のように書き換えてください。既定値と `env.example` は PostgreSQL を前提としています。
> 監査ログの保持期間は `AUDIT_LOG_RETENTION_DAYS=0` で記録無効化、`AUDIT_LOG_RETENTION_FOREVER=true` で削除無効化します。両方指定した場合は FOREVER を優先します。

## 4. バックエンドのセットアップ

```bash
cd backend
cargo fetch          # 任意：依存取得のみ
cargo sqlx prepare   # オフラインモードを使う場合
cargo run            # マイグレーション適用 + Axum API (3000番) 起動
```

便利なコマンド:

- `pwsh -File ..\scripts\backend.ps1 start|stop|status|logs`
- `cargo test`, `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`

初回起動時には `Server listening on 0.0.0.0:3000` が表示されれば成功です。

## 5. フロントエンドのセットアップ

```bash
cd frontend
wasm-pack build --target web --out-dir pkg --dev
python -m http.server 8000
```

`http://localhost:8000` にアクセスして UI を確認します。SPA は `http://localhost:3000/api` をバックエンドとして期待します。

補助ツール:

- `pwsh -File ..\scripts\frontend.ps1 start`（ビルド + 静的サーバ起動をまとめて実行）
- `wasm-pack test --headless --firefox`（フロントの単体テスト）

## 6. デフォルトアカウント

マイグレーション実行後、以下のシステム管理者アカウントが自動で作成されます。

```
username: admin
password: admin123
```

このユーザーでログインし、UI や `POST /api/admin/users` から従業員/管理者を追加してください。

## 7. Podman（任意）

リポジトリには以下のコンテナ定義が含まれています。

- `backend/Dockerfile`（Rust ビルド → Debian スリム）
- `frontend/Dockerfile`（Rust ビルド + nginx）
- compose 定義（`docker-compose.yml` / `.example`）

クイックスタート:

```bash
podman compose up --build
```

`.env` あるいは Compose の `environment` セクションでシークレットを上書きし、SQLite/PostgreSQL を永続化したい場合はボリュームをマウントしてください。

## 8. E2E スモークテスト

1. バックエンドとフロントエンドを起動（例：`scripts/backend.ps1 start`、`scripts/frontend.ps1 start`）。
2. `e2e/` ディレクトリで `npm install`（初回のみ）を実行し、`node run.mjs` を起動。`FRONTEND_BASE_URL` が `http://localhost:8080` 以外なら環境変数で上書きします。

Playwright スクリプトは管理者でログインし、主要ページを巡回してログアウトするスモークシナリオです。

---

より詳細な OS 別手順は `SETUP_GUIDE.md`、API のリクエスト/レスポンスは `API_DOCS.md` を参照してください。
