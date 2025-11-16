# Backend `src/` Readability Refactor Plan

> ブランチ方針: この計画は `refactor/backend`（横串）、機能別サブブランチ（`refactor/backend-attendance` など）で順次適用する想定。

## 進捗チェックリスト
- [x] `refactor/backend-attendance`: 共通ヘルパー + clock/break ハンドラのDRY化
- [ ] `refactor/backend-requests`: Leave/Overtimeリクエストのrepository化
- [ ] `refactor/backend-admin`: 管理APIのモジュール分割とレスポンス統一
- [ ] `refactor/backend-holidays`: Holiday Service/handlerの再構成
- [x] `refactor/backend-models`: Enum変換/モデルimplの統合

## 1. フロントエンドレビュー結果（参考）
- `frontend/src/pages/admin.rs` の `AdminPage` は view を分割済ながらシグナル生成・API呼び出し・状態変更が同一スコープに集約しており、`load_list` や `on_reject` などの handler は何をパラメータ化しているのか把握しづらい（例: `load_list` の `Rc<dyn Fn()>` が `status`/`per_page` などを外部から直接参照）。
- `frontend/src/components/cards.rs` の `WeeklyHolidayCard` や `AdminHolidayListCard` は表示と設定ロジックを混在させており、MVVM/MVC で言う「ViewModel」が存在しないため view が肥大化。APIレスポンスの整形や `json` マクロを直接呼び出す箇所が多く、試験的な変更が増えると再利用性の高いパーツに変化が集中します。
- これらの問題を観察すると、バックエンドでも「処理と応答の分離」「共通処理の再利用」「単一責務の関数」により全体の可読性と保守性を高めるメリットが顕著ですので、以下の plan に反映します。

## 2. 背景と目的
- 現行 `backend/src/handlers/attendance.rs`（`clock_in`/`clock_out`/`break_*`/取得系）には **同様の SQL 文・エラーハンドリング・ステータス判定**が複数箇所に登場し、1 関数の処理が長くなって可読性・保守性が低下しています。
- 依頼通り「処理フローは変えず」「DRY」かつ「関数を一文で説明できる単位に分割」する方向で、若干の構造化と共通化を導入します。
- ドキュメントでリファクタ全体を見通せるようにし、ステップを分けて段階的に適用します。

## 2. 現状の課題（主に `backend/src/handlers/attendance.rs`）
| 問題 | 影響 | 該当例 |
|------|------|--------|
| SQL/エラーハンドリングの重複 | ハンドラの本文が長く、どこが違うのか瞬時に判断できない | `clock_in` vs `clock_out` の SELECT/UPDATE 文・`map_err(|_| Json(...)` |
| ロジック内で日時計算／holiday guard／DB更新／response組立までまとめて処理 | 目的が一目でわからず「何をする」／「何を返す」かが曖昧 | `clock_in`（`naive_local` -> `reject_if_holiday` -> `existing attendance` -> `response`） |
| ブレーク関連で「attendance取得・所有確認・errorレスポンス」が毎回コピー | 変更時に漏れが発生しやすい | `break_start`, `break_end` の所有チェック＋`StatusCode::BAD_REQUEST` |
| `Json(json!({...}))` + `StatusCode` の `map_err` が数十回出現 | 変更するとき `StatusCode` を間違えがち；構造化が難しい | ほぼどの `sqlx::query` でも同様のクロージャ |

## 3. リファクタ方針（MVC/MVVMの思想を参考にした DRY + リーダブル + 小関数）
### 3.1 共通ユーティリティの導入
- `backend/src/handlers/attendance_utils.rs`（新ファイル）を作成し、以下のようなヘルパーを定義:
  - `fn db_error_response(msg: &str) -> (StatusCode, Json<Value>)` など一貫した JSON エラー。
  - Attendance/Break の取得・更新を行う `fetch_attendance`, `save_attendance`, `fetch_active_break`, `insert_break_record`。
  - `fn require_attendance_owned(att: &Attendance, user_id: &str) -> Result<(), (StatusCode, Json<Value>)>` のような再利用可能な前提チェック。
  - `fn now_in_user_tz(config: &Config) -> (NaiveDate, NaiveDateTime, DateTime<Utc>)` で日時計算を1箇所にまとめる。

### 3.2 各ハンドラの分割
- `clock_in`/`clock_out`:  
  - 開始: `let now = now_in_user_tz(&config); reject_if_holiday(...);` をヘルパー化。
  - 中盤: attendance の「取得/作成/更新」を `attendance_utils::upsert_attendance_for_clock_in(...)` のような説明的な関数に切り出し、ハンドラ本文は `if attendance.clock_in_time.is_none() { ... }` だけにする。
  - 最終: `construct_attendance_response(attendance)` で `break_records` 取得と `Json` に変換する処理を1関数にまとめる。
- `break_start`/`break_end`:  
  - `let attendance = fetch_attendance(attendance_id)` + `ensure_owned(&attendance, &user.id)` + `ensure_clocked_in(&attendance)` をそれぞれ1関数にし、ハンドラの本文は「条件チェック → DB操作 → response」だけにする。
  - `insert_break_record` / `update_break_record` も `attendance_utils` 内の関数として再利用。
- `get_my_attendance`, `get_breaks_by_attendance`, `get_my_summary`, `export_my_attendance` など取得系は、各 SQL を丁寧なヘルパー（例 `fn query_attendance_range(pool, user_id, from, to)`）に移し、`sqlx::query!` 直書きを減らす。

