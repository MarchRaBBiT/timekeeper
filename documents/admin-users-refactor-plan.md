# Admin Users Page Refactor Plan

`documents/pages-refactor-plan.md` の Admin Users セクションを起点に、現在 `frontend/src/pages/admin_users.rs` にまとまっている UI・状態管理・API 呼び出しをパネル／コンポーネント／リポジトリ単位へ分解する。ここでは Admin Users ページ専用のモジュール構造と移行手順を詳細化し、他ページのリファクタリング計画と整合するロードマップを提供する。

## 1. 目的

- システム管理者専用ページをモジュール化し、Admin Dashboard や Attendance で採用する `layout.rs` / `panel.rs` / `components/` / `repository.rs` / `utils.rs` パターンへ揃える。
- API 呼び出しを `repository.rs` に隔離し、`panel` では `create_resource` / `create_action` ベースの状態遷移のみ扱うシンプルな構造へ移す。
- ユーザー一覧、詳細ドロワー、招待フォーム、権限更新といった UI 部品を再利用可能なコンポーネントに分割し、テスト容易性を高める。
- 状態管理（フォーム入力、エラー／成功メッセージ、権限チェック）を `utils.rs` の小さなステートマシンへ移し、TDD で検証可能にする。

## 2. 現状と課題 (`frontend/src/pages/admin_users.rs`)

1. 単一ファイルにフォーム・一覧・API 呼び出し・認可判定・トースト表示が密集しており、保守や差分テストが困難。
2. API 呼び出しは `ApiClient::new()` を直呼びし、エラーや pending UI の分岐も `view!` 内に散在。
3. フォーム状態は `create_rw_signal` の羅列で、入力検証やリセット処理を共有できない。
4. システム管理者チェックや空欄バリデーションのテストが存在せず、リグレッション検出が難しい。
5. 招待／権限更新／MFA リセットなど追加要件を実装しづらく、`documents/pages-refactor-plan.md` の再利用方針と乖離。

## 3. 目標アーキテクチャ

### 3.1 ディレクトリ構成

```
frontend/src/pages/admin_users/
    mod.rs                # ルーティング露出（`pub use panel::AdminUsersPage;`）
    layout.rs             # 認可ガード / breadcrumb / タイトル
    panel.rs              # 状態管理 + Resource/Action の配線（UI ロジックなし）
    components/
        list.rs           # UserList テーブル（ソート／行クリック）
        detail.rs         # UserDetailDrawer（権限変更・MFA リセット・リセット結果表示）
        invite_form.rs    # InviteForm（入力・検証・Submit ボタン）
    repository.rs         # get_users / create_user / update_role / reset_password / invite_user の API ラッパー
    utils.rs              # InviteFormState / RoleChangeState / MessageState / guard ヘルパー
    types.rs              # Repository で使う request/response/enum のみ（必要なら `api::types` を re-export）
```

### 3.2 責務整理

- `mod.rs`: 既存の `frontend/src/pages/mod.rs` から `AdminUsersPage` を re-export し、`panel.rs` との境界を一本化。
- `layout.rs`: `state::auth::use_auth` を使ったシステム管理者ガード・パンくず・共通カード枠。Pending/Error 表示はここで吸収。
- `panel.rs`:  
  - `create_resource(fetch_users)` で一覧を取得。  
  - `create_action(invite_user)` / `create_action(update_role)` / `create_action(reset_password)` を束ね、Action の pending/error を `MessageState` へ集約。  
  - `create_rw_signal(SelectedUserId)` や `create_memo(IsSystemAdmin)` など軽量シグナルのみ保持し、UI への props 提供に専念。
- `components/:` 純粋 UI（signals を props 経由で受け、イベントをクロージャでパネルに返す）。
- `repository.rs`:  
  - `AdminUsersRepository` 構造体で `ApiClient` を注入し、テスト時はモックを差し替え可にする。  
  - `async fn fetch_users()` / `async fn invite_user()` / `async fn update_role()` / `async fn reset_mfa()` を定義。必要に応じて `Result<_, RepositoryError>` を返却。
