# Dashboard Page Refactor Plan

`documents/pages-refactor-plan.md` で示された整理方針を起点に、`frontend/src/pages/dashboard` を他ページのリファクタリング計画書（Requests, Admin Dashboard, Attendance）と併せて再設計します。Dashboard は複数の KPI/panel をまとめるハブなので、`requests` のような `layout/panel/repository/utils` の責務分割、`admin-dashboard` の Resource/Action + コンポーネント分割、`attendance` のような shared component（例：summary・alerts）を組み合わせて共通化の基盤とします。

## 1. 参考とする他ページの構成
- `documents/requests-refactor-plan.md`：`components/filter.rs` や `utils.rs` による信号分離、`panel.rs` での `create_resource` + `create_action` の取り回し、`repository/types` の役割分解がシンプルなので同じ “panel に API と状態を集約” 方式をダッシュボード上でも採用する。
- `documents/admin-dashboard-refactor-plan.md`：複数コンポーネント（weekly_holidays/requests/attendance/system_tools/holidays）を `panel` 経由で組み合わせ、`repository` と `utils` で API/フォーム状態を切り出す手法を、Dashboard の summary/alert/activity パネル群に流用。
- `documents/attendance-refactor-plan.md`：`components/summary.rs` や `components/alerts.rs` のような再利用可能な UI ブロックを取り込み、`panel` の `Resource` 状態から共通 `LoadingSpinner/ErrorMessage` レイアウトに差し替える段取りを参考にする。
- `documents/pages-refactor-plan.md`：Dashboard セクションで指摘されている KPI 用 `utils`、`panel` 内での `Memo`, `create_rw_signal` の扱いを再確認。

## 2. Dashboard のファイル構成（想定）
```
frontend/src/pages/dashboard/
    mod.rs             # pub use panel::DashboardPage
    layout.rs          # UnauthorizedMessage + DashboardFrame scaffold
    panel.rs           # Resource/Action を束ねるメインレイヤー
    repository.rs      # summary/alerts/activities/announcements API
    utils.rs           # KPI 計算, memoized helper, formatting utils
    components/
        summary.rs     # KPI cards reused across admin/attendance
        alerts.rs      # 状態別で使い回す通知カード
        activities.rs  # recent activities list shared with requests log
        global_filters.rs # Requests の `filter` と似た抽象化
```

## 3. 実装ステップ
1. **レイヤー分解**（`documents/requests-refactor-plan.md` に倣い）
   - `mod.rs` から `panel.rs` を公開し、`layout.rs` で DashboardFrame を構成して認証状態や UnauthorizedMessage を統一。
   - `panel.rs` では `create_resource`（summary/alerts/activities）と、必要なら `create_action`（通知の dismiss/reload）を `Resource`/`Action` で分けて pend/error を明示化。
2. **API と utils**（`documents/admin-dashboard-refactor-plan.md` の repository/utils 分離）
   - `repository.rs` で `fetch_summary`, `fetch_alerts`, `fetch_recent_activities`, `reload_announcements` を用意し、それぞれ `ApiClient` で `spawn_local`。
   - `utils.rs` には `KpiFormatter`, `memo_kpi`, `dashboardFilterState`（`requests` の filter を参考）を収め、`panel.rs` で `create_rw_signal` を使う。
3. **再利用コンポーネント**（`documents/attendance-refactor-plan.md` 形式）
   - `components/summary.rs`・`alerts.rs`・`activities.rs` を `summary` / `alert` / `activity` それぞれの `Resource` props で制御し、`LoadingSpinner`/`ErrorMessage` を使い回す。
   - `components/global_filters.rs` を作ることで Requests の filter 仕様と統一された UI を提供し、必要なら他ページでも再利用。
4. **共通スタイルとフィードバック**（`documents/pages-refactor-plan.md` に基づく）
   - `panel` から `Memo`/`create_rw_signal` を通じて KPI キャッシュを持たせ、reload で `Resource::refetch()` を使う。
   - `utils.rs` では `#[cfg(test)]` を使った `wasm_bindgen_test` を書き、requests/attendance でも共有できる helper をカバレッジ。

## 4. 実装オプション（推奨順、各 pros/cons）
1. **パネルごとに Resource/Action を保持する方式**（推奨）  
   - Pros: `LoadingSpinner`/`ErrorMessage` を各コンポーネントで使い回せ、`Resource::refetch()` で個別リロードが可能。`documents/requests-refactor-plan.md` と同じ構成なので理解とテストが容易。  
   - Cons: API 呼び出し数が増えるため、需要に応じて `batch fetch` を検討する必要あり。
2. **まとめて DashboardState を持ち一括フェッチする方式**  
   - Pros: ネットワーク負荷を抑え、一度のロードで全パネルを埋められる。  
   - Cons: 個別パネルの pending/error 表示や再読込が `panel` レベルで扱いにくくなり、`ErrorMessage` の再利用性が下がる。
3. **Requests/Attendance と共通の FilterState を単一コンポーネントに抽象化し、他ページからも import する方式**  
   - Pros: `global_filters.rs` で UI と state を共有することで UX に一貫性が出る。  
   - Cons: 依存が強いため、Dashboard 側の変更が他ページに波及しやすく、別々のチームがあると調整コストが上がる。

## 5. 実装方針
`documents/dashboard-refactor-plan.md` では上記オプション1（パネルごとに独立した `Resource`/`Action`）を採用し、各コンポーネントが `LoadingSpinner`/`ErrorMessage` や `Resource::refetch()` を直接扱う構造で進めます。`panel.rs` で summary/alerts/activities を分離し、必要なら `create_action` でリロード・dismiss を扱います。

## 6. 現在進捗チェックリスト
- [x] `frontend/src/pages/dashboard/mod.rs` → `panel.rs` を `pub use` し、`layout.rs` で `DashboardFrame` と `UnauthorizedMessage` を整備
- [x] `panel.rs` に summary/alerts/activities 用 `Resource` を構築し、個別 `create_action`（リロード/ dismiss）を `LoadingSpinner`/`ErrorMessage` で制御
- [x] `repository.rs` で `fetch_summary`/`fetch_alerts`/`fetch_recent_activities`/`reload_announcements` を実装し、`ApiClient` + `spawn_local` で API 呼び出し
- [x] `utils.rs` に KPI memo や `dashboardFilterState` を定義し、`panel.rs` で `create_rw_signal` を用いた状態管理を追加
- [x] `components/summary.rs`・`alerts.rs`・`activities.rs`・`global_filters.rs` を作成し、それぞれ `Resource` props で `LoadingSpinner`/`ErrorMessage` を再利用
- [x] 上記実装に対して `cargo fmt --all` / `cargo test -p timekeeper-frontend --lib` / `wasm-pack test --headless --firefox frontend` / `node e2e/run.mjs` を実行

## 7. 検証
- `cargo fmt --all`
- `cargo test -p timekeeper-frontend --lib`（ダッシュボード関連のユニット/内部テスト確認）
- `wasm-pack test --headless --firefox frontend`（UI ロジックの wasm_bindgen_test を含む）
- `node e2e/run.mjs`（Playwright で Dashboard の smoke check、`FRONTEND_BASE_URL` 設定）
- `documents/pages-refactor-plan.md` と合わせてテスト実行ログを記録