### 3.3 ファイル分割と構造整理
- `attendance.rs` は「ルーティングレベル」の定義だけを残し、ロジックは `attendance_core.rs` や `attendance_validation.rs` のようなサブモジュールへ委譲。`pub mod attendance` として `handlers/attendance/mod.rs` に再構成も検討。
- `services/` 以下で見られる `HolidayService:is_holiday` への依存（`reject_if_holiday`）は変えず、呼び出しを新たなヘルパー経由で行うことで可読性を保つ。

## 4. ステップと優先順位
1. **共通ヘルパー制作**（1-2日）  
   - `attendance_utils.rs` + `attendance_response.rs` としてエラー/取得/構築ロジックをまとめる。既存の主な SQL をそのまま移し、`map_err` は共通化。
2. **`clock_in` + `clock_out` のリファクタ**（0.5日）  
   - 新ヘルパーを使って脳内フローを「holiday → fetch/insert → respond」に整形。
3. **`break_*` の切り出し**（0.5日）  
   - 所有チェック/状態チェック/DB操作を helper へ切り出し、ハンドラ本体を短く。
4. **取得系 (`get_my_attendance` など) の SQL を helper に移動**（1日）  
   - `sqlx::query_as!` を複数箇所で使っている箇所は `attendance_repo.rs` で再利用関数を定義。
5. **モジュール再編**（1日）  
   - `mod attendance { pub mod handlers; pub mod repo; }` などに分割し、1ファイル1責務の原則を意識。
6. **テスト／フォーマット**  
   - `cargo fmt`, `cargo t`. 変更は pure refactor のため既存 API テストが生きていれば問題なし。

## 5. リスクと防止策
- **意図しない振る舞い変更**: 新しい helper は既存 SQL 文字列をそのまま使うか、使い回しの `sqlx` ステートメントに差し替える。リファクタ前後の `cargo test --test attendance_api`（存在すれば）を必ず通す。
- **モジュール分割でパスがずれる**: `pub use` を活用し、`handlers::attendance::clock_in` など現行の `use` を変えずに済むように設計。
- **1関数への理解負荷**: 「1関数1文」ルールを守るため、例外処理（`reject_if_holiday`）や `map_err` も helper 化して本文は1行で読めるよう意識。

## 6. 次のアクション
1. ドキュメントをチームでレビューし、アプローチを承認。
2. 共通ヘルパーを作る PR → `clock_in` などを順に小分けで PR 連続投入。
3. `backend/src/handlers` 以下の他の large handler（`admin.rs` 等）も同様の方針で拡張可能。必要なら `handlers/README.md` に DRY ルールを記載。

## 7. その他 backend/src 横断レビューの要点
- **`handlers/requests.rs`**: SQL を各ハンドラで直書きしており、`LeaveRequest`/`OvertimeRequest` の作成・取得フローが重複。`map_err` で同じ JSON エラーを複数回組み立てているため、helper による共通化が必要。フォームバリデーション関数もハンドラと同居していて再利用しにくい。
- **`handlers/admin.rs`**: 900行超のファイルにユーザー一覧・申請承認・休日一覧・MFA・CSVなどが混在し、責務境界が曖昧。認可チェックやレスポンス shape も統一されておらず、モジュール分割（`admin::{requests, holidays, mfa}`）と共通レスポンス/エラーヘルパー導入が望ましい。
- **`services/holiday.rs`/`handlers/holidays.rs`**: 月次リスト生成 (`list_month`) が 1日ずつ `is_holiday` を呼び出すため非効率かつ読みにくい。祝日/週次/例外をまとめたモデルを services で統合し、handler では結果を返すだけに整理する。
- **`models/*`**: Enum ↔ str の変換 (`AttendanceStatus`, `UserRole` 等) が散在しており、`handlers` 側で文字列を手打ちしている箇所が多い。Model impl に DB 変換/状態遷移をまとめ、repository/helper から再利用する。

## 8. 横断リファクタのブランチ案と検証
`refactor/backend-attendance` での進め方にならい、他モジュールでも以下のように小さく枝を分ける。

| ブランチ | 対象 | 主な作業 | 推奨テスト |
|---------|------|----------|-----------|
| `refactor/backend-requests` | `handlers/requests.rs`, `models/leave_request.rs`, `models/overtime_request.rs` | Repository パターン導入、エラーヘルパー共有、モデル活用 | `cargo test -p timekeeper-backend --test requests_api` |
| `refactor/backend-admin` | `handlers/admin.rs`, `handlers/mod.rs` | モジュール分割、認可/レスポンス共通化、SQL repo 化 | `cargo test -p timekeeper-backend --test admin_holiday_list` |
| `refactor/backend-holidays` | `services/holiday.rs`, `handlers/holidays.rs` | `HolidayService` の再構成、祝日結合クエリの repo 化 | `cargo test -p timekeeper-backend --test holiday_service` |
| `refactor/backend-models` | `models/*` | Enum 変換ヘルパー整理、モデル impl に DB 補助を集約 | 各モデルのユニットテスト (`cargo test -p timekeeper-backend`) |

各ブランチで `cargo fmt --all` → `cargo test -p timekeeper-backend` を実行し、`backend/tests` の CI が通ることを確認する。

## 9. 次のアクション（横断）
1. 本ドキュメントをチームで共有し、優先度を決定して各 refactor ブランチを順次切る。  
2. ブランチごとに小さな plan（目的、手順、テスト）を `documents/` 配下に追加し、`refactor/backend-attendance` と同様の粒度で進行ログを残す。  
3. 各 PR で repository/helper が追加されたら、API ハンドラが細くなったことをレビュー観点に含める。  
4. backend の整理後は frontend 側（特に `AdminPage`）の ViewModel 化と合わせて e2e (`backend/tests`, Playwright) を回し、リグレッションを防止する。