- `utils.rs`:  
  - `InviteFormState`（入力値・バリデーション・`reset()`）  
  - `RoleChangeState`（選択ユーザー・新ロール・`is_dirty()`）  
  - `MessageState`（Success/Error メッセージと TTL）  
  - これらに `#[cfg(test)]` のユニットテストを付与して TDD を実践。

## 4. データ／状態フロー

1. `panel.rs` 初期化時に `create_resource` で `repository.fetch_users()` を呼び、`Resource<UsersResponse, _>` を `UserList` へ渡す。pending/error は `LoadingSpinner` / `ErrorMessage` コンポーネントで表示。
2. `InviteForm` は `InviteFormState` と `Action` を props でもらい、入力更新を `state::update_field` メソッドに委譲。送信イベントは `invite_user_action.dispatch(form_state.to_payload())` を実行。
3. `UserList` の行クリックで `SelectedUserSignal` を更新し、`UserDetailDrawer` が `Option<UserResponse>` と `RoleChangeState` を受け取って開閉を制御。
4. `RoleChangeState` は `update_role_action` と連携。成功時は `MessageState::success("ロールを更新しました")` をセットし、`fetch_users_resource.refetch()` で再取得。
5. MFA リセットやパスワードリセットなど副作用アクションはそれぞれ `create_action` で包み、`Action::pending()` に応じたボタン無効化／スピナー表示を Drawer 内で行う。

## 5. リファクタリング手順

1. **モジュール雛形の作成**  
   - `frontend/src/pages/admin_users/` 配下に `mod.rs` / `layout.rs` / `panel.rs` の空ファイルを追加し、既存 `admin_users.rs` から最低限のページエクスポートのみ移植。  
   - ルーティング (`frontend/src/pages/mod.rs`) で新モジュールを指すよう更新し、ビルドが通るまで段階的に削除。

2. **Repository & Types 抽出 (TDD)**  
   - `repository.rs` に `AdminUsersRepository` を実装し、既存ページの `ApiClient::create_user` / `get_users` 呼び出しを移す。  
   - 将来的な `update_role`, `reset_password`, `invite_user` も関数だけ先行定義し、`#[cfg(test)]` でエラー・成功パスをモック化して振る舞いを固定。  
   - ここで `documents/pages-refactor-plan.md` に追記された API 設計と整合するか確認。

3. **State & Utils 切り出し**  
   - `InviteFormState` / `RoleChangeState` / `MessageState` を `utils.rs` へ作成し、フォームリセットやバリデーションを関数化。  
   - `wasm_bindgen_test` / 通常ユニットテストで状態遷移を検証（例: 空欄時に `is_valid()` が false になる）。

4. **UI コンポーネント分割**  
   - まず `InviteForm` を分離し、`panel` は props を構築して渡すだけにする。  
   - 続いて `UserList` + `UserDetailDrawer` を実装し、ドロワーからのイベント（ロール更新・MFA リセット）を `panel` 経由で Action に伝搬。  
   - それぞれ Storybook 代替として `wasm_bindgen_test` で props しばりのスナップショットテストを検討。

5. **最終統合と削除**  
   - 旧 `frontend/src/pages/admin_users.rs` を削除し、新モジュールへ完全移行。  
   - `panel` で `create_resource` / `create_action` の pending/error を `components::messages.rs` 系と共有。  
   - `documents/pages-refactor-plan.md` の Admin Users セクションに作業履歴を反映し、完了条件（テスト結果等）を記録。

## 6. テスト戦略

