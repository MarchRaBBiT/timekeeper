# Admin Dashboard Page Refactor Plan

`documents/pages-refactor-plan.md` の Admin Dashboard セクションおよび Login/MFA・Admin Users の計画書を踏まえ、`frontend/src/pages/admin/` 以下のダッシュボード画面をモジュール構成に再編するロードマップをまとめる。

## 1. 目的

- `admin/panel.rs` に集約されている WeeklyHoliday/Requests/Attendance/SystemTools/Holiday ロジックをセクション単位のモジュールへ分離し、`repository.rs` / `utils.rs` / `components/` の共通パターンに揃える。
- `create_resource` / `create_action` を全セクションに導入して pending/error UI を統一し、State/Effect の散在を解消する。
- System Admin/General Admin の権限制御をレイアウト層に閉じ込め、セクション側は props で受け取るだけのシンプルな interface にする。
- TDD 方針に従い、状態ユーティリティや repository 層のテストを先に書き、Playwright シナリオで回帰を担保する。

## 2. 現状の課題 (`frontend/src/pages/admin/panel.rs`)

1. `ApiClient::new()` / `spawn_local` がファイル全体で乱立し、API 呼び出し／UI 表示／状態更新が複雑に絡み合っている。
2. WeeklyHoliday などのフォーム検証・日付レンジ計算が繰り返し記述されており、ユーティリティ化されていない。
3. pending/error の表示がセクション毎に異なり、`LoadingSpinner`/`ErrorMessage` を統一的に使えていない。
4. Admin Requests / System Tools / Holiday などのセクションが 1 つの `panel.rs` にベタ書きされており、差分レビューが困難。
5. `create_resource` / `create_action` を使ったリフレッシュ制御が不足しており、リロード or `spawn_local` 再実行のタイミングが明示されていない。

## 3. 目標構成

```
frontend/src/pages/admin/
    mod.rs
    layout.rs              # AdminDashboardFrame / UnauthorizedMessage
    panel.rs               # 権限判定＋セクション配置のみ
    repository.rs          # Admin API (holidays/weekly/mfa/etc.) を一括管理
    utils.rs               # フォーム状態・バリデーション・権限 helper
    components/
        weekly_holidays.rs
        requests.rs
        attendance.rs
        system_tools.rs
        holidays.rs
```

- `repository.rs`: 既存の `AdminUsersRepository` と同様に `AdminRepository` を用意し、`list_holidays` / `create_holiday` / `delete_holiday` / `list_weekly` / `create_weekly` / `fetch_google_holidays` / `list_requests` / `reset_mfa` などをメソッド化。
- `utils.rs`: `WeeklyHolidayFormState`, `HolidayFormState`, `SystemToolState`, `PermissionGuard` を定義し、`#[cfg(test)]` で TDD。
- `components/*.rs`: 表示責務のみを持ち、Action/Resource の `pending()` や `error` を props 経由で受け取る。
- `panel.rs`: memo 化した権限 (`admin_allowed`, `system_admin_allowed`) をセクションへ渡すだけに留める。

## 4. リファクタリングステップ

1. **モジュール骨格（mod/layout/panel）整備**  
   - `admin/layout.rs` に `AdminDashboardFrame`/`UnauthorizedMessage` を再配置し、`panel.rs` から UI 部分を切り出す。  
   - `panel.rs` は認可チェックとセクション配置のみを担い、セクションは `components/` 配下へ移動。

2. **Repository/Utils 抽出**  
   - `repository.rs` を新設し、既存 `panel.rs` 内の `ApiClient::new()` 呼び出しを `AdminRepository` メソッドへ移動。  
   - `utils.rs` に WeeklyHoliday/Requests/SystemTools/Holiday のフォーム状態 struct とバリデーションを実装し、`wasm_bindgen_test` でカバー。

3. **セクション別コンポーネント化**  
   - `components/weekly_holidays.rs`: `create_resource` + `create_action` で List/Create/Delete を制御。  
   - `components/requests.rs`: フィルタ／リスト／詳細を Leptos コンポーネントへ分割。  
   - `components/attendance.rs`: 一括操作（打刻修正・勤怠リセット）のフォームを整理。  
   - `components/system_tools.rs`: MFA リセットや Export など管理者ツールをカードとして表示。  
   - `components/holidays.rs`: 祝日管理（一覧＋Google import）を repository と連携。

4. **Action/Resource wiring**  
   - 各セクションで `create_resource` を活用してデータ取得（`(allowed, reload_counter)` を key に）を統一。  
   - `create_action` 成功時に `MessageState` を更新し、`reload` シグナルをインクリメントして一覧を再取得。  
   - pending/error は `components::layout::{LoadingSpinner, ErrorMessage, SuccessMessage}` を共通使用。

5. **TDD / UI テスト追記**  
   - `utils.rs` の状態遷移テスト、`repository.rs` の API 呼び出しモックテストを追加。  
   - `documents/pages-refactor-plan.md` と本計画書にテスト結果を記録。  
   - Playwright (`e2e/run.mjs` 系) に Admin Dashboard 主要フロー（WeeklyHoliday CRUD、Holiday Import、MFA Reset）を追加。

## 5. TODO チェックリスト

- [x] `frontend/src/pages/admin/` に `layout.rs` を追加し、`panel.rs` をセクション配置専用に書き換える。
- [x] `repository.rs` に Admin API 呼び出しを集約し、`utils.rs` でフォーム状態・権限判定を実装（テスト付き）。
- [x] `components/weekly_holidays.rs` / `requests.rs` / `attendance.rs` / `system_tools.rs` / `holidays.rs` を作成し、UI を分割。
- [x] 各セクションで `create_resource` / `create_action` を導入し pending/error UI を共通化。
- [x] `documents/pages-refactor-plan.md` と本計画書を最新化し、`cargo test` / `wasm-pack test` / Playwright シナリオ結果を記録。

- 2025-11-19: `AdminRepository` を導入し、WeeklyHoliday/Requests/Holidays/Attendance/SystemTools すべてのセクションが構造体経由で API を利用するように更新。これによりテスト時の `ApiClient` 差し替えが容易になった。

## テストログ

- 2025-11-19: `cargo test -p timekeeper-frontend --lib`
- 2025-11-19: `wasm-pack test --headless --firefox frontend`
- 2025-11-19: `node e2e/admin-dashboard.mjs`

> この計画に沿って `admin/` を再構成することで、Admin Users など他ページとのモジュールパターンが揃い、エンタープライズ管理フローの追加にも耐えられる可読性・テスト容易性を確保できる。
