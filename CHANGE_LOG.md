# Change Log

## 2025-11-15

- **feat:** 休日機能の基盤を刷新。`weekly_holidays`/`holiday_exceptions` を追加し、`HolidayService` で祝日・定休・例外を横断的に判定できるようにしました。
- **feat:** `/api/holidays/check`・`/api/holidays/month` を追加し、利用者が指定日や月次カレンダー単位で休日情報を取得できるようにしました。
- **feat:** `/api/admin/holidays/weekly` (GET/POST) を実装し、管理者が定休曜日と適用期間を管理できるようにしました。システム管理者以外は翌日以降のみ登録可能です。
- **feat:** 勤怠の打刻 API (`/api/attendance/clock-in` / `clock-out`) で休日チェックを行い、例外がない限り休日の打刻を拒否するようにしました。
- **feat:** フロントエンドに定休曜日タブと月次休日表示／打刻ブロックを実装し、API と連動して UI 上でも休日運用を管理できるようにしました。
- **feat:** 追加した休日 API に合わせてフロントエンドを拡張し、管理画面での定休登録・定休一覧表示、勤怠画面でのカレンダー表示と打刻ブロック、ダッシュボードでの休日判定を実装しました。
- **feat:** 管理者向け休日一覧カードにページネーションと種別/期間フィルタ、表示件数切り替えを実装し、`page`・`per_page`・`type`・`from`・`to` クエリを備えた `/api/admin/holidays` と一対一で連動するようにしました。
- **feat:** `/api/config/timezone` を公開し、フロントエンドがバックエンドと同じタイムゾーン設定を取得できるようにしました。
- **ui:** 休日判定メッセージや定休設定フォームなど、各 UI コンポーネントにバリデーションとトーストを追加し、操作フィードバックを改善しました。
- **test:** `backend/tests/holiday_service.rs`、`holiday_api.rs`、`attendance_holiday.rs` を追加し、サービスロジック・APIバリデーション・勤怠ブロック挙動を TDD でカバーしました。
- **docs:** `API_DOCS.md` に休日 API と定休管理 API を追記し、`documents/holiday-implementation-plan.md` / `holiday-test-cases.md` で設計とテスト計画を共有しました。
- **ui:** フロントエンドが `/api/config/timezone` のフェッチに失敗した場合に、ページ上部へ警告バナーと手動再取得ボタンを表示し、バックエンド時刻とのズレを検知できるようにしました。
