# AdminPage モデル層リファクタリング計画

## 背景
- `frontend/src/pages/admin.rs` の `AdminPage` は ~900 行以上あり、「申請一覧」「手動打刻」「MFA リセット」「祝日管理」「Google 祝日インポート」「週次祝日」「管理用祝日一覧」「モーダル対応」を単一関数で完結させている。
- 各カード/モーダルに必要なシグナル生成・ API 呼び出し・状態更新が同じスコープにあり、責務の分離が弱く副作用の把握やテストが困難。
- 既に view はカードコンポーネントへ切り出されているため、それぞれに対応する model 層を分離し、見通しと再利用性を改善したい。

## ゴール
1. AdminPage の model ロジックを機能別の `use_*_model` フック (または小さなコンテキスト) に切り出し、単一関数の肥大化を解消する。
2. 各モデルが返す値/ハンドラを構造体で明示し型境界を強化、API 呼び出しや副作用の責務を局所化する。
3. 認可状態 (`admin_allowed`/`system_admin_allowed`) の判定を共通ヘルパーにまとめ、各機能で重複させない。
4. 最終的に `AdminPage` は view レイヤーの構成と hook 呼び出しのみを担い、状態の共有は専用モデル経由になる。

## ノンゴール
- view コンポーネント (`AdminRequestCard` など) のレイアウト変更。
- API インターフェースそのものの変更。
- 既存のテスト拡充 (最小限の退行監視として既存 wasm テストを継続使用)。

## 提案アーキテクチャ

### ディレクトリ構成案
```
frontend/
  src/
    pages/
      admin/
        mod.rs           # 既存 AdminPage のエントリ。view を中心に保持。
        access.rs        # 認可判定ヘルパー。
        requests.rs      # 申請一覧 + モーダルモデル。
        attendance.rs    # 手動勤怠 + 強制休憩終了。
        mfa.rs           # MFA リセット機能モデル。
        public_holidays.rs   # 単発祝日 + Google インポート。
        weekly_holidays.rs   # 週次祝日。
        admin_holidays.rs    # 管理用祝日一覧。
```
- `frontend/src/pages/admin.rs` は `mod.rs` として module ルートになり、上記サブモジュールを `pub use` する。
- 既存小さなヘルパー (`parse_dt_local`, `next_allowed_weekly_start`, `weekday_label`) は `mod.rs` または適切なサブモジュールへ移す。

### モデル API イメージ
各サブモジュールに「モデル構造体 + 初期化関数」を置く。例:

```rust
pub struct AdminRequestsModel {
    pub status: RwSignal<String>,
    pub user_id: RwSignal<String>,
    pub list: RwSignal<Value>,
    pub load_list: Rc<dyn Fn()>,
    pub open_modal: Rc<dyn Fn(RequestListItem)>,
    pub modal: RequestModalModel,
}

pub fn use_admin_requests(access: &AdminAccess) -> AdminRequestsModel { ... }
```

- `ManualAttendanceModel` には `att_user`, `att_date`, `breaks`, `add_break`, `on_submit_att`, `force_end_break` を閉じ込める。
- `MfaResetModel` は `Resource<Vec<UserResponse>>` + `selected_mfa_user` + `reset` ハンドラを持つ。
- `HolidayManagementModel` は単発祝日作成、Google 祝日取得/インポート、既存祝日リストの状態をまとめる。内部で `refresh_holidays` などの関数を `Rc` で返却。
- `WeeklyHolidayModel` は `weekly_weekday_input` などのシグナルと `refresh_weekly_holidays`、`create_weekly_holiday` を包含し、`next_allowed_weekly_start` を利用。
- `AdminHolidayListModel` はページング条件シグナルと `fetch_admin_holidays` を保持。
- `RequestDetailModalModel` は `show_modal`、`modal_data`、`modal_comment`、`approve`/`reject` ハンドラをまとめる。承認/却下時に `AdminRequestsModel::load_list` を呼べるようクロージャを受け取る。

### 認可ハンドリング
- `use_auth()` の結果を `AdminAccess` 構造体 (例: `is_admin`, `is_system_admin`, `user_id`) にまとめ、`Rc<AdminAccess>` を各モデルに渡す。
- 認可が必要なモデル (`ManualAttendanceModel`, `MfaResetModel`) は `Option<...>` を返すか `enabled` フラグを expose し、`AdminPage` 側で `<Show when=...>` を維持。

