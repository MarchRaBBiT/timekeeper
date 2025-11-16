# Timekeeper API 仕様書

## 概要

Timekeeper勤怠管理システムのREST API仕様書です。

**ベースURL**: `http://localhost:3000/api`

## 認証

すべてのAPIエンドポイント（ログイン・リフレッシュ以外）はJWT認証が必要です。

### 認証ヘッダー
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
  "access_token": "string",
  "refresh_token": "string",
  "user": {
    "id": "string",
    "username": "string",
    "full_name": "string",
    "role": "employee" | "admin"
  }
}
```

#### トークンリフレッシュ
```http
POST /api/auth/refresh
Content-Type: application/json

{
  "refresh_token": "string"
}
```

#### ログアウト（トークン失効）
```http
POST /api/auth/logout
Authorization: Bearer <token>
Content-Type: application/json

{ "refresh_token": "string" }
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
  "access_token": "string",
  "refresh_token": "string",
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

#### 休日の横断一覧をページネーション表示
```http
GET /api/admin/holidays?page=1&per_page=25&type=public&from=2025-01-01&to=2025-03-31
Authorization: Bearer <token>
```

**クエリパラメータ**

- `page` (デフォルト 1) … 1 始まりのページ番号。指定しない場合は 1。
- `per_page` (デフォルト 25) … 1〜100 の間で表示件数を指定。
- `type` … `public` (祝日) / `weekly` (定休) / `exception` (休日例外) のいずれか。省略または `all` で全件。
- `from` / `to` … `YYYY-MM-DD` 形式。適用開始日 (`applies_from`) の範囲で絞り込みます。

**レスポンス**
```json
{
  "page": 1,
  "per_page": 25,
  "total": 42,
  "items": [
    {
      "id": "a8f6...",
      "kind": "public",
      "applies_from": "2025-01-01",
      "applies_to": null,
      "date": "2025-01-01",
      "weekday": null,
      "starts_on": null,
      "ends_on": null,
      "name": "元日",
      "description": "Google Calendar",
      "user_id": null,
      "reason": null,
      "created_by": "system",
      "created_at": "2025-01-01T00:00:00Z",
      "is_override": null
    }
  ]
}
```

- `kind` … `public` / `weekly` / `exception` を返します。
- `applies_from` / `applies_to` … 区間があるレコード（定休や例外）で適用範囲を示します。
- `date` … 祝日実体の日付。定休や例外は `null` になります。
- `weekday` … `kind=weekly` のときに 0=月〜6=日 を返します。
- `user_id`・`is_override` … 例外 (exception) の対象と上書き種別を示します。

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
