# Handler Layer Patterns

**Generated:** 2026-01-16
**Commit:** (not tracked)

## OVERVIEW
Axum API ハンドラー層。認証・検証・エラーハンドリングの標準パターン適用。Extension<User> によるユーザー抽出、統一 AppError 使用。

## STRUCTURE
```
handlers/
├── mod.rs            # ルート定義、モジュール公開
├── auth.rs           # 認証・MFA (642 lines - 要リファクタ)
├── attendance.rs      # 勤怠打刻・CSV エクスポート (610 lines)
├── requests.rs       # 休暇・残業申請
├── admin/            # 管理者機能（詳細: admin/AGENTS.md）
│   ├── mod.rs
│   ├── users.rs      # 従業員管理（登録・編集・アーカイブ）
│   ├── attendance.rs # 全従業員勤怠・CSV エクスポート
│   ├── requests.rs   # 申請承認・却下
│   ├── holidays.rs   # 祝日管理 (590 lines)
│   └── requests_repo.rs # 関数ベースリポジトリ
├── auth_repo.rs      # 関数ベース認証リポジトリ
└── attendance_utils.rs # 勤怠ヘルパー（ステートチェック）
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| 認証エンドポイント | `auth.rs` | login/refresh/MFA |
| 勤怠エンドポイント | `attendance.rs` | clock-in/break/CSV |
| 申請エンドポイント | `requests.rs` | leave/overtime |
| 管理者エンドポイント | `admin/*` | 権限チェック・監査ログ |
| DB ヘルパー | `*_repo.rs` | 関数ベース実装 |
| 検証パターン | `models/*.rs` | `#[derive(Validate)]` |

## CONVENTIONS
### 認証・認可
- ユーザー抽出: `Extension<User>` エクストラクター
- 管理者エンドポイント: `main.rs` で `auth_admin` ミドルウェア適用
- 所有権チェック: `ensure_authorized_access(resource, user.id)?`

### 検証
- 構造体: `#[derive(Validate)]` + `#[validate(...)]`
- ハンドラー: `payload.validate()?` を実行（明示的）
- クエリ: カスタム検証関数（例: `validate_list_query`）

### エラーハンドリング
- 戻り値: `Result<T, AppError>` または `HandlerResult<T>` エイリアス
- 自動変換: `From<sqlx::Error>`, `From<validator::ValidationErrors>`
- レスポンス形式: `{"error": "...", "code": "...", "details": {...}}`

### 構造
- スリム原則: 重い DB ロジックは repository/service に委譲
- 繰り返し回避: ヘルパーモジュール（`attendance_utils.rs`）
- 共有ロジック: `_repo.rs` ファイルで関数ベース実装

## ANTI-PATTERNS
- `Extension<User>` 以外のトークン手動解析禁止
- 検証なしで JSON ペイロード使用禁止
- 生 SQL 発行禁止（必ず repository 経由）
- ハンドラーでの CSV 構築禁止（utils へ分離予定）

## PATTERNS BY FEATURE

### Auth (`auth.rs`)
- パスワードハッシュ: `argon2`
- トークン: JWT (access) + DB ハッシュ (refresh)
- MFA: TOTP (`totp-rs`)
- クッキー: HttpOnly `access_token`/`refresh_token`
- パスワードリセット: `password_resets` テーブル + メール通知
- セルフサービス更新: プロフィール (email/name) 更新エンドポイント (`PUT /api/auth/me`)

### Attendance (`attendance.rs`)
- ステートマシン: 打刻前 → 出勤中 → 休憩中 → 退勤済
- 祝日考慮: `HolidayService` 経由
- CSV エクスポート: `utils::csv` モジュール予定

### Requests (`requests.rs`)
- ステータス: `Pending` → `Approved`/`Rejected`
- 重複申請防止: DB ユニーク制約 + 楽観的ロック

### Admin (`admin/`)
- 権限チェック: `auth_admin` ミドルウェアで事前フィルタリング
- 監査ログ: `middleware::audit_log` 自動記録
- フィルタリング: 共通 `validate_admin_*_query` 関数

## NOTES
- `main.rs` ですべてのルートを定義（今後モジュール化予定）
- 管理者機能は `admin/` ディレクトリに集約
- `*_repo.rs` はハンドラー固有の DB ロジック（汎用ロジックは `repositories/`）
