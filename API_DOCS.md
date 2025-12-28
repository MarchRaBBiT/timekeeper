# Timekeeper API 仕様書

## 概要

Timekeeper勤怠管理システムのREST API仕様書です。

**ベースURL**: `http://localhost:3000/api`

## 認証

すべてのAPIエンドポイント（ログイン・リフレッシュ以外）は認証が必要です。

### 認証方式
- ブラウザ向け: HttpOnly Cookie（`access_token`/`refresh_token`）を使用します。
- 代替手段: `Authorization: Bearer <access_token>` も利用可能です。

### 認証ヘッダー（代替手段）
```
Authorization: Bearer <access_token>
```

## エンドポイント一覧

### 認証

#### ログイン
```http
POST /api/auth/login
Content-Type: application/json

{
  "username": "string",
  "password": "string"
}
```

**レスポンス**
```json
{
  "id": "string",
  "username": "string",
  "full_name": "string",
  "role": "employee" | "admin"
}
```
※ `Set-Cookie` で `access_token`/`refresh_token` を発行します。

#### トークンリフレッシュ
```http
POST /api/auth/refresh
Content-Type: application/json

{
  "device_label": "string"
}
```

#### ログインユーザー情報
```http
GET /api/auth/me
```

**レスポンス**
```json
{
  "user": {
    "id": "string",
    "username": "string",
    "full_name": "string",
    "role": "employee" | "admin"
  }
}
```

#### ログアウト（トークン失効）
```http
POST /api/auth/logout
Content-Type: application/json

{ }
```

もしくは全リフレッシュトークンを失効させる場合:

```http
POST /api/auth/logout
Authorization: Bearer <token>
Content-Type: application/json

{ "all": true }
```

**レスポンス**
```json
{ "message": "Logged out" }
```

### 構成情報

#### タイムゾーン取得
```http
GET /api/config/timezone
```

**レスポンス**
```json
{ "time_zone": "Asia/Tokyo" }
```

- 認証不要で、バックエンドが採用している IANA タイムゾーン名（`Config.time_zone`）を返します。

### 休日API（Holiday API）

#### 登録済み祝日の取得
```http
GET /api/holidays
Authorization: Bearer <token>
```

**レスポンス**
```json
[
  {
    "id": "8f0c5fd0-...",
    "holiday_date": "2025-01-01",
    "name": "元日",
    "description": "Google Calendar からの取り込み"
  }
]
```

#### 任意日が休日かどうかを確認する (`check`)
```http
GET /api/holidays/check?date=2025-01-08
Authorization: Bearer <token>
```

**レスポンス**
```json
{
  "is_holiday": true,
  "reason": "weekly holiday"
}
```

- `reason` は `public holiday`（祝日） / `weekly holiday`（定休） / `forced holiday`（例外で休日化） / `working day` のいずれか。
- 個別例外 (`holiday_exceptions`) を登録している場合、最優先で結果に反映されます。

#### 月単位の休日一覧を取得
```http
GET /api/holidays/month?year=2025&month=1
Authorization: Bearer <token>
```

**レスポンス**
```json
[
  { "date": "2025-01-01", "reason": "public holiday" },
  { "date": "2025-01-05", "reason": "weekly holiday" }
]
```

- `month` は 1〜12 のみ指定可能。
- `reason` の値は `check` API と同じ。
- 個別例外を設定したユーザーは、該当する日に `forced holiday` または `working day` が返却されます。

#### MFA登録開始
```http
POST /api/auth/mfa/register
Authorization: Bearer <token>
```

**レスポンス**
```json
{
  "secret": "JBSWY3DPEHPK3PXP",
  "otpauth_url": "otpauth://totp/Timekeeper:alice?secret=JBSWY3DPEHPK3PXP&issuer=Timekeeper"
}
```

**備考**
- まだMFAを有効化していない認証済みユーザーのみ利用可能
- 新しいシークレットを払い出し、既存の保留中シークレットを置き換えます
- `/api/auth/mfa/activate` でワンタイムコードを送信するまで有効化されません