- `utils.rs`: `InviteFormState::is_valid`, `reset()`、`RoleChangeState::apply` などを `#[cfg(test)]` で TDD。  
- `repository.rs`: `async fn fetch_users` などを `wasm_bindgen_test` + `gloo_net::http::Mock` で API エラーハンドリングを検証。  
- `panel.rs`: リソース／アクションの pending フラグが UI に伝搬するかを `wasm_bindgen_test` で確認。  
- `components/`: フォームやドロワーの表示切替を `wasm_bindgen_test` で DOM スナップショット検証。  
- E2E: `e2e/run.mjs` のユーザー招待シナリオを追加／更新し、ロール変更や MFA リセットのハッピーパスを Playwright で担保。  
- 最終確認として `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p timekeeper-frontend --lib`, `wasm-pack test --headless --firefox frontend/` を実行。

## 7. TODO チェックリスト

- [x] `frontend/src/pages/admin_users/` モジュール骨格（mod/layout/panel）を追加し、旧ファイルを段階的に空にする。
- [x] `repository.rs` にユーザー CRUD/MFA 連携 API を実装し、モックテストを作成。
- [x] `utils.rs` のフォーム／ロール／メッセージ状態をテスト駆動で実装。
- [x] `components/` に `InviteForm` / `UserList` / `UserDetailDrawer` を追加し、`panel` と疎結合にする。
- [x] `panel.rs` で `create_resource` / `create_action` を用いた状態遷移を整備し Pending/Error を UI へ反映。
- [x] Playwright / wasm_bindgen_test / cargo test を昇順で実行し、Documents へ実行ログを追記。

> この計画を完了することで、Admin Users ページは他ページと同じモジュールパターンを共有し、将来的な権限管理機能の追加や TDD サイクルに耐える構造へ移行できる。

## テストログ

- 2025-11-19: `cargo test -p timekeeper-frontend --lib`（成功）
- 2025-11-19: `wasm-pack test --headless --firefox frontend`（成功）

## UI 連携テストシナリオ（Playwright）

1. **ログイン & ページ遷移**  
   - `FRONTEND_BASE_URL` を設定し、システム管理者アカウントでログイン。  
   - ヘッダーの「ユーザー追加」リンクから `/admin/users` へ遷移し、`ユーザー招待 (管理者専用)` / `ユーザー一覧` が表示されることを確認。

2. **フォーム検証**  
   - すべて空欄のまま「ユーザーを作成」を押下し、`"すべての必須項目を入力してください。"` メッセージが出ることを確認。  
   - システム管理者権限を持たないユーザーでアクセスしようとした場合はガード文言が表示されることも確認。

3. **ユーザー招待フロー**  
   - ランダムな username/full_name/password を入力し、必要に応じてロールとシステム管理者チェックボックスをセット。  
   - 送信後、成功メッセージ表示とフォームリセットをassertし、一覧に新しいユーザー行が追加されるまで `await` で待機。  
   - API モックがない場合はテスト用ユーザーを後続で削除するクリーンアップを記述。

4. **詳細ドロワー & MFA リセット**  
   - 新規ユーザー行をクリックしてドロワーが開くことを確認し、氏名/ユーザー名/ロール/システム管理者/MFA ステータスが props と一致しているか検証。  
   - 「MFA をリセット」ボタンを押し、pending 中はボタンが disabled になること、API 成功後に成功メッセージが表示されることを確認（API 失敗ケースもモックで再現）。

5. **リロード & 状態保持**  
   - `invite_action` / リソースの再フェッチによりユーザー一覧がリロードされることを、`users_reload` カウンタに相当する UI 更新（新行表示）で検証。  
   - ドロワー閉鎖後にメッセージがクリアされること、ブラウザ更新後もシステム管理者であればフォーム/一覧が表示されることを確認。

6. **スクリプト化**  
   - 上記シナリオを `e2e/admin-users.spec.ts` などに分割（招待/バリデーション/MFA リセット）し、`node e2e/run.mjs --grep "Admin Users"` 相当で個別実行できるようタグを付与。  
   - CI では `pwsh -File .\scripts\frontend.ps1 start` → `node e2e/run.mjs` の順で起動し、完了ログを PR の Evidence セクションへ添付する。`e2e/admin-users.mjs` に上記フローを実装済みなので、`node e2e/admin-users.mjs` で単体実行できる。
