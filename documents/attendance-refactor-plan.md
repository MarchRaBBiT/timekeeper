# Attendance Page Refactor Plan

`documents/pages-refactor-plan.md` と Login/MFA リファクタの方針を踏まえ、`frontend/src/pages/attendance.rs` をモジュラー構成へ再編する。

## 1. 目標

- 出退勤（出勤/退勤/休憩）の UI と API 呼び出しを切り離し、`panel` が状態制御に集中できるようにする。
- １日のサマリーや履歴リストなどの断片 UI を再利用可能なコンポーネントとして分割する。
- フォーム入力・バリデーション・タイムゾーン処理を `utils.rs` に集約し、ユニットテストで担保する。

## 2. ディレクトリ構成

```
frontend/src/pages/attendance/
    mod.rs          # ルート（エントリ＋ガード）
    panel.rs        # AttendancePanel（状態＋イベントのハブ）
    layout.rs       # Unauthorized/Frame コンポーネント
    components/
        summary.rs  # 当日の勤務ステータス/メトリクス表示
        form.rs     # 出勤/退勤/休憩トグル
        history.rs  # 最近の打刻・履歴テーブル
        alerts.rs   # 例外・休日チェックなど
    repository.rs   # attendance API ラッパー（fetch/submit/履歴）
    utils.rs        # フォーム state, payload 変換, TZ 処理, バリデーション
```

## 3. 実装ステップ

1. **モジュール作成**  
   - `mod.rs` を新設し、既存の `attendance.rs` 本体は `panel.rs` へ移動。`pub use panel::AttendancePage` でルーティングへ再公開。
   - レイアウト/コンポーネント/リポジトリ/ユーティリティの各ディレクトリを配置。

2. **レイアウトとコンポーネント分割**  
   - `layout.rs` に `AttendanceFrame`（ヘッダ＋ガード）と `UnauthorizedMessage` を実装。
   - `components/summary.rs` で当日の打刻状態（出勤中・休憩中など）＋ KPI を表示。
   - `components/form.rs` はボタン群（出勤/退勤/休憩開始/休憩終了）と確認ダイアログを受け取る UI に限定。
   - `components/history.rs` で最近の打刻履歴リスト、`alerts.rs` で休日警告やシステムメッセージを表示。

3. **状態管理・API 層**  
   - `panel.rs` で `create_resource` / `create_action` を活用。例：  
     - `fetch_today_status`（`create_resource`）  
     - `fetch_history(page)`（ページング対応 Resource）  
     - `clock_action`（出勤/退勤/休憩の共通 `Action<ClockEventPayload, Result<...>>`）  
   - Repository の API 例：`get_today_summary()`, `get_history(page)`, `submit_clock_event(payload)`, `check_holiday(date)`。
   - `ClockEventPayload` など API 向けの struct は `repository.rs` or `types.rs` へ定義。

4. **フォーム状態とユーティリティ**  
   - `utils.rs` に `AttendanceFormState` を定義し、`create_rw_signal` で panel が保持。`to_payload()` / `reset()` / `toggle_break()` などを実装。
   - タイムゾーン補正や所定労働時間の計算なども `utils.rs` へ集約。`#[cfg(test)]` で wasm_bindgen_test を用意。

5. **ガード / Auth 連携**  
   - `layout.rs` または `panel.rs` の冒頭で `state::auth::use_auth()` を利用。`loading` 中はスピナー、未認証は `UnauthorizedMessage` を表示。
   - すでに認証状態が `AuthProvider` で共有されている前提で実装する。

6. **テスト & ドキュメント**  
   - 主要ユーティリティ（フォーム state、タイムゾーン変換、payload バリデーション）に wasm_bindgen_test を追加。
   - `repository.rs` は `ApiClient` を注入可能な形にし、モックテストまたは e2e で検証。
   - 作業完了後に `documents/pages-refactor-plan.md` の Attendance セクションを更新。

## 4. 実装ロードマップ

1. `mod.rs` / `panel.rs` / `layout.rs` を作成してルート構造を確立。  
2. `components/` ディレクトリに `summary.rs` / `form.rs` / `history.rs` / `alerts.rs` を追加し、既存テンプレートから UI を移行。  
3. `repository.rs` / `utils.rs` を新設し、Attendance API とフォーム state を切り出す。  
4. `panel.rs` を `create_resource` / `create_action` ベースで再実装し、子コンポーネントへ props で `Resource` と `Action` の結果を配布。  
5. ユニットテスト（`utils.rs`）を整備し、`cargo fmt`, `cargo test -p timekeeper-frontend --lib` を実行。  
6. `documents/pages-refactor-plan.md` に進捗と残課題を反映。

## 6. 実行チェックリスト

- [x] `frontend/src/pages/attendance/mod.rs` を新設し、既存 `attendance.rs` を `panel.rs` へ移して `pub use panel::AttendancePage` で再公開する。
- [x] ベースラインとして現在のロジックを `panel.rs` へ移植し、レイアウト分割前でもビルドが通る状態を確保する。
- [x] `layout.rs` に `AttendanceFrame` / `UnauthorizedMessage` を実装し、`state::auth::use_auth()` で UI を出し分ける。
- [x] `components/summary.rs` / `components/form.rs` / `components/history.rs` / `components/alerts.rs` を追加し、既存 UI を切り出す。
- [x] `repository.rs` を追加し、`get_today_summary()`・`get_history(page)`・`submit_clock_event(payload)`・`check_holiday(date)` などの API ラッパーを移行する。
- [x] `utils.rs` を追加し、`AttendanceFormState`（`create_rw_signal` 管理）やタイムゾーン/ payload 変換ロジックを実装する。
- [x] `panel.rs` を `create_resource` / `create_action` ベースへ再実装し、`Resource` / `Action` の結果を子コンポーネントに props で渡す。
- [x] 各コンポーネントに `LoadingSpinner` / `ErrorMessage` を組み込み、pending/error の表示を統一する。
- [x] `utils.rs` に wasm_bindgen_test を追加し、フォーム state・TZ 変換・payload 生成をテストする。
- [x] `documents/pages-refactor-plan.md` の Attendance セクションを更新する。
- [x] `cargo fmt` を実行する。
- [x] `cargo test -p timekeeper-frontend --lib` を実行する。


## 5. 注意点

- `state::auth` のグローバル状態に依存した処理（例：ユーザーのタイムゾーン設定）は panel 内に隠蔽し、コンポーネントには純粋な props を渡す。  
- `create_action` を複数併用する場合でも、ボタン有効/無効は `pending()` の結果のみに依存させ、UI 側で追加の状態管理を持たない。  
- 休憩開始/終了などは `enum ClockEventKind` を導入し、フォームや repository で重複コードが出ないようにする。  
- `repository.rs` では `ApiClient` のインスタンス共有を意識し、連続リクエスト時の base_url 解決コストを抑える。  
- `components/history.rs` などで大規模なテーブルを扱う際は、`Resource` のローディング状態/エラー表示を共通の `LoadingSpinner` / `ErrorMessage` コンポーネントで行う。

---

この計画に沿って段階的にリファクタを進め、Login/MFA と同様の構造・テスト戦略へ寄せる。