**レスポンス**
```json
{
  "user": {
    "id": "string",
    "username": "string",
    "full_name": "string",
    "role": "employee" | "admin"
  }
}
```

### 勤怠管理

#### 出勤打刻
```http
POST /api/attendance/clock-in
Authorization: Bearer <token>
Content-Type: application/json

{
  "date": "2024-01-15" // オプション、指定しない場合は今日
}
```

**レスポンス**
```json
{
  "id": "string",
  "user_id": "string",
  "date": "2024-01-15",
  "clock_in_time": "2024-01-15T09:00:00",
  "clock_out_time": null,
  "status": "present",
  "total_work_hours": null,
  "break_records": []
}
```

#### 退勤打刻
```http
POST /api/attendance/clock-out
Authorization: Bearer <token>
Content-Type: application/json

{
  "date": "2024-01-15" // オプション、指定しない場合は今日
}
```

**レスポンス**
```json
{
  "id": "string",
  "user_id": "string",
  "date": "2024-01-15",
  "clock_in_time": "2024-01-15T09:00:00",
  "clock_out_time": "2024-01-15T18:00:00",
  "status": "present",
  "total_work_hours": 8.0,
  "break_records": []
}
```

#### 休憩開始
```http
POST /api/attendance/break-start
Authorization: Bearer <token>
Content-Type: application/json

{
  "attendance_id": "string"
}
```

**レスポンス**
```json
{
  "id": "string",
  "attendance_id": "string",
  "break_start_time": "2024-01-15T12:00:00",
  "break_end_time": null,
  "duration_minutes": null
}
```

#### 休憩終了
```http
POST /api/attendance/break-end
Authorization: Bearer <token>
Content-Type: application/json

{
  "break_record_id": "string"
}
```

**レスポンス**
```json
{
  "id": "string",
  "attendance_id": "string",
  "break_start_time": "2024-01-15T12:00:00",
  "break_end_time": "2024-01-15T13:00:00",
  "duration_minutes": 60
}
```

#### 勤怠履歴取得
```http
GET /api/attendance/me?year=2024&month=1
Authorization: Bearer <token>
```

**クエリパラメータ**
- `year` (optional): 年
- `month` (optional): 月

**レスポンス**
```json
[
  {
    "id": "string",
    "user_id": "string",
    "date": "2024-01-15",
    "clock_in_time": "2024-01-15T09:00:00",
    "clock_out_time": "2024-01-15T18:00:00",
    "status": "present",
    "total_work_hours": 8.0,
    "break_records": [
      {
        "id": "string",
        "attendance_id": "string",
        "break_start_time": "2024-01-15T12:00:00",
        "break_end_time": "2024-01-15T13:00:00",
        "duration_minutes": 60
      }
    ]
  }
]
```

#### 月次集計取得
```http
GET /api/attendance/me/summary?year=2024&month=1
Authorization: Bearer <token>
```

**レスポンス**
```json
{
  "month": 1,
  "year": 2024,
  "total_work_hours": 160.0,
  "total_work_days": 20,
  "average_daily_hours": 8.0
}
```

#### CSVエクスポート（勤怠データの出力）
```http
GET /api/attendance/export?from=2024-01-01&to=2024-01-31
Authorization: Bearer <token>
```

**クエリパラメータ**
- `from` (optional): YYYY-MM-DD 形式の出力対象期間の開始日
- `to` (optional): YYYY-MM-DD 形式の出力対象期間の終了日

**備考**
- 認証済みユーザー本人の勤怠のみがエクスポート対象
- `from`/`to` を同時に指定する場合は `from <= to` が必須（違反時は 400 Bad Request）
- どちらも省略すると取得済みの全履歴を返します

**レスポンス**
```json
{
  "csv_data": "Username,Full Name,Date,Clock In,Clock Out,Total Hours,Status\n...",
  "filename": "my_attendance_export_20240131_120000.csv"
}
```
### 申請管理