### 副作用の整理
- すべての API 呼び出しは対応モジュール内で `spawn_local` を行い、`ApiClient` 生成もそこで閉じる。
- `store_value` によるクロージャ固定はモデル内で完結させ、view には `Rc<dyn Fn>` を渡す (store_value を `AdminPage` から排除)。
- `create_effect` を利用した初期ロード (`refresh_holidays`, `refresh_weekly_holidays`, `load_list`) も各モジュール内で `on_mount` 相当の関数として実装し、`AdminPage` では `let requests = use_admin_requests(...);` の呼び出しだけにする。

## 実施ステップ
1. **モジュール雛形を作成**  
   - `frontend/src/pages/admin/mod.rs` を作り既存コードを一旦そのまま移す。ビルドを通すことを優先し `.rs` -> ディレクトリ化のみ実施。
2. **共通認可 & ヘルパー整備**  
   - `access.rs` を追加し、`AdminAccess::new(auth_signal)` を提供。`next_allowed_weekly_start` 等も移設し公開。
3. **リクエスト一覧/モーダル移行**  
   - `AdminRequestsModel` と `RequestModalModel` を導入し、`AdminPage` からリスト/モーダル関連シグナルを除去。`AdminRequestCard` と `RequestDetailModal` はモデルのフィールドを受け取るだけに変更。
4. **手動勤怠 & 休憩強制終了**  
   - `ManualAttendanceModel` を追加し、`ManualAttendanceCard` への props を `ManualAttendanceProps { model: ManualAttendanceModel }` のようにまとめる。既存 API 呼び出しを移管。
5. **MFA リセット & ユーザー一覧**  
   - `use_mfa_reset_model` を実装。ユーザー resource の取得、選択状態、リセット処理をモジュールに閉じ込める。
6. **祝日管理 (単発 + Google)**  
   - `HolidayManagementModel` を導入。祝日一覧の初期ロードと Google 関連処理、`on_create_holiday`/`import_google_holidays` を分離。
7. **週次祝日**  
   - `WeeklyHolidayModel` に `refresh_weekly_holidays`, `create_weekly_holiday`, 入力シグナルを保持させる。`WeeklyHolidayCard` はモデル props 化。
8. **管理用祝日一覧**  
   - `AdminHolidayListModel` としてページングパラメータを集約。
9. **`AdminPage` の整理**  
   - すべてのモデルインスタンスを作成し、view で props として渡すだけの形に仕上げる。`store_value` や `Rc::new` の乱立を解消。
10. **フォーマット & 検証**  
    - `cargo fmt --all`, `wasm-pack test --headless --firefox frontend` (可能なら) を実施し、diff を確認。

## テスト & 検証方針
- `frontend/src/pages/admin.rs` のユニットテスト (既存 `next_allowed_weekly_start`, `weekday_label`) は移設後も `wasm_bindgen_test` で維持。場所移動時は `mod tests` を該当サブモジュールに保持。
- 主要副作用の E2E は Playwright の `run.mjs` でカバーされているため、リファクタ直後に最低限 `wasm-pack test --headless --firefox` を走らせ、API コールのリグレッションは `pwsh -File scripts/test_backend.ps1` (可能なら) で補完。
- モデル切り出しによって追加される純粋関数があれば、`#[cfg(test)]` で簡易ユニットテストを同ファイルに設置しやすくなる。

## リスクと緩和策
- **カード間の依存 (`load_list` を承認/却下後に再利用)**: モデル間でコールバックが必要な箇所は `trait AdminRequestActions` のような薄いインターフェースを導入して循環依存を避ける。
- **大量の props 変更による diff 拡大**: 各カードコンポーネントに `Props` 構造体を導入し、1 ファイルずつ移行することで reviewer 負荷を下げる。
- **`store_value` 廃止後のクロージャ lifetime 問題**: 各モデルで `Rc<dyn Fn>` を保持して view へ渡す形にすれば、`store_value` と等価の lifetime を確保できる。初期段階で `#[derive(Clone)]` 可能なモデル構造体にまとめる。
- **動作確認コスト**: フェーズ毎に `wasm-pack test` を回し、必要であれば `pnpm start` などを併用して画面確認する。段階的 PR 提出で差分を最小化する。

## まとめ
- モデル切り出しは「ディレクトリ化 → 認可/共通ヘルパー → 各機能モデル → props 置き換え → 最終整理」の順で進める。
- それぞれの機能が自己完結した `use_*_model` を持つことで、将来的に別ページ/カードからもロジックを再利用でき、API 変更時の影響範囲を局所化できる。
