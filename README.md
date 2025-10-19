# Timekeeper - 勤怠管理システム

Rustで構築された少人数向けの勤怠管理Webアプリケーションです。

## 機能

### 従業員向け機能
- 出勤/退勤の打刻
- 休憩時間の記録
- 勤務履歴の閲覧
- 有給休暇申請
- 残業申請
- 月次勤務時間の確認

### 管理者向け機能
- 従業員管理(登録/編集/削除)
- 勤怠データの閲覧/集計
- 申請の承認/却下
- 勤務データのエクスポート(CSV)
- 勤務時間の修正
- レポート生成

## 技術スタック

### バックエンド
- **フレームワーク**: Axum
- **データベース**: SQLite (将来PostgreSQL対応)
- **認証**: JWT
- **ORM**: SQLx

### フロントエンド
- **フレームワーク**: Leptos (WebAssembly)
- **スタイリング**: TailwindCSS
- **HTTP通信**: reqwest-wasm

## セットアップ

### 前提条件
- Rust 1.70+
- wasm-pack (フロントエンドビルド用)

### インストール

1. リポジトリをクローン
```bash
git clone <repository-url>
cd timekeeper
```

2. 環境変数を設定
```bash
cp env.example .env
# .envファイルを編集
```

3. wasm-packをインストール
```bash
cargo install wasm-pack
```

4. バックエンドを起動
```bash
cd backend
cargo run
```

5. フロントエンドをビルド・起動
```bash
cd frontend
wasm-pack build --target web --out-dir pkg
python -m http.server 8000
```

6. ブラウザでアクセス
```
http://localhost:8000
```

## 環境変数

```env
DATABASE_URL=sqlite:./timekeeper.db
JWT_SECRET=your-secret-key-change-this-in-production
JWT_EXPIRATION_HOURS=1
REFRESH_TOKEN_EXPIRATION_DAYS=7
```

## デフォルトアカウント

- **管理者**: username: `admin`, password: `admin123`

## プロジェクト構造

```
timekeeper/
├── backend/                 # バックエンド (Axum + SQLx)
│   ├── src/
│   │   ├── main.rs         # エントリーポイント
│   │   ├── config.rs       # 設定管理
│   │   ├── models/         # データモデル
│   │   ├── handlers/       # APIハンドラー
│   │   ├── middleware/     # JWT認証ミドルウェア
│   │   ├── db/             # データベース層
│   │   └── utils/          # ユーティリティ
│   ├── migrations/         # SQLマイグレーション
│   └── Cargo.toml
├── frontend/               # フロントエンド (Leptos + WASM)
│   ├── src/
│   │   ├── main.rs         # エントリーポイント
│   │   ├── components/     # UIコンポーネント
│   │   ├── pages/          # ページコンポーネント
│   │   ├── api/            # APIクライアント
│   │   └── state/          # 状態管理
│   ├── index.html
│   └── Cargo.toml
└── README.md
```

## API仕様

詳細なAPI仕様については [API_DOCS.md](API_DOCS.md) を参照してください。

### 認証
- `POST /api/auth/login` - ログイン
- `POST /api/auth/refresh` - トークンリフレッシュ

### 勤怠管理
- `POST /api/attendance/clock-in` - 出勤打刻
- `POST /api/attendance/clock-out` - 退勤打刻
- `POST /api/attendance/break-start` - 休憩開始
- `POST /api/attendance/break-end` - 休憩終了
- `GET /api/attendance/me` - 自分の勤怠履歴
- `GET /api/attendance/me/summary` - 月次集計

### 申請管理
- `POST /api/requests/leave` - 休暇申請
- `POST /api/requests/overtime` - 残業申請
- `GET /api/requests/me` - 自分の申請一覧

### 管理者機能
- `GET /api/admin/users` - 従業員一覧
- `POST /api/admin/users` - 従業員登録
- `GET /api/admin/attendance` - 全従業員の勤怠データ
- `PUT /api/admin/requests/:id/approve` - 申請承認
- `PUT /api/admin/requests/:id/reject` - 申請却下
- `GET /api/admin/export` - データエクスポート

## 開発

### バックエンド開発
```bash
cd backend
cargo run
```

### フロントエンド開発
```bash
cd frontend
wasm-pack build --target web --out-dir pkg --dev
python -m http.server 8000
```

### テスト
```bash
# バックエンドテスト
cd backend
cargo test

# フロントエンドテスト
cd frontend
wasm-pack test --headless --firefox
```

## ライセンス

MIT License