#### 休暇申請
```http
POST /api/requests/leave
Authorization: Bearer <token>
Content-Type: application/json

{
  "leave_type": "annual" | "sick" | "personal" | "other",
  "start_date": "2024-01-20",
  "end_date": "2024-01-22",
  "reason": "家族旅行"
}
```

**レスポンス**
```json
{
  "id": "string",
  "user_id": "string",
  "leave_type": "annual",
  "start_date": "2024-01-20",
  "end_date": "2024-01-22",
  "reason": "家族旅行",
  "status": "pending",
  "approved_by": null,
  "approved_at": null,
  "created_at": "2024-01-15T10:00:00Z"
}
```

#### 残業申請
```http
POST /api/requests/overtime
Authorization: Bearer <token>
Content-Type: application/json

{
  "date": "2024-01-15",
  "planned_hours": 2.5,
  "reason": "プロジェクト締切"
}
```

**レスポンス**
```json
{
  "id": "string",
  "user_id": "string",
  "date": "2024-01-15",
  "planned_hours": 2.5,
  "reason": "プロジェクト締切",
  "status": "pending",
  "approved_by": null,
  "approved_at": null,
  "created_at": "2024-01-15T10:00:00Z"
}
```

#### 申請一覧取得
```http
GET /api/requests/me
Authorization: Bearer <token>
```

**レスポンス**
```json
{
  "leave_requests": [
    {
      "id": "string",
      "user_id": "string",
      "leave_type": "annual",
      "start_date": "2024-01-20",
      "end_date": "2024-01-22",
      "reason": "家族旅行",
      "status": "pending",
      "approved_by": null,
      "approved_at": null,
      "created_at": "2024-01-15T10:00:00Z"
    }
  ],
  "overtime_requests": [
    {
      "id": "string",
      "user_id": "string",
      "date": "2024-01-15",
      "planned_hours": 2.5,
      "reason": "プロジェクト締切",
      "status": "pending",
      "approved_by": null,
      "approved_at": null,
      "created_at": "2024-01-15T10:00:00Z"
    }
  ]
}
```

### 管理者機能

#### 従業員一覧取得
```http
GET /api/admin/users
Authorization: Bearer <token>
```

**レスポンス**
```json
[
  {
    "id": "string",
    "username": "string",
    "full_name": "string",
    "role": "employee" | "admin"
  }
]
```

#### 従業員登録
```http
POST /api/admin/users
Authorization: Bearer <token>
Content-Type: application/json

{
  "username": "string",
  "password": "string",
  "full_name": "string",
  "role": "employee" | "admin"
}
```

**レスポンス**
```json
{
  "id": "string",
  "username": "string",
  "full_name": "string",
  "role": "employee" | "admin"
}
```

#### 定休曜日の設定一覧取得
```http
GET /api/admin/holidays/weekly
Authorization: Bearer <token>
```

**レスポンス**
```json
[
  {
    "id": "4634f6e3-...",
    "weekday": 0,
    "starts_on": "2025-01-06",
    "ends_on": null,
    "enforced_from": "2025-01-06",
    "enforced_to": null
  }
]
```

- `weekday` は 0=月曜〜6=日曜。
- `enforced_*` は実際の適用期間（履歴管理用）。

#### 定休曜日の適用開始/終了を登録
```http
POST /api/admin/holidays/weekly
Authorization: Bearer <token>
Content-Type: application/json

{
  "weekday": 2,
  "starts_on": "2025-01-08",
  "ends_on": null
}
```

**レスポンス**
```json
{
  "id": "4634f6e3-...",
  "weekday": 2,
  "starts_on": "2025-01-08",
  "ends_on": null,
  "enforced_from": "2025-01-08",
  "enforced_to": null
}
```

**補足**
- `weekday` は 0..6 のみ受け付け。
- `ends_on` は `null`（無期限）でも可。
- 一般管理者 (`is_system_admin=false`) は `starts_on` を翌日以降にしか設定できません。

#### 全従業員の勤怠データ取得
```http
GET /api/admin/attendance
Authorization: Bearer <token>
```

