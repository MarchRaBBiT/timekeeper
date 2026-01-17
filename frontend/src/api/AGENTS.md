# API Client Architecture Guide

**Generated:** 2026-01-16
**Commit:** (not tracked)

## OVERVIEW
フロントエンド API クライアントの集中実装。WASM 対応の reqwest-wasm、自動トークンリフレッシュ、エラーハンドリングを提供。

## STRUCTURE
```
frontend/src/api/
├── client.rs         # 集中 API クライアント（692 lines - 要リファクタ）
└── types.rs          # ApiError + 共通型定義
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| API メソッド追加 | `client.rs` | `ApiClient` impl |
| エラー型定義 | `types.rs` | `ApiError` + `IntoView` |
| トークンリフレッシュ | `client.rs` | `send_with_refresh` |
| 自動リダイレクト | `client.rs` | 401 時 `/login` へ |

## CONVENTIONS
### API 呼び出しパターン
- メソッド名：`get_`, `post_`, `put_`, `delete_` プレフィックス
- URL 構築：`format_url!` マクロまたは resolved_base_url()
- ヘッダー：`Authorization` ヘッダーまたは Cookie 設定
- レスポンス：`ApiClient::handle_unauthorized_status()` で 401 自動処理

### トークン管理
- アクセス：HttpOnly `access_token` Cookie + `Authorization` ヘッダー
- リフレッシュ：401 で自動リフレッシュ
- リフレッシュトークン：DB ハッシュ（POST /auth/refresh）

### プラットフォーム対応
- WASM: `with_credentials` で credentials 設定
- 非WASM: credentials 設定なし

## ANTI-PATTERNS
- API クライアント直呼び出し禁止（必ず `client.rs` 経由）
- エラー詳細漏洩禁止（必ず `ApiError::IntoView` 使用）
- ハードコードされた URL/エンドポイント禁止（`resolved_base_url` 使用）
- 手動トークンリフレッシュ禁止（自動リフレッシュ機構に依存）

## COMPLEXITY HOTSPOTS (要リファクタ)

### client.rs (692 lines)
- **問題点**：
  - ドメインが混在：Auth, Attendance, Requests, Admin, etc.
  - 繰り返し：URL 構築、エラーハンドリング、ヘッダー設定
  - メソッド数：50以上の API メソッド

- **リファクタ方針**：
  - ドメイン別クライアント分割：
    ```
    AuthClient: login, logout, mfa, token_refresh
    AttendanceClient: clock_in, clock_out, break_start/end
    RequestsClient: leave_request, overtime_request, approval
    AdminClient: users, holidays, statistics, export
    ```
  - 共通エラーハンドリングを `ApiClient::base()` に集約
  - URL 構築マクロを `url!` マクロで簡略化

## NOTES
- ApiClient は Leptos Context で提供（`use_context`/`expect_context`）
- 自動トークンリフレッシュは `send_with_refresh` で実装
- エラーは `ApiError` で統一・レンダリング対応
- WASM テスト用の `storage_utils` は `#[cfg(test)]` でガード
