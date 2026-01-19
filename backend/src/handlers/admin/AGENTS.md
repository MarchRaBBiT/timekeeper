# Admin Handlers Implementation Guide

**Generated:** 2026-01-16
**Commit:** (not tracked)

## OVERVIEW
管理者向け API ハンドラー集約。従業員管理・勤怠閲覧・申請承認・祝日管理。監査ログ自動記録、権限チェックミドルウェア適用。

## STRUCTURE
```
admin/
├── mod.rs           # ルート定義、モジュール公開
├── users.rs         # 従業員管理（登録・編集・アーカイブ）
├── attendance.rs    # 全従業員勤怠・CSV エクスポート
├── requests.rs      # 申請一覧・承認・却下
├── holidays.rs      # 祝日管理・Google カレンダー連携 (590 lines)
└── requests_repo.rs # 関数ベース申請リポジトリ
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| 従業員 CRUD | `users.rs` | is_active フラグ管理 |
| 勤怠閲覧 | `attendance.rs` | 日付範囲・従業員フィルタ |
| 申請承認 | `requests.rs` | update_status 呼び出し |
| 祝日管理 | `holidays.rs` | Google API 連携 |
| 申請リポジトリ | `requests_repo.rs` | 検索・ステータス更新 |

## CONVENTIONS
### 権限チェック
- ミドルウェア: `auth_admin` で事前フィルタリング（Admin or System Admin）
- System Admin 専用: `auth_system_admin` ミドルウェア使用（例: システム設定）

### 監査ログ
- 自動記録: `middleware::audit_log` でリクエストパス分類
- メタデータ: リクエストボディから対象 ID・アクション抽出
- 監査対象: CRUD 操作・承認・却下

### フィルタリング
- クエリ検証: `validate_admin_*_query` 関数
- パラメータ: `page`, `per_page`, `from`, `to`, `status`
- デフォルト: 1ページ20件、日付範囲省略可能

## PATTERNS BY FEATURE

### Users (`users.rs`)
- 新規登録: ユーザー情報 + ロール指定（`role` enum）
- 編集: ユーザー情報更新（`update_user`）: 名前・Email・ロール・権限
- アーカイブ: `is_active` フラグ（論理削除）
- 権限: System Admin のみ `is_system_admin` 変更可

### Attendance (`attendance.rs`)
- 一覧取得: 日付範囲 `from`/`to`・従業員 ID `user_id` フィルタ
- CSV エクスポート: ヘッダー行 + レコード各行
- データソース: `attendance` + `users` (JOIN)

### Requests (`requests.rs`)
- 一覧取得: ステータス `status`・タイプ `type`・従業員フィルタ
- 承認/却下: `requests_repo::update_request_status` 呼び出し
- ロジック: ステータスチェック・権限チェック済み（ミドルウェア）

### Holidays (`holidays.rs`)
- 一覧取得: 年度 `year` フィルタ
- 新規作成: 日付・名前・タイプ `type`（固定祝日/法定祝日）
- Google 連携: カレンダー JSON パース・DB 一括インサート
- 重複除外: `ON CONFLICT ... DO NOTHING`

## ANTI-PATTERNS
- 権限チェックなしでデータ操作禁止（`auth_admin` 依存）
- 監査ログなしで機密操作禁止（自動記録前提）
- 生 SQL 発行禁止（必ず `requests_repo` 経由）

## COMPLEXITY NOTES
- `holidays.rs` (590 lines): Google API 連携・パースロジックが複雑
- CSV エクスポート: 個別ハンドラーに実装（将来的に utils へ分離予定）

## NOTES
- すべてのハンドラーは `Extension<User>` で管理者アカウント取得
- 監査ログは自動記録（ハンドラー側での明示的呼び出し不要）
- 関数ベース `requests_repo` はハンドラー固有ロジック（汎用は `repositories/`）