**レスポンス**
```json
[
  {
    "id": "string",
    "user_id": "string",
    "date": "2024-01-15",
    "clock_in_time": "2024-01-15T09:00:00",
    "clock_out_time": "2024-01-15T18:00:00",
    "status": "present",
    "total_work_hours": 8.0,
    "break_records": []
  }
]
```

#### 申請承認
```http
PUT /api/admin/requests/{id}/approve
Authorization: Bearer <token>
```

**レスポンス**
```json
{
  "message": "Leave request approved" // または "Overtime request approved"
}
```

#### 申請却下
```http
PUT /api/admin/requests/{id}/reject
Authorization: Bearer <token>
```

**レスポンス**
```json
{
  "message": "Leave request rejected" // または "Overtime request rejected"
}
```

#### データエクスポート
```http
GET /api/admin/export
Authorization: Bearer <token>
```

**クエリパラメータ（任意）**
- `username` (optional): ユーザー名（完全一致）
- `from` (optional): 期間開始日 `YYYY-MM-DD`
- `to` (optional): 期間終了日 `YYYY-MM-DD`

例:
```
GET /api/admin/export?username=alice&from=2025-10-01&to=2025-10-31
```

**レスポンス**
```json
{
  "csv_data": "Username,Full Name,Date,Clock In,Clock Out,Total Hours,Status\n...",
  "filename": "attendance_export_20240115_120000.csv"
}
```

#### 監査ログ一覧取得
```http
GET /api/admin/audit-logs
Authorization: Bearer <token>
```

**クエリパラメータ（任意）**
- `from`: 期間開始（RFC3339 または `YYYY-MM-DD`）
- `to`: 期間終了（RFC3339 または `YYYY-MM-DD`）
- `actor_id`: 行為者ユーザーID
- `actor_type`: 行為者タイプ（例: `user`, `anonymous`）
- `event_type`: イベント種別
- `target_type`: 対象種別
- `target_id`: 対象ID
- `result`: `success` | `failure`
- `page`: ページ番号（既定=1）
- `per_page`: 1..100（既定=25）

**レスポンス**
```json
{
  "page": 1,
  "per_page": 25,
  "total": 120,
  "items": [
    {
      "id": "string",
      "occurred_at": "2025-01-01T09:00:00Z",
      "actor_id": "string",
      "actor_type": "user",
      "event_type": "attendance_clock_in",
      "target_type": "attendance",
      "target_id": "string",
      "result": "success",
      "error_code": null,
      "metadata": { "source": "web" },
      "ip": "127.0.0.1",
      "user_agent": "string",
      "request_id": "string"
    }
  ]
}
```

**備考**
- システム管理者のみアクセス可能
- `from` と `to` を同時に指定する場合は `from <= to` が必須

#### 監査ログ詳細取得
```http
GET /api/admin/audit-logs/{id}
Authorization: Bearer <token>
```

**レスポンス**
```json
{
  "id": "string",
  "occurred_at": "2025-01-01T09:00:00Z",
  "actor_id": "string",
  "actor_type": "user",
  "event_type": "attendance_clock_in",
  "target_type": "attendance",
  "target_id": "string",
  "result": "success",
  "error_code": null,
  "metadata": { "source": "web" },
  "ip": "127.0.0.1",
  "user_agent": "string",
  "request_id": "string"
}
```

#### 監査ログJSONエクスポート
```http
GET /api/admin/audit-logs/export
Authorization: Bearer <token>
```

**クエリパラメータ（任意）**
- `from` / `to` / `actor_id` / `actor_type` / `event_type` / `target_type` / `target_id` / `result`

**レスポンス**
- `Content-Disposition: attachment; filename="audit_logs_YYYYMMDD_HHMMSS.json"`
- `Content-Type: application/json`
- ボディは監査ログ配列（一覧と同じ形式）

## エラーレスポンス

すべてのエンドポイントは以下の形式でエラーを返します：

```json
{
  "error": "エラーメッセージ"
}
```

### HTTPステータスコード

