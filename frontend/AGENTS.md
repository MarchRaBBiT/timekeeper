# Frontend Architecture (Leptos/WASM)

**Generated:** 2026-01-16
**Commit:** (not tracked)

## OVERVIEW
Leptos CSR-only WASM フロントエンド + TailwindCSS。MVVM パターン、集中 API クライアント、グローバル状態管理。

## REBUILD MODE NOTE
通常の保守作業ではこの文書の現行構造を source of truth にする。
`1から作り直す` / `再設計` / `rebuild` 系の作業では、先に `docs/design-docs/rebuild-architecture.md` を読み、次の target boundary へ寄せる。

- `apps/web`: Leptos app shell
- `apps/web/src/features/<feature>/`: page, view model, repository/client, feature-local components
- `crates/contract`: API DTO と OpenAPI schema
- global API client の肥大化を避け、feature client または contract-generated client に分ける

## STRUCTURE
```
frontend/src/
├── main.rs          # エントリーポイント
├── lib.rs           # ライブラリ
├── router.rs        # Leptos Router 定義 (3.8 KB)
├── config.rs        # 実行時設定 (9.7 KB - 構成ファイル)
├── api/
│   ├── client.rs    # 集中 API クライアント (692 lines - 監視要)
│   └── types.rs    # ApiError + 共通型
├── components/
│   ├── common.rs    # 共通コンポーネント
│   ├── guard.rs     # RequireAuth 認証ガード
│   └── layout.rs   # レイアウトコンポーネント
├── pages/           # 画面（詳細: 下記 WHERE TO LOOK）
│   ├── attendance/   # 勤怠打刻画面
│   ├── dashboard/   # ダッシュボード
│   ├── login/       # ログイン画面
│   ├── mfa/        # MFA 設定
│   ├── forgot_password/ # パスワードリセット要求
│   ├── reset_password/  # パスワード再設定
│   ├── requests/    # 申請画面
│   ├── admin/       # 管理者画面
│   └── admin_users/ # ユーザー管理
└── state/
    └── auth.rs     # グローバル認証状態
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| 新規画面追加 | `pages/` | MVVM: panel/view_model/repository |
| API 呼び出し | `api/client.rs` | `ApiClient` メソッド |
| 認証状態管理 | `state/auth.rs` | `AuthState` プロバイダー |
| 共通コンポーネント | `components/` | DatePicker/InlineErrorMessage 等 |
| 路由定義 | `router.rs` | `<Route path="..." view=... />` |

## CONVENTIONS
### MVVM パターン
- Panel (`panel.rs`): メインコンポーネント・View コーディネーター
- ViewModel (`view_model.rs`): 状態・ロジック・Resource/Action 管理
- Repository (`repository.rs`): API 呼び出しラッパー

例:
```
pages/attendance/
├── mod.rs
├── panel.rs         # View
├── view_model.rs    # ViewModel
└── repository.rs    # Repository
```

### API 呼び出し
- クライアント: `api::ApiClient`（Leptos Context 提供）
- Repository: ドメイン固有関数（例: `fetch_monthly_holidays`）
- データ取得: `create_resource`（読み取り）
- 副作用: `create_action`（書き込み）

### 状態管理
- ローカル: `RwSignal<T>` または `*Signal<T>`
- グローバル: Leptos Context（例: `AuthState`）
- 取得: `use_context::<T>()` または `expect_context()`

### コンポーネント構成
- 機能別: `pages/{feature}/components/`（画面固有）
- 共通: `components/`（再利用ウィジェット）
- プロパティ: `Signal<T>` または `ReadSignal<T>` 受け渡し
- イベント: `Callback<T>` で子→親通知

### UI 文言 / i18n
- user-facing copy は Rust コードへ直接書かず、translation key 経由で表示する
- 新しい表示文言を追加する場合は `frontend/locales/ja.yml` と `frontend/locales/en.yml` を同じ変更で更新する
- key 配置、例外、test 方針は `docs/manual/frontend-i18n-rule.md` を source of truth にする

## ANTI-PATTERNS
- API クライアント直呼び出し禁止（必ず `repository.rs` 経由）
- 生 DOM 操作禁止（Leptos リアクティブシステム利用）
- user-facing copy の直書き禁止（translation key 経由にする）
- プロパティドリーリング回避（Context 利用推奨）
- 未実装 TODO コメント禁止（15+ 件存在：リファクタ後判定予定）
- rebuild work で巨大 panel / view_model / global API client を移植先にも再作成することは禁止。feature boundary へ分ける

## COMPLEXITY HOTSPOTS (要リファクタ)
- `api/client.rs` (692 lines): ドメイン分割（AuthClient/AttendanceClient 等）予定
- `pages/admin/components/holidays.rs` (654 lines): コンポーネント分割予定（フィルタ・Google 連携）

## COMMANDS
```bash
# WASM ビルド（開発）
wasm-pack build --target web --out-dir pkg --dev

# 静的サーバー起動（:8000）
python -m http.server 8000

# テスト（WASM）
wasm-pack test --headless --firefox
```

## NOTES
- 認証: `RequireAuth` コンポーネントでルート保護（未認証時 `/login` へリダイレクト）
- 自動リフレッシュ: `api/client.rs` で 401 時トークン自動リフレッシュ
- CSP: `wasm-unsafe-eval`/`unsafe-inline` 許容（現在スタック制約）
- 純粋ロジック: `utils.rs` または `view_model.rs` に分離（WASM テスト容易化）
