# 休日定義拡張 実装計画

## 1. 背景
- 現状の休日定義は `holidays` テーブル（単発指定日）に限定されており、定休曜日や個別例外を表現できない。
- 休日判定ロジックが各所に散在しており、勤怠 API では休日や打刻制御を考慮できていない。
- 全社での定休設定を先行導入しつつ、将来的な部署別・個人例外へ拡張しやすい構造が必要。

## 2. 要件整理
- **休日条件**: `NOT exception AND (weekly_holiday OR public_holiday)` を満たす日を休日扱い。
- **適用範囲**: 今回は全社共通の定休曜日。将来は部署別にも拡張可能な構造とする。
- **例外**: 個人単位で休日/稼働日を上書き可能。例外は最優先。
- **適用開始制約**: 通常は翌日以降のみ。システム管理者は制約無し。
- **タイムゾーン**: すべて `config.time_zone` 基準のローカル日付で処理。
- **勤怠連携**: 休日は原則打刻禁止。休日出勤は別途申請フローで扱う。

## 3. データモデル
| テーブル | 主なカラム | 説明 |
| --- | --- | --- |
| `weekly_holidays` | `id TEXT PK`, `weekday SMALLINT` (0=Mon), `starts_on DATE`, `ends_on DATE NULL`, `enforced_from DATE`, `enforced_to DATE NULL`, `created_by TEXT`, `created_at TIMESTAMPTZ` | 全社定休設定。`starts_on` は UI 制約、`enforced_from` は実際の適用開始日。`ends_on/enforced_to` で将来の無効化や切り替えに備える。 |
| `holiday_exceptions` | `id TEXT PK`, `user_id TEXT`, `date DATE`, `override BOOLEAN`, `reason TEXT NULL`, `created_at TIMESTAMPTZ`, `created_by TEXT` | 個人例外。`override=true` は強制休日、`false` は出勤扱い。`UNIQUE(user_id, date)` 制約。 |
| 既存 `holidays` | 変更無しだが `updated_at` の自動更新トリガーを検討 (任意)。 |

## 4. マイグレーション計画
1. `013_create_weekly_holidays.sql` を追加し上記テーブルを生成。`weekday` に CHECK 制約 (0-6) を付与。
2. `014_create_holiday_exceptions.sql` を追加し例外テーブルを生成。`holiday_exceptions_user_date_key` で重複禁止。
3. 既存レコード初期投入: 現行運用に合わせて `weekly_holidays` に土日を登録 (`starts_on = tomorrow`, `enforced_from = today`)。
4. 以降の ALTER 系は将来の部署別対応に備えて `department_id NULL` カラムをコメント付きで予約する案も検討可。

## 5. バックエンド実装
1. **ドメインサービス追加**: `backend/src/services/holiday_service.rs`（新規）に以下を実装。
   - `HolidayService::is_holiday(date_naive, user_id_opt)` -> `HolidayDecision { is_holiday, reason }`
   - 定休/祝日/例外をまとめて取得する SQL。月次キャッシュ用に `fetch_month(year, month)` も提供。
   - アプリ起動時に `Arc<HolidayService>` を `Extension` で注入し、`Config` と一緒に `State` に保持。
2. **API ルーティング** (`backend/src/main.rs`):
   - `GET /api/holidays/weekly` (admin) : 現在および未来の定休設定一覧。
   - `POST /api/holidays/weekly` (admin) : starts_on/ends_on/weekday をバリデーションして登録。`is_system_admin` 以外は `starts_on >= tomorrow` を enforce。
   - `GET /api/holidays/check?date=YYYY-MM-DD` : `HolidayService::is_holiday` を呼び結果を返却。
   - `GET /api/holidays/month?year=&month=` : 月次の祝日+定休+例外を返却（種別フラグ付き）。頻繁利用を想定し、サービス内で 1 ヶ月単位の結果をキャッシュ（`tokio::sync::RwLock<HashMap<MonthKey, CacheEntry>>`）する。
   - 将来の例外 CRUD (`/api/admin/holiday-exceptions`) も同サービスを利用して実装予定（今回 Optional）。