- `200` - 成功
- `201` - 作成成功
- `400` - リクエストエラー
- `401` - 認証エラー
- `403` - 権限エラー
- `404` - リソースが見つからない
- `409` - 競合（例：ユーザー名重複）
- `500` - サーバーエラー

## データ型

### 日付・時刻フォーマット
- 日付: `YYYY-MM-DD` (例: `2024-01-15`)
- 時刻: `YYYY-MM-DDTHH:MM:SS` (例: `2024-01-15T09:00:00`)

### 列挙型

#### ユーザー権限
- `employee` - 従業員
- `admin` - 管理者

#### 勤怠ステータス
- `present` - 出勤
- `absent` - 欠勤
- `late` - 遅刻
- `half_day` - 半日

#### 休暇種別
- `annual` - 有給休暇
- `sick` - 病気休暇
- `personal` - 私用休暇
- `other` - その他

#### 申請ステータス
- `pending` - 承認待ち
- `approved` - 承認済み
- `rejected` - 却下
- `cancelled` - 取消（本人によるキャンセル）


## 表記ルール（Conventions）

### Enum値の表記（casing）
- 本APIで扱う列挙値はすべて `snake_case` を使用します。
- ユーザーの役割（`role`）: `employee` | `admin`
- 勤怠ステータス（`status`）: `present` | `absent` | `late` | `half_day`
- 申請ステータス（`status`）: `pending` | `approved` | `rejected` | `cancelled`
- 休暇種別（`leave_type`）: `annual` | `sick` | `personal` | `other`

# P0 Updates (New/Changed Endpoints)

本セクションはP0実装に伴う追加/変更点の要約です。既存セクションの補足として参照してください。

## 勤怠（Attendance）

### 現在ステータス取得

GET /api/attendance/status

クエリ:
- `date` (任意, YYYY-MM-DD, 省略時=今日)

レスポンス:
```json
{
  "status": "not_started" | "clocked_in" | "on_break" | "clocked_out",
  "attendance_id": "string|null",
  "active_break_id": "string|null",
  "clock_in_time": "2024-01-15T09:00:00|null",
  "clock_out_time": "2024-01-15T18:00:00|null"
}
```

### 自分の勤怠（期間対応）

GET /api/attendance/me

クエリ（拡張）:
- `from` (任意, YYYY-MM-DD)
- `to` (任意, YYYY-MM-DD)

備考:
- `from`/`to` 指定時は `year`/`month` と併用不可
- `from` のみ指定で当日までの範囲

### 勤怠IDに紐づく休憩一覧

GET /api/attendance/{id}/breaks

レスポンス:
```json
[
  {
    "id": "string",
    "attendance_id": "string",
    "break_start_time": "2024-01-15T12:00:00",
    "break_end_time": "2024-01-15T13:00:00|null",
    "duration_minutes": 60
  }
]
```

## 管理者（Admin）勤怠操作

### 勤怠の作成/置換（アップサート）

PUT /api/admin/attendance

ボディ:
```json
{
  "user_id": "string",
  "date": "YYYY-MM-DD",
  "clock_in_time": "YYYY-MM-DDTHH:MM:SS",
  "clock_out_time": "YYYY-MM-DDTHH:MM:SS|null",
  "breaks": [
    { "break_start_time": "YYYY-MM-DDTHH:MM:SS", "break_end_time": "YYYY-MM-DDTHH:MM:SS|null" }
  ]
}
```

レスポンス: Attendanceの詳細（従来型。`break_records` を含む）

備考: 同一ユーザー/同一日が存在する場合は既存を置換します。

### 休憩の強制終了

PUT /api/admin/breaks/{id}/force-end

ボディ: なし（現在時刻で終了）

レスポンス: BreakRecord（更新後）

## 申請（Requests）

### 管理者向け 申請一覧

GET /api/admin/requests

クエリ:
- `status` (任意: pending|approved|rejected|cancelled)
- `user_id` (任意)
- `page` (任意, 既定=1)
- `per_page` (任意, 1..100, 既定=20)

レスポンス:
```json
{
  "leave_requests": [ { /* LeaveRequestResponse */ } ],
  "overtime_requests": [ { /* OvertimeRequestResponse */ } ],
  "page_info": { "page": 1, "per_page": 20 }
}
```

