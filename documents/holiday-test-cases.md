# 休日定義拡張 テストケース集

このドキュメントは「休日定義拡張」実装で追加する単体テストおよび機能テスト（API/E2E）の網羅を目的とする。実装完了後に全ケースを自動化して回帰テストへ組み込む。

## 1. 単体テスト

### 1.1 `holiday_service.rs`
| ID | シナリオ | 前提データ | 期待結果 |
| --- | --- | --- | --- |
| UT-HS-001 | 祝日のみ休日判定 | `holidays` に 2025-01-01 | `is_holiday(2025-01-01, None)` → `true (reason=public_holiday)` |
| UT-HS-002 | 定休のみ休日判定 | `weekly_holidays`: 水曜 (starts_on=2025-01-01) | 2025-01-08(水) → `true (weekly_holiday)` |
| UT-HS-003 | 祝日+定休の重複 | 両方登録 | `reason` は `public_holiday` を優先（複数理由配列でも可） |
| UT-HS-004 | 例外: 出勤扱い | `holiday_exceptions`: userA 2025-01-08 override=false | `is_holiday(date, userA)` → `false (reason=exception_override)` |
| UT-HS-005 | 例外: 休暇付与 | override=true on 平日 | `is_holiday` → `true (reason=exception_override)` |
| UT-HS-006 | 定休の適用期間外 | `ends_on=2025-01-31` | 2025-02-01 → `false` |
| UT-HS-007 | キャッシュ無効化 | 月次キャッシュ生成後、`invalidate_month(year, month)` 呼び出しで再計算されること |
| UT-HS-008 | タイムゾーン変換 | `config.time_zone=Asia/Tokyo`、UTC で 2025-01-01 15:00 をローカル 2025-01-02 と計算するユーティリティの挙動を確認 |

### 1.2 `handlers::attendance`
| ID | シナリオ | 入力 | 期待結果 |
| --- | --- | --- | --- |
| UT-AT-001 | 平日の打刻成功 | 祝日/定休なし | `clock_in` → `200 OK` |
| UT-AT-002 | 休日打刻禁止 | `is_holiday=true` | `clock_in` → `403` + メッセージ |
| UT-AT-003 | 例外による打刻許可 | `holiday_service` が `false` を返す | 打刻成功 |
| UT-AT-004 | `check_holiday` エンドポイント JSON 構造 | Query=2025-01-01 | `{"is_holiday":true,"reason":"public_holiday"}` |

### 1.3 `handlers::admin`（定休 API）
| ID | シナリオ | 入力 | 期待結果 |
| --- | --- | --- | --- |
| UT-AD-001 | starts_on バリデーション | 非システム管理者、starts_on=今日 | `400` with validation error |
| UT-AD-002 | システム管理者の制約緩和 | starts_on=過去日 | `201` |
| UT-AD-003 | 重複登録防止 | 既存と同 weekday + overlapping range | `409` |
| UT-AD-004 | ends_on < starts_on | `400` |
| UT-AD-005 | 正常登録 | 有効期間指定 | DB に作成され戻り値に ID が含まれる |

## 2. 機能テスト（API/E2E）

### 2.1 API 統合テスト（`backend/tests/holiday_api.rs` など）
| ID | シナリオ | ステップ | 期待結果 |
| --- | --- | --- | --- |
| IT-API-001 | 定休登録→月次取得 | (1) Admin トークンで `POST /api/holidays/weekly` (水曜) (2) `GET /api/holidays/month?2025-01` | レスポンスに対象日の `kind="weekly_holiday"` が含まれる |
| IT-API-002 | 例外登録→休日解除 | (1) 定休日あり (2) `holiday_exceptions` に対象ユーザーの override=false を直接投入（現在 API 未提供のためテスト helper か SQL で登録） (3) `/api/holidays/check` を user 指定で呼ぶ | `is_holiday=false` |
| IT-API-003 | 祝日+定休→休日 | (1) `holidays` に 1/1 (2) 定休日 1/1 と被る (3) `/api/holidays/check` | `reason` に祝日が含まれる |
| IT-API-004 | 勤怠禁止→例外許可 | (1) 休日に `POST /api/attendance/clock-in` → `403` (2) 例外 override=false 登録 (3) 再度打刻 → `200` |

### 2.2 E2E（Playwright）
| ID | シナリオ | ステップ概要 | 期待結果 |
| --- | --- | --- |
| E2E-001 | 管理者が定休追加 | Admin ログイン → Admin ページ → 「休日設定」タブで曜日/期間入力 → 保存 | 成功トースト + 一覧に表示 |
| E2E-002 | ユーザーの休日表示 | 一般ユーザーで勤務表を開く | 月次カレンダーに該当曜日が休日表示（色/ラベル） |
| E2E-003 | 休日打刻禁止 UI | 休日を選択した状態で打刻ボタン押下 | UI ダイアログでブロック表示 |
| E2E-004 | 例外による許可 | 管理者が特定ユーザーに例外を登録 → ユーザー画面を再読み込み | 該当日に「例外で出勤可」の表示 + ボタン有効 |

## 3. 実装ノート
- SQLx ベースの統合テストは `#[sqlx::test(migrations = "./migrations")]` を使用し、準備したテスト DB で自動的に schema を適用する。
- Playwright では `FRONTEND_BASE_URL` を `.env.e2e` で指定し、休日 API をスタブせず実サーバーを起動して検証する。
- 将来的な部署別拡張時には上記テーブルに `department_id` が追加される想定のため、テストデータ投入は必ず helper を介して行う（直接 INSERT ではなく `TestDataBuilder` を利用）。

以上のケースを CI に組み込むことで、休日ロジック変更時の回帰を防ぐ。
