# Admin Page Refactor Plan

`documents/pages-refactor-plan.md` の Admin Dashboard セクションおよび `frontend/src/pages/admin` 配下の現状を起点に、Weekly Holiday / Requests / System Tools / Attendance Tools / Holiday Management を共通のリファクタリングパターンへ統合する。

## 1. 目的

- `ApiClient::new()` と `spawn_local` が各セクションへ散在している現状を改め、API 呼び出しは `repository.rs` に集約する。
- `create_resource` / `create_action` をベースに保守性と UX（pending/error 表示）を統一し、`components/` へ UI ロジックを分離する。
- 週次休日フォームやリクエストフィルタなどのバリデーション／状態遷移を `utils.rs` と wasm テストで保証する。

## 2. ディレクトリ構成

```
frontend/src/pages/admin/
    mod.rs              # 既存
    panel.rs            # 管理パネル全体のルート（権限チェック + セクション配置のみ）
    layout.rs           # Unauthorized/Frame などの共通レイアウト
    repository.rs       # Admin API ラッパー（weekly_holidays/requests/attendance/system_tools/holidays）
    utils.rs            # WeeklyHolidayFormState, RequestFilterState などの状態・検証ロジック
    components/
        weekly_holidays.rs
        requests.rs
        system_tools.rs
        attendance.rs
        holidays.rs
```

## 3. リファクタリングステップ

1. **基盤ファイル追加**
   - `repository.rs` に週次休日・リクエスト・MFA リセット等の API 呼び出しを集約。
   - `utils.rs` にフォーム／フィルタ state とバリデーション（週次休日の日付範囲、曜日入力、コメント必須など）を定義。

2. **Weekly Holiday セクション**
   - `weekly_holidays.rs` を `components/weekly_holidays.rs` へ移動。
   - `create_resource` で一覧取得（`admin_list_weekly_holidays`）、`create_action` で登録／解除を実装し、`LoadingSpinner` / `ErrorMessage` を表示。

3. **Admin Requests セクション**
   - `requests.rs` を `components/requests.rs` へ移し、`RequestFilterState` から `create_resource` で一覧を再取得。
   - 承認／却下アクションを `create_action` 化し、モーダル状態と pending/error を統一。

4. **Attendance Tools / System Tools / Holiday Management**
   - 各セクションを `components/attendance.rs`, `components/system_tools.rs`, `components/holidays.rs` へ整理。
   - CSV エクスポート、MFA リセット、休日登録などを `create_action` で実装し、メッセージ表示は共通コンポーネントへ寄せる。

5. **panel.rs と layout の整理**
   - `panel.rs` は権限判定と `AdminDashboardFrame` の配置のみとし、各セクションへ `Resource` / `Action` ハンドルや `Memo<bool>` を渡す。
   - 認可失敗時の表示は `layout::UnauthorizedMessage` で統一。

6. **テスト & 文書**
   - `utils.rs` のバリデーション／state を `wasm_bindgen_test` でカバー。
   - `documents/pages-refactor-plan.md` の Admin セクション進捗を更新し、`cargo fmt`, `cargo test -p timekeeper-frontend --lib` を実行。

## 4. 実行ロードマップ

1. `repository.rs` / `utils.rs` / `components/mod.rs` の土台を追加。
2. Weekly Holiday セクションを新構成に移植（Resource/Action + UI 分離）。
3. Requests セクションを Resource/Action パターンへ刷新。
4. System Tools / Attendance Tools / Holiday Management を順次移行。
5. `panel.rs` を最終形に調整し、ドキュメント／テストを更新。

## 5. TODO チェックリスト

- [x] `frontend/src/pages/admin/repository.rs` と `utils.rs` を追加し、主要 API 呼び出し・state を集約する
- [x] `components/weekly_holidays.rs` を作成し、一覧取得 `Resource` と登録/削除 `Action` を実装する
- [x] `components/requests.rs` を作成し、フィルタ state + `Resource`/`Action` で承認フローを統一する
- [x] `components/system_tools.rs` / `components/attendance.rs` / `components/holidays.rs` を分離し、各アクションを `create_action` 化する
- [x] `panel.rs` を権限チェック + セクション配置のみへ簡素化し、`RequireAuth` → `AdminDashboardFrame` の構成を整理する
- [x] `utils.rs` のバリデーションや state へ wasm_bindgen_test を追加する
- [x] `documents/pages-refactor-plan.md` の Admin Dashboard 進捗を更新する
- [x] `cargo fmt` を実行する
- [x] `cargo test -p timekeeper-frontend --lib` を実行する