備考: 追加のフィルタ（type/from/to等）は今後拡張予定。

### 管理者向け 申請詳細

GET /api/admin/requests/{id}

レスポンス:
```json
{ "kind": "leave" | "overtime", "data": { /* *RequestResponse */ } }
```

### 申請の承認（コメント必須）

PUT /api/admin/requests/{id}/approve

ボディ:
```json
{ "comment": "string (1..500)" }
```

レスポンス例:
```json
{ "message": "Leave request approved" }
```

### 申請の却下（コメント必須）

PUT /api/admin/requests/{id}/reject

ボディ:
```json
{ "comment": "string (1..500)" }
```

レスポンス例:
```json
{ "message": "Leave request rejected" }
```

### 本人による申請の編集（承認前）

PUT /api/requests/{id}

ボディ（leave の例。部分更新可）:
```json
{ "leave_type": "annual|sick|personal|other", "start_date": "YYYY-MM-DD", "end_date": "YYYY-MM-DD", "reason": "string|null" }
```

ボディ（overtime の例。部分更新可）:
```json
{ "date": "YYYY-MM-DD", "planned_hours": 1.5, "reason": "string|null" }
```

レスポンス:
```json
{ "message": "Leave request updated" }
```

### 本人による申請の取消（承認前）

DELETE /api/requests/{id}

レスポンス:
```json
{ "id": "string", "status": "cancelled" }
```

## レスポンス項目の拡張（Requests）

LeaveRequestResponse / OvertimeRequestResponse に以下が追加されました:
- `decision_comment` (string|null)
- `rejected_by` (string|null)
- `rejected_at` (datetime|null)
- `cancelled_at` (datetime|null)

## 管理系エンドポイント抜粋（フロント実装で使用）

| 区分 | メソッド | パス | 主な用途 | 主なパラメータ/ボディ |
| --- | --- | --- | --- | --- |
| 管理・申請 | GET | `/api/admin/requests` | 管理者が申請一覧を取得 | `status`、`user_id`、`page`、`per_page` |
| 管理・申請 | GET | `/api/admin/requests/{id}` | 申請詳細取得 | `id` (path) |
| 管理・申請 | PUT | `/api/admin/requests/{id}/approve` | 申請承認 | `id` (path), BODY: `comment` |
| 管理・申請 | PUT | `/api/admin/requests/{id}/reject` | 申請却下 | `id` (path), BODY: `comment` |
| 管理・ユーザー | GET | `/api/admin/users` | ユーザー一覧取得 | なし |
| 管理・ユーザー | POST | `/api/admin/users` | 新規ユーザー作成 | BODY: `username`, `full_name`, `role` など |
| 管理・ユーザー | PUT | `/api/admin/users/{id}` | ユーザー更新 | `id` (path), BODY: `full_name`, `role` など |
| 管理・MFA | POST | `/api/admin/mfa/reset` | MFA リセット | BODY: `user_id` |
| 管理・休日 | GET | `/api/admin/holidays` | 休日一覧取得 | なし |
| 管理・休日 | POST | `/api/admin/holidays` | 休日登録 | BODY: `date`, `reason` など |
| 管理・休日 | DELETE | `/api/admin/holidays/{id}` | 休日削除 | `id` (path) |
| 管理・休日（週次） | GET | `/api/admin/holidays/weekly` | 週次休日一覧取得 | なし |
| 管理・休日（週次） | POST | `/api/admin/holidays/weekly` | 週次休日登録 | BODY: `weekday`, `reason` など |
| 管理・休日（インポート） | GET | `/api/admin/holidays/google` | Google 祝日一覧取得 | `year` (optional) |
| 管理・エクスポート | GET | `/api/admin/export` | 管理用エクスポート（CSV/JSON） | クエリ: `username`, `from`, `to` など |
| 管理・監査 | GET | `/api/admin/audit-logs` | 監査ログ一覧取得 | クエリ: `from`, `to`, `actor_id`, `event_type`, `result` など |
| 管理・監査 | GET | `/api/admin/audit-logs/{id}` | 監査ログ詳細取得 | `id` (path) |
| 管理・監査 | GET | `/api/admin/audit-logs/export` | 監査ログJSONエクスポート | クエリ: `from`, `to`, `actor_id`, `event_type`, `result` など |

