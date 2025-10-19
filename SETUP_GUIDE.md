# Timekeeper セットアップガイド

## 前提条件

### 必要なソフトウェア
- **Rust 1.70+**: [rustup.rs](https://rustup.rs/) からインストール
- **wasm-pack**: WebAssemblyビルドツール
- **Python 3.x**: フロントエンド開発サーバー用

### インストール手順

#### 1. Rustのインストール
```bash
# rustupをインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 環境変数を読み込み
source ~/.cargo/env

# バージョン確認
rustc --version
cargo --version
```

#### 2. wasm-packのインストール
```bash
cargo install wasm-pack
```

#### 3. プロジェクトのクローン
```bash
git clone <repository-url>
cd timekeeper
```

## セットアップ手順

### 1. 環境変数の設定

```bash
# 環境変数ファイルをコピー
cp env.example .env

# .envファイルを編集
nano .env
```

`.env`ファイルの内容：
```env
DATABASE_URL=sqlite:./timekeeper.db
JWT_SECRET=your-secret-key-change-this-in-production
JWT_EXPIRATION_HOURS=1
REFRESH_TOKEN_EXPIRATION_DAYS=7
```

**重要**: 本番環境では`JWT_SECRET`を強力なランダム文字列に変更してください。

### 2. バックエンドの起動

```bash
cd backend

# 依存関係のインストール
cargo build

# データベースの初期化とサーバー起動
cargo run
```

バックエンドが正常に起動すると、以下のメッセージが表示されます：
```
Server listening on 0.0.0.0:3000
```

### 3. フロントエンドのビルドと起動

新しいターミナルを開いて：

```bash
cd frontend

# WebAssemblyビルド
wasm-pack build --target web --out-dir pkg

# 開発サーバー起動
python -m http.server 8000
```

### 4. アプリケーションへのアクセス

ブラウザで以下のURLにアクセス：
- **フロントエンド**: http://localhost:8000
- **バックエンドAPI**: http://localhost:3000/api

## 初回ログイン

デフォルトの管理者アカウントでログイン：
- **ユーザー名**: `admin`
- **パスワード**: `admin123`

## 開発環境での作業

### バックエンド開発

```bash
cd backend

# 開発モードで起動（ホットリロード）
cargo run

# テスト実行
cargo test

# リントチェック
cargo clippy

# フォーマット
cargo fmt
```

### フロントエンド開発

```bash
cd frontend

# 開発ビルド
wasm-pack build --target web --out-dir pkg --dev

# 本番ビルド
wasm-pack build --target web --out-dir pkg --release

# テスト実行
wasm-pack test --headless --firefox
```

### データベース管理

```bash
cd backend

# マイグレーション実行
sqlx migrate run

# 新しいマイグレーション作成
sqlx migrate add <migration_name>

# データベースリセット
sqlx database drop
sqlx database create
sqlx migrate run
```

## トラブルシューティング

### よくある問題

#### 1. wasm-packが見つからない
```bash
# wasm-packを再インストール
cargo install wasm-pack --force
```

#### 2. データベース接続エラー
```bash
# データベースファイルの権限確認
ls -la timekeeper.db

# データベースファイルを削除して再作成
rm timekeeper.db
cargo run
```

#### 3. フロントエンドがビルドできない
```bash
# キャッシュクリア
wasm-pack build --target web --out-dir pkg --dev -- --features console_error_panic_hook

# 依存関係の更新
cargo update
```

#### 4. CORSエラー
バックエンドのCORS設定を確認し、フロントエンドのURLが許可されているか確認してください。

### ログの確認

#### バックエンドログ
```bash
cd backend
RUST_LOG=debug cargo run
```

#### フロントエンドログ
ブラウザの開発者ツールのコンソールで確認できます。

## 本番環境へのデプロイ

### Dockerを使用したデプロイ

#### 1. Dockerfileの作成

**backend/Dockerfile**:
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY backend/ .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/timekeeper-backend /usr/local/bin/
EXPOSE 3000
CMD ["timekeeper-backend"]
```

**frontend/Dockerfile**:
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY frontend/ .
RUN cargo install wasm-pack
RUN wasm-pack build --target web --out-dir pkg --release

FROM nginx:alpine
COPY --from=builder /app/pkg/ /usr/share/nginx/html/
COPY frontend/index.html /usr/share/nginx/html/
EXPOSE 80
```

#### 2. docker-compose.ymlの作成

```yaml
version: '3.8'
services:
  backend:
    build: ./backend
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=sqlite:/app/timekeeper.db
      - JWT_SECRET=your-production-secret
    volumes:
      - ./data:/app/data

  frontend:
    build: ./frontend
    ports:
      - "80:80"
    depends_on:
      - backend
```

#### 3. デプロイ実行

```bash
# イメージビルド
docker-compose build

# サービス起動
docker-compose up -d

# ログ確認
docker-compose logs -f
```

### 手動デプロイ

#### 1. バックエンドのビルド
```bash
cd backend
cargo build --release
```

#### 2. フロントエンドのビルド
```bash
cd frontend
wasm-pack build --target web --out-dir pkg --release
```

#### 3. ファイルの配置
- バックエンドバイナリをサーバーに配置
- フロントエンドファイルをWebサーバーに配置
- 環境変数を設定

## セキュリティ設定

### 本番環境での推奨設定

1. **JWT_SECRET**を強力なランダム文字列に変更
2. **HTTPS**の使用
3. **CORS**の適切な設定
4. **データベース**のバックアップ設定
5. **ログ**の適切な管理

### 環境変数の例（本番）

```env
DATABASE_URL=postgresql://user:password@localhost/timekeeper
JWT_SECRET=your-very-secure-random-secret-key-here
JWT_EXPIRATION_HOURS=1
REFRESH_TOKEN_EXPIRATION_DAYS=7
RUST_LOG=info
```

## サポート

問題が発生した場合は、以下を確認してください：

1. ログファイルの確認
2. 環境変数の設定
3. 依存関係のバージョン
4. ネットワーク接続

詳細な情報については、[README.md](README.md)と[API_DOCS.md](API_DOCS.md)を参照してください。
