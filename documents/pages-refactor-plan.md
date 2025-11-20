# Frontend Pages Refactor Plan

Attendance ページ以外の `frontend/src/pages` についても、`admin` リファクタリングと同じ構造・責務分離を適用する。

## 共通方針

1. **薄いエントリ**  
   - 各ページ（例: `dashboard.rs`, `requests.rs` など）は認可ガードやルーティングのみに留め、実装をサブモジュールへ委譲する。

2. **ディレクトリ構成**  
   - `frontend/src/pages/<page>/mod.rs` を追加し、`layout.rs`, `panel.rs`, `components/`, `repository.rs`, `utils.rs` を配置。
   - テスト可能なロジック（計算やバリデーション）は `utils.rs` へ、API 呼び出しは `repository.rs` へ集約する。

3. **状態管理の統一**  
   - データ取得には `create_resource`、フォーム操作には `create_rw_signal` を用い、`panel` 層でのみ副作用を管理。
   - エラーハンドリング・ローディング表現を再利用可能なコンポーネントへ抽出。

## ページ別メモ

### Dashboard ✅（想定規模: 中〜大）
- **構成**: `dashboard/mod.rs`（エントリ＋認可）、`layout.rs`（Unauthorized/Frame）、`panel.rs`（セクション結合）、`components/summary.rs`・`alerts.rs`・`activities.rs`、`repository.rs`、`utils.rs`。
- **データ取得**: `repository` に `fetch_summary()`, `fetch_alerts()`, `fetch_recent_activities()` を実装し `create_resource` で取得。必要に応じて `ResourceHandle` を `panel` 経由で各セクションに渡す。
- **状態管理**: KPI 計算 (`utils.rs`) の結果を `Memo` 化し、表示専用コンポーネントへ供給。フィルタや期間変更がある場合は `create_rw_signal` でフォームとバインド。
- **テスト**: KPI 集計やトレンド判定を `#[cfg(test)]` で検証し、外部 API 変化への回帰テストを容易にする。

### Requests ✅（想定規模: 大）
- **構成**: `requests/mod.rs` に `layout.rs`（一覧の枠）、`panel.rs`（フィルタ＋テーブル＋モーダル制御）、`components/filter.rs`, `list.rs`, `detail_modal.rs`、`repository.rs`、`types.rs`（API レスポンス用 struct）を配置。
- **モデル層**: `repository.rs` に `load_requests(filter: RequestFilter)`, `load_request_detail(id)`, `approve_request(id, comment)`, `reject_request(id, comment)` を実装し、`panel` は `Action` を介して呼び出す。
- **ビュー層**: リストとモーダルは `RequestsList`, `RequestDetailModal` として分割し、`Resource<Option<RequestDetail>>` を props で受け取る。JSON を直接扱わず `types.rs` の struct を描画。
- **状態管理**: フィルタフォーム用 `RequestFilterState` を `utils.rs` に定義し、`panel` は `create_rw_signal` で保持。モーダルの開閉・コメント入力も専用シグナルへ。
- **テスト**: フィルタロジック／表示状態切り替えを `utils.rs` の単体テストでカバー。

### Admin Users ✅（想定規模: 大）
- **構成**: `admin_users/mod.rs` に `panel.rs`, `layout.rs`, `components/list.rs`, `components/detail.rs`, `components/invite_form.rs`, `repository.rs`, `utils.rs` を設置。
- **API レイヤ**: `repository.rs` に `fetch_users()`, `update_role(user_id, role)`, `invite_user(payload)`, `reset_password(user_id)` を集約し、`Action`/`Resource` で制御。
- **フォーム/バリデーション**: 招待フォーム・ロール変更フォームを `InviteFormState` / `RoleChangeState` で管理し、メール形式チェックなどを `utils.rs` に定義。`ErrorMessage`/`SuccessMessage` は共通コンポーネントを利用。
- **コンポーネント**: `UserList`（並べ替え・検索付き）、`UserDetailDrawer`（モーダル）、`InviteForm` の 3 つを中心に組み立て、`panel` はイベントの受け渡しのみ行う。
- **テスト**: `utils.rs` 内でメール・ロール入力検証を `#[cfg(test)]` で確認。