## 監査ログ（イベントカタログ）

監査ログは API 呼出を起点とし、成功/失敗を含むイベントを記録します。記録は Create/Read のみで、更新・削除は行いません。

### event_type 命名規約
- `snake_case` を使用します（本 API の列挙値ルールに合わせる）。
- 例: `attendance_clock_in`, `admin_request_approve`, `password_change`

### result/error_code ルール
- `result` は `success` / `failure`。
- `error_code` は固定文字列のエラー識別子を優先し、未定義の場合は `http_<status>` で記録します。

### target_type/target_id ルール
- `target_type` は `snake_case`（例: `user`, `attendance`, `break_record`, `request`, `holiday`, `weekly_holiday`, `holiday_exception`, `export`, `system`）。
- `target_id` は対象 ID を設定し、一覧/検索系で対象が特定できない場合は `null`。

### metadata 共通キー

| key | 用途 | 例 |
| --- | --- | --- |
| `request_type` | 申請の種別 | `leave` / `overtime` |
| `payload_summary` | 申請内容の概要（機微情報を除外） | `{ "leave_type": "annual", "start_date": "2025-01-10" }` |
| `approval_step` | 承認フェーズ | `single` |
| `decision` | 承認/却下の判定 | `approve` / `reject` |
| `clock_type` | 打刻の種別 | `clock_in` / `clock_out` / `break_start` / `break_end` |
| `timezone` | 打刻のタイムゾーン | `Asia/Tokyo` |
| `source` | 打刻の起点 | `web` / `api` |
| `method` | パスワード変更の方式 | `password` |
| `mfa_enabled` | パスワード変更時の MFA 有効状態 | `true` / `false` |
| `export_from` | エクスポート開始日 | `2025-01-01` |
| `export_to` | エクスポート終了日 | `2025-01-31` |
| `filters` | 一覧取得の検索条件 | `{ "status": "pending", "user_id": "..." }` |
| `year` | 祝日取得の対象年 | `2025` |

※ metadata には機微情報（パスワードやトークンなど）を保存しないこと。

### 監査対象イベント

#### 認証/セキュリティ

| メソッド | パス | event_type | target_type | target_id | 備考 |
| --- | --- | --- | --- | --- | --- |
| POST | `/api/auth/login` | `auth_login` | `user` | `user_id/null` | 失敗時は user_id 不明 |
| POST | `/api/auth/refresh` | `auth_refresh` | `user` | `user_id` |  |
| POST | `/api/auth/logout` | `auth_logout` | `user` | `user_id` |  |
| POST | `/api/auth/mfa/register` | `mfa_register` | `user` | `user_id` |  |
| POST | `/api/auth/mfa/setup` | `mfa_setup` | `user` | `user_id` |  |
| POST | `/api/auth/mfa/activate` | `mfa_activate` | `user` | `user_id` |  |
| DELETE | `/api/auth/mfa` | `mfa_disable` | `user` | `user_id` |  |
| PUT | `/api/auth/change-password` | `password_change` | `user` | `user_id` |  |

#### 勤怠

| メソッド | パス | event_type | target_type | target_id | 備考 |
| --- | --- | --- | --- | --- | --- |
| POST | `/api/attendance/clock-in` | `attendance_clock_in` | `attendance` | `attendance_id` |  |
| POST | `/api/attendance/clock-out` | `attendance_clock_out` | `attendance` | `attendance_id` |  |
| POST | `/api/attendance/break-start` | `attendance_break_start` | `break_record` | `break_record_id` |  |
| POST | `/api/attendance/break-end` | `attendance_break_end` | `break_record` | `break_record_id` |  |
| GET | `/api/attendance/export` | `attendance_export` | `export` | `null` | 期間は metadata に格納 |

