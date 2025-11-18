# Attendance Page Refactor Plan

`frontend/src/pages/admin` リファクタリングの構成を踏まえて、`frontend/src/pages/attendance.rs` を以下の方針で整理する。

## 1. ファイル構成

- `frontend/src/pages/attendance/mod.rs`
  - 認可ガードとエントリ (`AttendancePage`) のみを保持。
  - `pub mod layout; pub mod components; pub mod panel; pub mod repository;` などを再エクスポート。
- `layout.rs`
  - `UnauthorizedMessage`・`AttendanceDashboardFrame` 等、共通レイアウトコンポーネント。
- `panel.rs`
  - `AttendancePanel` を定義し、他モジュールのセクションを組み合わせるだけの薄い層にする。
- `components/`
  - `summary.rs`（勤怠サマリ）、`form.rs`（打刻フォーム）、`history.rs`（履歴テーブル）など、UI パーツを用途別に分割。
- `repository.rs`
  - `ApiClient` 呼び出しをまとめ、`create_resource` が扱いやすい async 関数を提供。
- `utils.rs`
  - 日付計算・入力バリデーション等の純粋関数と単体テストを配置。

## 2. 状態管理の方針

- 取得系データ（当日打刻、履歴一覧、ステータス等）は `create_resource`＋`ResourceHandle` に統一し、`panel` から各セクションに渡す。
- フォーム入力は `struct AttendanceFormState` を `create_rw_signal` で保持し、`to_payload()/reset()` といったメソッドを `impl` にまとめる。
- エラー/成功メッセージは `create_rw_signal<Option<String>>` で一元管理し、`layout` のメッセージコンポーネントを再利用する。

## 3. UI/ロジック分離

- 各セクションコンポーネントは props として `Resource<T>` や `Action<T, E>` を受け取り、ビュー描画のみを担当。
- `repository.rs` で `fetch_today_summary()`, `fetch_history(page)`、`submit_clock_event(payload)` などを定義し、`panel` から呼び出す。
- フォームのバリデーションや time zone 変換は `utils.rs` の純関数に切り出し、`#[cfg(test)]` でカバー。

## 4. テスト

- `utils.rs` に `#[cfg(test)]` を設け、日付計算/バリデーションのケースを `wasm_bindgen_test` で網羅。
- `repository.rs` は将来のモック容易性を意識し、引数に `ApiClient` を受け取る構造体/トrait化を検討。

## 5. 作業手順サマリ

1. `frontend/src/pages/attendance/mod.rs` とフォルダ構成を新設し、既存 `attendance.rs` を `panel.rs` に移動。
2. `layout.rs` と `components/` を作成し、HTML テンプレートを分割。
3. `repository.rs` と `utils.rs` を実装して API/ロジックを切り出し、`panel` を薄く保つ。
4. 既存コードを段階的に移し替えつつ `cargo fmt`, `cargo test -p timekeeper-frontend --lib` で回帰確認。