### Admin Dashboard (進行中)
- **現状**: `frontend/src/pages/admin/panel.rs` で WeeklyHoliday/Requests/Attendance/SystemTools/Holiday を一括で描画しているが、各セクションファイル（例: `frontend/src/pages/admin/components/weekly_holidays.rs`, `frontend/src/pages/admin/components/requests.rs`）が肥大化しており、更なる責務分離が必要。
- **構造**: `admin/` 直下に `repository.rs`（週次休日・申請・勤怠・システムツール・祝日管理 API ラッパー）と `utils.rs`（WeeklyHoliday フォーム検証、RequestFilter/Pagination state など）を維持しつつ、`AdminRepository` を導入して API 呼び出しを構造体経由に統一。`layout.rs` には `AdminDashboardScaffold` を追加し、`Layout` + 認可 `Show` + `AdminDashboardFrame` を隠蔽、`panel.rs` は権限計算とセクション配置のみを担当するように整理する。
- **データ取得**: 週次休日やリクエスト一覧更新、MFA リセット・休日登録処理などは `create_resource` / `create_action` へ刷新し、`Resource`/`Action` の pending / error を `LoadingSpinner` / `ErrorMessage` で表示。`AdminRepository` が `ApiClient` をラップすることで `create_rw_signal` + 生 API 呼び出しを廃止した。
- **テスト**: `utils.rs` に分離した曜日・日付・コメント検証やページングロジックを `wasm_bindgen_test` でカバーし、アクション完了時のモーダルクローズやトースト表示は Action の `value()` を観測して一元化する。
- 2025-02-??: repository/utils/components を追加し、WeeklyHoliday/Requests/SystemTools/Attendance/Holiday の各セクションを Resource/Action ベースへ移行完了。2025-11-??: `AdminRepository` へ集約し、セクション側は依存注入のみで API を利用。

### Login / MFA ✅（構成反映済み）
- **構造**: login/mod.rs と mfa/mod.rs を導入済み。panel.rs がフォーム状態と create_action を担い、components/form.rs / components/messages.rs は UI のみを担当する。MFA は components/setup.rs／components/verify.rs の 2 ブロック構成。
- **API**: Login 側は 
rontend/src/pages/login/repository.rs で API をラップし、MFA 側の API は state::auth に統合して UI からは uth::register_mfa() / uth::activate_mfa() などを利用する。
- **フォーム/状態**: 送信処理は create_action で統一し、pending() によりボタン制御・ロード表示を実装済み。入力バリデーションは login/utils.rs / mfa/utils.rs へ集約。
- **メッセージ/UX**: エラー/成功メッセージは再利用可能なコンポーネントへ切り出し、panel 層でメッセージ状態を管理することで UX を統一している。
- **テスト**: login/utils.rs と mfa/utils.rs に wasm_bindgen_test を追加済み。加えて 
rontend/src/api/client.rs ではトークン/ラベル処理をブラウザテストで検証している。
### Attendance (補足) ✅（想定規模: 大）
- `documents/attendance-refactor-plan.md` の詳細に沿い、`mod.rs`／`layout.rs`／`panel.rs`／`components/`／`repository.rs`／`utils.rs` の構造を構築する。
- 打刻フォーム、サマリカード、履歴テーブルを個別コンポーネント化し、`repository` で API 呼び出しを一元化、`utils` でバリデーション・時間計算をテスト可能に整理する。
- 2025-02-??: `panel.rs` を `create_resource` / `create_action` ベースへ移行し、RangeForm・HolidayAlerts・HistorySection で `Resource`／`Action` の pending/error を `LoadingSpinner` / `ErrorMessage` で可視化できるようにした。

## 作業手順サマリ

1. 各ページごとにディレクトリを作成し、既存ファイルを `panel.rs` へ移動。
2. `layout.rs` と `components/` を導入してテンプレートを分割。
3. API よびロジックを `repository.rs` / `utils.rs` に抽出し、テストを追加。
4. `cargo fmt`, `cargo test -p timekeeper-frontend --lib` を用いた回帰チェックを実施。
