# Backend `src/` 横断レビューとリファクタリング案

## 1. 全体観察
- **`handlers/requests.rs`**  
  - API ハンドラ側で `sqlx::query` を直書きしており、`LeaveRequest`/`OvertimeRequest` 作成・取得フローが 4 回以上コピーされています。  
  - `map_err` で `Json(json!({"error": ...}))` を生成する処理が複数箇所に登場し、可読性低下＆ future ミスの温床。  
  - 入力バリデーション (`is_valid_leave_window`, `is_valid_planned_hours`) はハンドラと同ファイルにあり、他の API から使いづらい。
- **`handlers/admin.rs`**  
  - 900+ 行のファイルに「ユーザー一覧」「申請承認」「休日一覧」「MFA」「CSV」などが混在し、`serde_json::Value` の操作やステータス計算が分散。  
  - `Rc` などの構造はなく同期的な SQL + Json 設計のため、責務ごとの境界があいまい。  
  - 認可チェック (`user.is_admin`) は繰り返され、レスポンス shape も統一されていない。
- **`services/holiday.rs` / `handlers/holidays.rs`**  
  - `HolidayService::is_holiday` → `list_month` のループで `is_holiday` を毎日クエリしているため効率も可読性も低い。  
  - UI から必要とされる「祝日/週次/例外」をまとめたモデルは `admin` 側に直書きされているため、`services` で共有できない。
- **`models` パッケージ**  
  - `LeaveRequest`/`OvertimeRequest` の `impl` に状態遷移や `serde` 表現が整っているが、SQL 側で文字列を手動で指定しているため全く DRY になっていない。  
  - `models::user` の `UserRole` や `AttendanceStatus` など enum → str 変換の helper が点在し、`handlers::attendance_utils` 以外では共通化されていない。

## 2. 可読性・DRY を担保するリファクタ案
1. **問い合わせを repository （`handlers/*.repo.rs`）に切り出す**  
   - `requests`/`admin` のそれぞれに `repo` module（`requests_repo.rs` 等）を作り、SQL を一箇所で保持。`LeaveRequestRepo::create`, `RequestRepo::list_by_user`, `AdminHolidayRepo::list_combined` など軽量な関数を導入して、ハンドラでは結果の集約とレスポンス構築に集中できるようにする。SQL の文字列・`map_err` も Repo 内で共通化し、カスタム `DbError` で表現してハンドラは `?.map_err(|e| e.into())` するだけに。
2. **共通エラー/レスポンスビルダーを再利用**  
   - `attendance_utils` で成功したエラーハンドリングの分離ができているので、同じ方針で `requests` や `admin` ハンドラにも `crate::handlers::error.rs` のような共通ユーティリティを作り、`StatusCode` + `Json(json!({ "error": ... }))` の定型文を減らす。
3. **モデルの状態遷移を reuse**  
   - `LeaveRequest::new` などで作成したオブジェクトを `repo` で受け渡すのではなく、`LeaveRequest::insert(&pool)` のようにメソッド化して `handlers` から `sqlx::query_as!` を呼ばせない工夫も検討。`models::attendance::Attendance` にある `status_to_str` などの `impl` を `repo` 側に移せば、SQL を param binding する共通関数が `models` に一塊で残せる。
4. **管理機能の handler をモジュール分割**  
   - `handlers/admin.rs` は `pub mod` でさらに `mod requests`, `mod holidays`, `mod mfa` と分割し、各モジュールが `AdminRequestHandler`, `HolidayHandler` を expose する形に整理する。API ルートも `use crate::handlers::admin::requests::*` のように整理すると、ファイルの長さを制御できる。
5. **サービス層の再利用性強化**  
   - `HolidayService` の `list_month` は現在ループで `is_holiday` を逐次呼ぶが、`models::holiday::HolidayCalendarEntry` を `impl From<HolidayDecision>` などで写すと `services` から `handlers` へのレスポンス構築が簡潔になる。`WeeklyHoliday`/`HolidayException` など SQL による合成も `repo` に移すことで Handler は `service.get_monthly_calendar(user)` のように使える。

## 3. ブランチ分割 & 検証ステップ
以下のようにバックエンド用 refactor ブランチを刻むと、`refactor/backend-attendance` と同じ進め方でレビューしやすい。

| ブランチ案 | 対象 | 主な作業 | テスト |
|------------|------|----------|--------|
| `refactor/backend-requests` | `handlers/requests.rs`, `models/leave_request.rs`, `models/overtime_request.rs` | Repo モジュール化・エラー helper 導入 | `cargo test -p timekeeper-backend --test requests_api` |
| `refactor/backend-admin` | `handlers/admin.rs`, `handlers/mod.rs` | `mod admin::{requests,holidays,mfa}` 分割、共通レスポンス | `cargo test -p timekeeper-backend --test admin_holiday_list` 他 |
| `refactor/backend-holidays` | `services/holiday.rs`, `handlers/holidays.rs` | `HolidayService` の再構築、結合クエリの repo 化 | `cargo test -p timekeeper-backend --test holiday_service` |
| `refactor/backend-models` | `models/*` | Enum ↔ str helper の集中、`impl` で DB 変換を提供 | モデル単体テスト (`cargo test -p timekeeper-backend models::...`) |

各ブランチで `cargo fmt --all` → `cargo test -p timekeeper-backend` を実行し、`backend/tests` の CI が通ることを確認。

## 4. 次のアクション
1. 本ドキュメントを共有して優先 LP を決定 (`requests` → `admin` → `holidays` の順など)。  
2. ブランチごとに `plan.md` を作成し、`refactor/backend-attendance` と同様に作業ログ/テスト結果を残す。  
3. 各 PR で repository/helper の追加が完了したら、API レイヤーが slender になったことを確認するレビュー観点をチームに共有。  
4. コード整理後、`frontend` 側の `AdminPage` モデル移行と合わせて e2e (`backend/tests`, Playwright) を回し、リグレッションを防止する。
