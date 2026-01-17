# Admin Components Architecture Guide

**Generated:** 2026-01-16
**Commit:** (not tracked)

## OVERVIEW
管理者向けコンポーネント集約。祝日管理、データエクスポート、Google カレンダー連携等の複雑 UI ロジックを実装。

## STRUCTURE
```
frontend/src/pages/admin/components/
├── holidays.rs           # 祝日管理・Google 連携（654 lines - 要リファクタ）
├── attendance.rs         # 勤怠集計・CSV エクスポート
├── requests.rs           # 申請承認・却下
└── weekly_holidays.rs    # 週間祝日表示
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| 祝日管理 | `holidays.rs` | CRUD + Google カレンダー連携 |
| 勤怠集計 | `attendance.rs` | 全従業員勤怠一覧・CSV |
| 申請承認 | `requests.rs` | 休暇・残業申請一覧 |
| 週間祝日 | `weekly_holidays.rs` | カレンダー表示コンポーネント |

## CONVENTIONS
### コンポーネント構成
- メインセクション：`view!` マクロで UI 定義
- 状態管理：`RwSignal<T>` で入力・エラー・ロード状態
- データ取得：`create_resource` でサーバー同期
- 副作用：`create_action` で API 呼び出し

### API 呼び出し
- Repository: `admin_repository` から関数呼び出し
- API Client: `api::ApiClient` 経由（`use_context`）
- エラー処理：`ApiError::IntoView` で自動表示

### ページネーション
- 管理者ページ: `/admin` → `/admin/{feature}`
- ルート定義：`frontend/src/router.rs` で定義

## ANTI-PATTERNS
- API クライアント直呼び出し禁止（必ず repository 経由）
- 状態管理をコンポーネント内で密結合禁止（必要に応じて Context 分離）
- 重複コードのコピーペースト禁止（再利用コンポーネント化）

## COMPLEXITY HOTSPOTS (要リファクタ)

### holidays.rs (654 lines)
- **問題点**：
  - 654 行の単一コンポーネント
  - UI ロジック・フィルタリング・Google 連携が混在
  - `view!` マクロが 200 行超

- **リファクタ方針**：
  - コンポーネント分割：
    ```
    components/
    ├── HolidayFilterPanel    # フィルタ入力
    ├── HolidayTable           # 一覧表示・ページネーション
    ├── HolidayForm            # 作成・編集フォーム
    └── GoogleImportSection     # Google カレンダー連携
    ```
  - Google 連携ロジックを `hooks/use_google_calendar.ts` に分離
  - ページネーションを独立コンポーネントに移動

## NOTES
- 祝日管理は日本祝日 + Google カレンダー連携
- Google カレンダー JSON は `utils::calendar` でパース
- CSV エクスポートは `attendance.rs` で実装（holidays.rs ではない）