3. **勤怠ハンドラ修正** (`backend/src/handlers/attendance.rs`):
   - `clock_in` / `clock_out` の冒頭で `HolidayService::is_holiday` を呼び、休日かつ override で平日扱いが無い場合は `403` を返す。エラーメッセージに「残業申請を行ってください」を含める。
   - 休日での打刻許可を与えるケース（例: 例外 override=false）では理由をレスポンスへ含められるよう `AttendanceStatusResponse` に `holiday_reason: Option<String>` を追加する案を検討。
4. **Admin ハンドラ補強** (`backend/src/handlers/admin.rs`):
   - 定休 API 用の DTO (`CreateWeeklyHolidayPayload`) を追加し、`weekday`/`starts_on`/`ends_on` の検証処理を実装。
   - 例外追加 API の骨格を実装し、UI 未整備でも REST 経由で登録できるようにする（任意）。

## 6. フロントエンド対応
1. **設定 UI** (`frontend/src/pages/admin.rs`):
   - 「休日設定」タブに定休曜日一覧を表示するテーブルと追加フォームを追加。starts_on/ends_on は DatePicker、weekday は複数選択可 (週2日分追加時は複数 POST)。
   - システム管理者と通常管理者で開始日の最小値を出し分け (`use_auth().is_system_admin` を利用)。
2. **勤怠画面** (`frontend/src/pages/attendance.rs`):
   - 月次ロード時に `GET /api/holidays/month` を呼び、カレンダー上で休日を強調表示。休日は打刻ボタンを disabled にし、例外で稼働可の場合のみアラート表示。
3. **API クライアント** (`frontend/src/api/client.rs`):
   - 追加エンドポイントの型定義とフェッチ関数を実装（`getWeeklyHolidays`, `createWeeklyHoliday`, `checkHoliday`, `getMonthlyHolidays` 等）。

## 7. テスト計画
- **バックエンド**:
  - `holiday_service` ユニットテスト: 祝日・定休・例外の組合せ 6 ケースを網羅。
  - `clock_in`/`clock_out` で休日拒否／例外許可の統合テストを `axum::Router` + `sqlx::query!` で実行。
  - マイグレーション適用後の SQLx `offline` データチェック（`cargo sqlx prepare`）を更新。
- **フロントエンド**:
  - `admin` ページのフォームバリデーションの unit テスト（`wasm-bindgen-test`）。
  - 休日取得 API をモックしたコンポーネントテストで表示状態を確認。
- **E2E**:
  - Playwright シナリオ: 管理者が定休を追加 → 翌日がカレンダー上で休日表示 → 一般ユーザーが休日に打刻を試み禁止される、という流れを追加。

## 8. 移行・ロールアウト
1. マイグレーションを本番 DB に適用。
2. 土日など現行定休日を投入。`enforced_from` を現行日に合わせ、`starts_on` は翌営業日で運用制約を満たす。
3. holiday_service を有効化したバックエンドをデプロイ。
4. フロントの管理 UI を利用して以降の定休日を運用。
5. 個人例外を使う運用が始まり次第、API から登録しログを監査。

## 9. リスクとフォローアップ
- **同時登録競合**: `weekly_holidays` に重複曜日が入ると判定が冗長になるため、`WHERE` 条件付きのユニーク制約 (例: `UNIQUE (weekday, daterange(enforced_from, enforced_to))`) を検討。初期実装では API 側で重複検知し、将来は DB 制約を追加。
- **キャッシュの鮮度**: 月次キャッシュが長期間残ると定休日更新反映が遅れるため、更新 API 完了時に該当月のキャッシュを明示的に invalidation。
- **例外 UI 未整備**: 直後に必要になる場合を想定し、最小限の API クライアント + 管理画面ボタンを準備するか、当面は SQL 直更新の手順を runbook に記載する。

以上を 1 スプリント (バックエンド 2〜3 日 + フロント 2 日 + テスト/レビュー 1 日) を目安に実装する。

