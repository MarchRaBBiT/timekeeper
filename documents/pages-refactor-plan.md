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

### Login / MFA ✅（想定規模: 中）
- **構成**: `login/mod.rs` と `mfa/mod.rs` を用意し、それぞれ `panel.rs`（フォーム＋ロジック）、`components/form.rs`, `components/message.rs`, `repository.rs`, `utils.rs` を持つ。
- **API**: 認証系 API を `repository.rs` で `login(credentials)`, `request_mfa_code(user_id)`, `verify_mfa_code(request)` などに抽象化し、UI 層から直接 `ApiClient` を触らない。
- **フォーム**: 入力状態とバリデーション（メール形式/パスワード長/MFA コード桁数）を `FormState` と `utils.rs` で管理。`Action` を利用して送信中の状態 (`pending()`) をボタンへ反映。
- **メッセージ**: 成功/失敗メッセージを `components::AuthMessage` で統一し、再利用性とアクセシビリティを確保。
- **テスト**: バリデーション関数とトークンパースを `#[cfg(test)]` で検証。

### Attendance (補足) ✅（想定規模: 大）
- `documents/attendance-refactor-plan.md` の詳細に沿い、`mod.rs`／`layout.rs`／`panel.rs`／`components/`／`repository.rs`／`utils.rs` の構造を構築する。
- 打刻フォーム、サマリカード、履歴テーブルを個別コンポーネント化し、`repository` で API 呼び出しを一元化、`utils` でバリデーション・時間計算をテスト可能に整理する。

## 作業手順サマリ

1. 各ページごとにディレクトリを作成し、既存ファイルを `panel.rs` へ移動。
2. `layout.rs` と `components/` を導入してテンプレートを分割。
3. API よびロジックを `repository.rs` / `utils.rs` に抽出し、テストを追加。
4. `cargo fmt`, `cargo test -p timekeeper-frontend --lib` を用いた回帰チェックを実施。