#### 申請

| メソッド | パス | event_type | target_type | target_id | 備考 |
| --- | --- | --- | --- | --- | --- |
| POST | `/api/requests/leave` | `request_leave_create` | `request` | `request_id` |  |
| POST | `/api/requests/overtime` | `request_overtime_create` | `request` | `request_id` |  |
| PUT | `/api/requests/{id}` | `request_update` | `request` | `{id}` |  |
| DELETE | `/api/requests/{id}` | `request_cancel` | `request` | `{id}` |  |

#### 管理者

| メソッド | パス | event_type | target_type | target_id | 備考 |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/admin/requests` | `admin_request_list` | `system` | `null` | フィルタ条件は metadata に格納 |
| GET | `/api/admin/requests/{id}` | `admin_request_detail` | `request` | `{id}` |  |
| PUT | `/api/admin/requests/{id}/approve` | `admin_request_approve` | `request` | `{id}` |  |
| PUT | `/api/admin/requests/{id}/reject` | `admin_request_reject` | `request` | `{id}` |  |
| GET | `/api/admin/holidays` | `admin_holiday_list` | `system` | `null` |  |
| POST | `/api/admin/holidays` | `admin_holiday_create` | `holiday` | `holiday_id` |  |
| DELETE | `/api/admin/holidays/{id}` | `admin_holiday_delete` | `holiday` | `{id}` |  |
| GET | `/api/admin/holidays/weekly` | `admin_weekly_holiday_list` | `system` | `null` |  |
| POST | `/api/admin/holidays/weekly` | `admin_weekly_holiday_create` | `weekly_holiday` | `weekly_holiday_id` |  |
| GET | `/api/admin/holidays/google` | `admin_holiday_google_fetch` | `system` | `null` | `year` は metadata に格納 |
| GET | `/api/admin/users/{user_id}/holiday-exceptions` | `admin_holiday_exception_list` | `user` | `{user_id}` |  |
| POST | `/api/admin/users/{user_id}/holiday-exceptions` | `admin_holiday_exception_create` | `holiday_exception` | `holiday_exception_id` |  |
| DELETE | `/api/admin/users/{user_id}/holiday-exceptions/{id}` | `admin_holiday_exception_delete` | `holiday_exception` | `{id}` |  |
| GET | `/api/admin/export` | `admin_export` | `export` | `null` | フィルタ条件は metadata に格納 |

#### システム管理者

| メソッド | パス | event_type | target_type | target_id | 備考 |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/admin/users` | `admin_user_list` | `system` | `null` |  |
| POST | `/api/admin/users` | `admin_user_create` | `user` | `user_id` |  |
| GET | `/api/admin/attendance` | `admin_attendance_list` | `system` | `null` |  |
| PUT | `/api/admin/attendance` | `admin_attendance_upsert` | `attendance` | `attendance_id` |  |
| PUT | `/api/admin/breaks/{id}/force-end` | `admin_break_force_end` | `break_record` | `{id}` |  |
| POST | `/api/admin/mfa/reset` | `admin_mfa_reset` | `user` | `user_id` |  |

### 除外対象（監査ログを記録しない）

| メソッド | パス | 理由 |
| --- | --- | --- |
| GET | `/api/config/timezone` | 公開設定の参照 |
| GET | `/api/auth/me` | 認証状態の参照 |
| GET | `/api/auth/mfa` | MFA 状態の参照 |
| GET | `/api/holidays` | 祝日カレンダー参照 |
| GET | `/api/holidays/check` | 祝日判定の参照 |
| GET | `/api/holidays/month` | 月次祝日一覧の参照 |
| GET | `/api/attendance/status` | 自分の勤怠状態の参照 |
| GET | `/api/attendance/me` | 自分の勤怠一覧参照 |
| GET | `/api/attendance/me/summary` | 自分の勤怠集計参照 |
| GET | `/api/attendance/{id}/breaks` | 休憩一覧参照 |
| GET | `/api/requests/me` | 自分の申請一覧参照 |
