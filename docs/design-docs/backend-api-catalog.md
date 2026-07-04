# Backend API Catalog

**Updated:** 2026-06-20
**Purpose:** frontend が「存在しない backend endpoint」を前提に実装しないための、実装起点の API 契約一覧です。

## Maintenance Directive

- `/api/**` の route、HTTP method、認可要件、request body、query/path parameter、response、error 仕様を変更した場合は、この文書を同じ PR / commit で必ず更新すること。
- endpoint の存在確認は `backend/src/main.rs` を最優先の source of truth とし、契約の詳細は `backend/src/handlers/**`、`backend/src/models/**`、`backend/tests/*_api.rs` を参照すること。
- `backend/src/docs.rs` の OpenAPI stub は補助資料です。この文書と差分が出た場合は、同じ変更内で同期を取り直すこと。
- frontend 側の新規改修では「推測した endpoint」を増やさず、この文書か実装を確認してから利用すること。

## Common Rules

- 認証レベル
  - `Public`: 認証不要
  - `User`: 認証済みユーザー
  - `Admin`: `admin` または `system admin`
  - `System Admin`: `is_system_admin = true`
- 標準エラー形式
  - 多くの endpoint は `backend/src/error/mod.rs` の共通 envelope を返します。
  - 形式: `{"error":"...","code":"...","details":{...}?}`
- 例外的な legacy error 形式
  - `POST /api/consents`
  - `GET /api/consents/me`
  - `POST /api/subject-requests`
  - `GET /api/subject-requests/me`
  - `DELETE /api/subject-requests/{id}`
  - これらは主に `{"error":"..."}` 形式を返します。
- PII マスキング用 header
  - 一部の管理 endpoint は `X-PII-Masked: true|false` を返します。
  - 監査ログ export は `X-Truncated: true|false` も返します。
- CSRF 保護
  - `User` / `Admin` / `System Admin` 認可レベルの全 mutation endpoint (POST / PUT / DELETE / PATCH) は、cookie 認証クライアントに対して `Origin` または `Referer` ヘッダーを要求します。
  - ヘッダーが欠落または設定済み allowed origin と不一致の場合は `403 Forbidden` を返します。
  - `Authorization: Bearer ...` ヘッダーを持つリクエストはプログラマティッククライアントとみなし、CSRF チェックをスキップします。
  - `POST /api/auth/refresh` は cookie 経由で refresh token が届く場合のみ上記と同様の CSRF チェックを行います。JSON body で `refresh_token` を渡すプログラマティッククライアントはチェックの対象外です。
  - 照合先の allowlist は環境変数 `CORS_ALLOW_ORIGINS`（サーバー起動時にロードされる `Config.cors_allow_origins`）そのものであり、CSRF と CORS は同じ設定値を共有します。ワイルドカード (`*`) はこの allowlist に設定できず、起動時に拒否されます（`Config::load()` が fail-closed で `Err` を返す）。

## Public

| Endpoint | Method | Auth | Parameters | Success Response | Primary Errors | Summary |
| --- | --- | --- | --- | --- | --- | --- |
| `/api/auth/login` | `POST` | Public | Body `LoginRequest { username, password, totp_code?, device_label? }` | `200 LoginResponse { user: UserResponse }` + `access_token` / `refresh_token` cookie | `400` validation, `401` invalid credentials or invalid MFA, `500` DB/token issue | ログインし、現在ユーザー情報と session cookie を発行する |
| `/api/auth/refresh` | `POST` | Public | Body JSON `refresh_token?`; cookie の `refresh_token` でも可 | `200 LoginResponse { user }` + rotated auth cookie | `400` refresh token missing, `401` invalid / expired / rotated refresh token, `403` origin 不正 (cookie 経由のみ), `500` token rotation failure | refresh token を使って access / refresh token を再発行する |
| `/api/auth/request-password-reset` | `POST` | Public | Body `RequestPasswordResetPayload { email }` | `200 {"message":"If the email exists, a password reset link has been sent"}` | `400` invalid email, `500` DB or mail setup issue | パスワードリセットメール送信を要求する。存在しない email でも成功メッセージを返す |
| `/api/auth/reset-password` | `POST` | Public | Body `ResetPasswordPayload { token, new_password }` | `200 {"message":"Password has been reset successfully"}` | `400` invalid / expired / already-used token, password policy violation, `500` DB/update failure | reset token を消費して新しい password に更新し、既存 session を失効させる |
| `/api/config/timezone` | `GET` | Public | なし | `200 TimeZoneResponse { time_zone }` | なし | backend の標準 timezone を返す |

## User Auth / Profile / Session

| Endpoint | Method | Auth | Parameters | Success Response | Primary Errors | Summary |
| --- | --- | --- | --- | --- | --- | --- |
| `/api/auth/mfa` | `GET` / `DELETE` | User | `GET`: なし; `DELETE`: Body `MfaCodeRequest { code }` | `GET 200 MfaStatusResponse { enabled, pending }`; `DELETE 200 {"message":"MFA disabled"}` | `401` invalid MFA code, `400` MFA 未有効, `500` disable failure | MFA 状態参照 / MFA 無効化 |
| `/api/auth/mfa/register` | `POST` | User | header の origin 検証あり; body なし | `200 MfaSetupResponse { secret, otpauth_url }` | `403/400` origin 不正, `500` setup failure | MFA enrollment を開始する。`/api/auth/mfa/setup` と同等 |
| `/api/auth/mfa/setup` | `POST` | User | header の origin 検証あり; body なし | `200 MfaSetupResponse { secret, otpauth_url }` | `403/400` origin 不正, `500` setup failure | MFA enrollment を開始する |
| `/api/auth/mfa/activate` | `POST` | User | header の origin 検証あり; Body `MfaCodeRequest { code }` | `200 {"message":"MFA enabled"}` | `400` setup 未開始, `401` invalid MFA code, `500` enable failure | MFA を有効化し、既存 token を失効させる |
| `/api/auth/me` | `GET` / `PUT` | User | `GET`: なし; `PUT`: Body `UpdateProfile { full_name?, email?, current_password? }` | `GET 200 UserResponse`; `PUT 200 UserResponse` | `400` validation, email conflict, current_password missing, `401` current password incorrect, `403` origin 不正 (PUT / cookie auth), `500` encrypt/update failure | 自分の profile 取得 / 更新 |
| `/api/auth/sessions` | `GET` | User | なし | `200 [SessionResponse]` | `500` session lookup failure | 自分の active session 一覧取得 |
| `/api/auth/sessions/{id}` | `DELETE` | User | Path `id` | `200 {"message":"Session revoked","session_id":id}` | `400` empty id or current session revoke, `403` own session 以外, `404` not found, `500` revoke failure | 自分の別 session を revoke する |
| `/api/auth/change-password` | `PUT` | User | header の origin 検証あり; Body `ChangePasswordRequest { current_password, new_password }` | `200 {"message":"Password changed successfully"}` | `400` validation, policy violation, same password, `401` current password incorrect, `500` update failure | password を変更し、既存 session を全失効させる |
| `/api/auth/logout` | `POST` | User | Body `{ all?: boolean, refresh_token?: string }`; cookie の refresh token でも可 | `200 {"message":"Logged out"}` or `{"message":"Logged out from all devices"}` + cookie clear | `403` origin 不正 (cookie auth), `500` revoke failure | 現在 session または全 session を logout する |

## User Attendance

| Endpoint | Method | Auth | Parameters | Success Response | Primary Errors | Summary |
| --- | --- | --- | --- | --- | --- | --- |
| `/api/attendance/clock-in` | `POST` | User | Body `ClockInRequest { date? }`; 省略時はtimezoneと日界時刻から勤務日を解決 | `200 AttendanceResponse` | `400` already clocked in, `422 WORK_SCHEDULE_NOT_CONFIGURED`, `500` resolver/persistence failure | 勤務予定を解決・固定して出勤打刻を保存する。祝日・非勤務日も予定外勤務として許可する |
| `/api/attendance/clock-out` | `POST` | User | Body `ClockOutRequest { date? }`; 省略時は未退勤attendanceを使用 | `200 AttendanceResponse` | `404` 未退勤attendanceなし, `400` 未出勤 / 既に退勤 / 休憩中, `500` update failure | 出勤時と同じ勤務日snapshotを使って退勤し、total_work_hoursを再計算する |
| `/api/attendance/break-start` | `POST` | User | Body `BreakStartRequest { attendance_id }` | `200 BreakRecordResponse` | `403` 他人の attendance, `400` 未出勤 / 既に休憩中, `404` attendance not found, `500` create failure | 休憩開始 |
| `/api/attendance/break-end` | `POST` | User | Body `BreakEndRequest { break_record_id }` | `200 BreakRecordResponse` | `400` 既に終了済み, `403` 他人の attendance, `404` break not found, `500` update failure | 休憩終了 |
| `/api/attendance/status` | `GET` | User | Query `date?` (`YYYY-MM-DD`) | `200 AttendanceStatusResponse { status, attendance_id?, active_break_id?, clock_in_time?, clock_out_time? }` | `500` lookup failure | 当日または指定日の打刻状態を返す |
| `/api/attendance/me` | `GET` | User | Query `year?`, `month?`, `from?`, `to?`; `from/to` 優先 | `200 [AttendanceResponse]` | `400` `from > to`, invalid year/month, `500` lookup/correction merge failure | 自分の勤怠一覧取得 |
| `/api/attendance/me/summary` | `GET` | User | Query `year?`, `month?` | `200 AttendanceSummary { month, year, total_work_hours, total_work_days, average_daily_hours }` | `400` invalid year/month, `500` lookup failure | 月次勤怠サマリー取得 |
| `/api/attendance/{id}/breaks` | `GET` | User | Path `id` | `200 [BreakRecordResponse]` | `400` invalid attendance id, `403` 他人の attendance, `404` not found, `500` lookup failure | 特定 attendance の休憩一覧取得 |
| `/api/attendance/export` | `GET` | User | Query `from?`, `to?` | `200 {"csv_data":string,"filename":string}` | `400` `from > to`, `500` export failure | 自分の勤怠を CSV 用文字列で export する |

## User Attendance Corrections

| Endpoint | Method | Auth | Parameters | Success Response | Primary Errors | Summary |
| --- | --- | --- | --- | --- | --- | --- |
| `/api/attendance-corrections` | `POST` | User | Body `CreateAttendanceCorrectionRequest { date, clock_in_time?, clock_out_time?, breaks?, reason }` | `200 AttendanceCorrectionResponse` | `400` reason 不正, snapshot 不正, 変更なし, `404` 対象 attendance なし, `500` create failure | 勤怠修正申請を作成する |
| `/api/attendance-corrections/me` | `GET` | User | なし | `200 [AttendanceCorrectionResponse]` | `500` lookup/serialization failure | 自分の勤怠修正申請一覧取得 |
| `/api/attendance-corrections/{id}` | `GET` / `PUT` / `DELETE` | User | `GET`: Path `id`; `PUT`: Path `id` + Body `UpdateAttendanceCorrectionRequest { clock_in_time?, clock_out_time?, breaks?, reason }`; `DELETE`: Path `id` | `GET 200 AttendanceCorrectionResponse`; `PUT 200 AttendanceCorrectionResponse`; `DELETE 200 {"id":id,"status":"cancelled"}` | `404` not found, `400` invalid snapshot or reason, `409` pending 以外は更新不可, `500` update/cancel failure | 自分の勤怠修正申請の取得 / 更新 / 取消 |

## User Requests / Compliance / Holidays

| Endpoint | Method | Auth | Parameters | Success Response | Primary Errors | Summary |
| --- | --- | --- | --- | --- | --- | --- |
| `/api/requests/leave` | `POST` | User | Body `CreateLeaveRequest { leave_type, start_date, end_date, reason? }` | `200 LeaveRequestResponse` | `400` validation, date range error, `500` create failure | 休暇申請作成 |
| `/api/requests/overtime` | `POST` | User | Body `CreateOvertimeRequest { date, planned_hours, reason? }` | `200 OvertimeRequestResponse` | `400` validation, `500` create failure | 残業申請作成 |
| `/api/requests/me` | `GET` | User | なし | `200 {"leave_requests":[...],"overtime_requests":[...],"attendance_corrections":[...]}` | `500` lookup or correction serialization failure | 自分の申請一覧を横断取得 |
| `/api/requests/{id}` | `PUT` / `DELETE` | User | `PUT`: Path `id` + leave/overtime/correction 用 JSON; `DELETE`: Path `id` | `PUT 200 {"message":"Leave request updated" | "Overtime request updated" | "Attendance correction request updated"}`; `DELETE 200 {"id":id,"status":"cancelled"}` | `400` invalid id/payload, pending 以外, validation error, `404` not found or not cancellable, `500` update/cancel failure | leave / overtime / attendance correction の pending 申請を更新または取消する |
| `/api/consents` | `POST` | User | Body `RecordConsentPayload { purpose, policy_version }` | `200 ConsentLogResponse` | legacy `400 {"error":"..."}`, `500 {"error":"Database error"}` | 同意ログを記録する |
| `/api/consents/me` | `GET` | User | なし | `200 [ConsentLogResponse]` | legacy `500 {"error":"Database error"}` | 自分の同意履歴取得 |
| `/api/subject-requests` | `POST` | User | Body `CreateDataSubjectRequest { request_type, details? }` | `200 DataSubjectRequestResponse` | legacy `400 {"error":"details is too long"}`, `500 {"error":"Database error"}` | 個人情報開示等の data subject request 作成 |
| `/api/subject-requests/me` | `GET` | User | なし | `200 [DataSubjectRequestResponse]` | legacy `500 {"error":"Database error"}` | 自分の data subject request 一覧取得 |
| `/api/subject-requests/{id}` | `DELETE` | User | Path `id` | `200 {"id":id,"status":"cancelled"}` | legacy `404 {"error":"Request not found or not cancellable"}`, `500 {"error":"Database error"}` | 自分の data subject request を取り消す |
| `/api/holidays` | `GET` | User | なし | `200 [HolidayResponse]` | `500` lookup failure | 公休日一覧取得 |
| `/api/holidays/check` | `GET` | User | Query `date` | `200 {"is_holiday":bool,"reason"?:string}` | `500` holiday service failure | 指定日が休日か判定する |
| `/api/holidays/month` | `GET` | User | Query `year`, `month` | `200 [{"date": "...", "reason": "..."}]` | `400` month 範囲外, `500` holiday service failure | 指定月の休日一覧取得 |

## Admin Workflow / Audit / Holiday / Export

| Endpoint | Method | Auth | Parameters | Success Response | Primary Errors | Summary |
| --- | --- | --- | --- | --- | --- | --- |
| `/api/admin/requests` | `GET` | Admin | Query `RequestListQuery { status?, type?, user_id?, from?, to?, page?, per_page? }` | `200 AdminRequestListResponse { leave_requests, overtime_requests, page_info }` | `400` invalid pagination/date filter, `403` forbidden, `500` lookup failure | leave / overtime 申請一覧取得 |
| `/api/admin/requests/{id}` | `GET` | Admin | Path `id` | `200 {"kind":"leave"|"overtime","data":...}` | `404` not found, `403` forbidden, `500` lookup failure | 個別申請詳細取得 |
| `/api/admin/requests/{id}/approve` | `PUT` | Admin | Path `id`; Body `ApprovePayload { comment }` | `200 {"message":"Request approved"}` | `400` comment 不正, `403` self-approval 禁止, `404` not found/already processed, `500` update failure | leave / overtime 申請承認 |
| `/api/admin/requests/{id}/reject` | `PUT` | Admin | Path `id`; Body `RejectPayload { comment }` | `200 {"message":"Request rejected"}` | `400` comment 不正, `403` self-reject 禁止, `404` not found/already processed, `500` update failure | leave / overtime 申請却下 |
| `/api/admin/attendance-corrections` | `GET` | Admin | Query `status?`, `user_id?`, `page?`, `per_page?` | `200 [AttendanceCorrectionResponse]` | `400` invalid user_id, `403` forbidden, `500` lookup failure | 勤怠修正申請一覧取得 |
| `/api/admin/attendance-corrections/{id}` | `GET` | Admin | Path `id` | `200 AttendanceCorrectionResponse` | `404` not found, `403` forbidden, `500` lookup failure | 勤怠修正申請詳細取得 |
| `/api/admin/attendance-corrections/{id}/approve` | `PUT` | Admin | Path `id`; Body `DecisionPayload { comment }` | `200 {"message":"Request approved"}` | `400` comment 不正, `403` self-approval 禁止, `404` not found, `500` apply failure | 勤怠修正を承認し effective value を反映する |
| `/api/admin/attendance-corrections/{id}/reject` | `PUT` | Admin | Path `id`; Body `DecisionPayload { comment }` | `200 {"message":"Request rejected"}` | `400` comment 不正, `403` self-reject 禁止, `404` not found, `500` reject failure | 勤怠修正申請を却下する |
| `/api/admin/subject-requests` | `GET` | Admin | Query `SubjectRequestListQuery { status?, type?, user_id?, from?, to?, page?, per_page? }` | `200 SubjectRequestListResponse { page, per_page, total, items }` | `400` filter 不正, `403` forbidden, `500` lookup failure | data subject request 一覧取得 |
| `/api/admin/subject-requests/{id}/approve` | `PUT` | Admin | Path `id`; Body `DecisionPayload { comment }` | `200 {"message":"Subject request approved"}` | `400` comment/filter error, `404` not found/already processed, `500` update failure | data subject request 承認 |
| `/api/admin/subject-requests/{id}/reject` | `PUT` | Admin | Path `id`; Body `DecisionPayload { comment }` | `200 {"message":"Subject request rejected"}` | `400` comment/filter error, `404` not found/already processed, `500` update failure | data subject request 却下 |
| `/api/admin/audit-logs` | `GET` | Admin | Query `AuditLogListQuery { from?, to?, actor_id?, actor_type?, event_type?, target_type?, target_id?, result?, page?, per_page? }` | `200 AuditLogListResponse` + header `X-PII-Masked` | `400` filter 不正, `403` permission 不足, `500` lookup failure | 監査ログ一覧取得 |
| `/api/admin/audit-logs/export` | `GET` | Admin | Query `AuditLogExportQuery { from, to, actor_id?, actor_type?, event_type?, target_type?, target_id?, result? }` | `200 application/json` download + headers `Content-Disposition`, `X-PII-Masked`, `X-Truncated` | `400` filter 不正, `403` permission 不足, `500` export failure | 監査ログ export |
| `/api/admin/audit-logs/{id}` | `GET` | Admin | Path `id` | `200 AuditLogResponse` + header `X-PII-Masked` | `400` invalid id, `403` permission 不足, `404` not found, `500` lookup failure | 監査ログ詳細取得 |
| `/api/admin/holidays` | `GET` / `POST` | Admin | `GET`: Query `AdminHolidayListQuery { page?, per_page?, type?, from?, to? }`; `POST`: Body `CreateHolidayPayload { holiday_date, name, description? }` | `GET 200 AdminHolidayListResponse`; `POST 200 HolidayResponse` | `400` filter/date/name error, `403` forbidden, `409` duplicate holiday, `500` persistence failure | 公休日一覧取得 / 作成 |
| `/api/admin/holidays/weekly` | `GET` / `POST` | Admin | `GET`: なし; `POST`: Body `CreateWeeklyHolidayPayload { weekday, starts_on, ends_on? }` | `GET 200 [WeeklyHolidayResponse]`; `POST 200 WeeklyHolidayResponse` | `400` weekday/date range/start date rule, `403` forbidden, `500` persistence failure | 週次休日一覧取得 / 作成 |
| `/api/admin/holidays/weekly/{id}` | `DELETE` | Admin | Path `id` | `200 {"message":"Weekly holiday deleted","id":id}` | `400` invalid id, `403` forbidden, `404` not found, `500` delete failure | 週次休日削除 |
| `/api/admin/holidays/{id}` | `DELETE` | Admin | Path `id` | `200 {"message":"Holiday deleted","id":id}` | `400` invalid id, `403` forbidden, `404` not found, `500` delete failure | 公休日削除 |
| `/api/admin/holidays/google` | `GET` | Admin | Query `year?` | `200 [GoogleHolidayCandidate { holiday_date, name, description? }]` | `403` forbidden, `500` Google Calendar fetch/parse failure | Google 祝日カレンダーから import 候補を取得する |
| `/api/admin/users/{user_id}/holiday-exceptions` | `GET` / `POST` | Admin | `GET`: Path `user_id` + Query `from?`, `to?`; `POST`: Path `user_id` + Body `CreateHolidayExceptionPayload { exception_date, reason? }` | `GET 200 [HolidayExceptionResponse]`; `POST 201 HolidayExceptionResponse` | `400` invalid user/date, `403` forbidden, `404` user not found, `409` duplicate exception, `500` service failure | 特定ユーザー向け休日例外の一覧取得 / 作成 |
| `/api/admin/users/{user_id}/holiday-exceptions/{id}` | `DELETE` | Admin | Path `user_id`, `id` | `204 No Content` | `400` invalid id, `403` forbidden, `404` not found, `500` service failure | 特定ユーザー向け休日例外削除 |
| `/api/admin/export` | `GET` | Admin | Query `ExportQuery { username?, from?, to? }` | `200 {"csv_data":string,"filename":string}` + header `X-PII-Masked` | `400` invalid date range, `403` forbidden, `500` export failure | 勤怠データの管理者向け CSV export |

## Admin / Department Management

| Endpoint | Method | Auth | Parameters | Success Response | Primary Errors | Summary |
| --- | --- | --- | --- | --- | --- | --- |
| `/api/admin/departments` | `GET` | Manager+ | なし | `200 [DepartmentResponse { id, name, parent_id?, created_at, updated_at }]` | `403` forbidden, `500` lookup failure | 部署一覧取得 |
| `/api/admin/departments/{id}` | `GET` | Manager+ | Path `id` | `200 DepartmentResponse` | `403` forbidden, `404` not found, `500` lookup failure | 部署詳細取得 |
| `/api/admin/departments` | `POST` | System Admin | Body `CreateDepartmentPayload { name, parent_id? }` | `200 DepartmentResponse` | `400` name empty, `403` forbidden, `404` parent not found, `500` create failure | 部署作成 |
| `/api/admin/departments/{id}` | `PUT` | System Admin | Path `id`; Body `UpdateDepartmentPayload { name?, parent_id? }` | `200 DepartmentResponse` | `400` name empty, `403` forbidden, `404` not found, `500` update failure | 部署更新 |
| `/api/admin/departments/{id}` | `DELETE` | System Admin | Path `id` | `200 {"message":"Department deleted","id":id}` | `403` forbidden, `404` not found, `409` has child departments, `500` delete failure | 部署削除（子部署あり → 409） |
| `/api/admin/departments/{id}/managers` | `GET` | Manager+ | Path `id` | `200 [DepartmentManagerEntry { department_id, user_id, assigned_at }]` | `403` forbidden, `404` not found, `500` lookup failure | 部署のマネージャー一覧取得 |
| `/api/admin/departments/{id}/managers` | `POST` | System Admin | Path `id`; Body `AssignManagerPayload { user_id }` | `200 DepartmentManagerEntry` | `400` user not found, `403` forbidden, `404` department not found, `409` already assigned, `500` assign failure | 部署にマネージャーを割り当てる |
| `/api/admin/departments/{id}/managers/{uid}` | `DELETE` | System Admin | Path `id`, `uid` | `200 {"message":"Manager removed","department_id":id,"user_id":uid}` | `403` forbidden, `404` not found, `500` remove failure | 部署のマネージャーを解除する |

## Admin / Work Schedule Master

| Endpoint | Method | Auth | Parameters | Success Response | Primary Errors | Summary |
| --- | --- | --- | --- | --- | --- | --- |
| `/api/admin/work-schedules` | `GET` | Manager+ | Query `status?`, `q?`, `page?`, `per_page?` | `200 WorkScheduleListResponse` | `400` filter不正, `403` forbidden, `500` lookup failure | 勤務体系マスタ一覧を取得する |
| `/api/admin/work-schedules` | `POST` | System Admin | Body `CreateWorkScheduleRequest { code, name, description? }` | `201 WorkScheduleResponse` + `Location` | `400` validation, `409 WORK_SCHEDULE_CODE_CONFLICT`, `403` forbidden | 勤務体系マスタを作成する |
| `/api/admin/work-schedules/{id}` | `GET` | Manager+ | Path `id` | `200 WorkScheduleDetailResponse` | `400` invalid id, `403` forbidden, `404` not found | マスタとバージョン概要を取得する |
| `/api/admin/work-schedules/{id}` | `PATCH` | System Admin | Path `id`; Body `UpdateWorkScheduleRequest { name?, description? }` | `200 WorkScheduleResponse` | `400` validation, `403` forbidden, `404` not found | マスタの表示情報を更新する。空descriptionはnullへ変更する |
| `/api/admin/work-schedules/{id}/retire` | `POST` | System Admin | Path `id` | `200 WorkScheduleResponse` | `400` invalid id, `403` forbidden, `404` not found | マスタをretiredにして新規版・割当を禁止する |
| `/api/admin/work-schedules/{id}/versions` | `POST` | System Admin | Path `id`; Body `CreateWorkScheduleVersionRequest { ..., schedule_type? = "fixed", flex_policy? }` | `201 WorkScheduleVersionResponse` + `Location` | `422 INVALID_SCHEDULE_INTERVALS`, `409 WORK_SCHEDULE_RETIRED`, `404` not found | 7曜日を持つdraftバージョンを作成する。`schedule_type="flex"`時は`flex_policy`（清算期間・コアタイム）必須 |
| `/api/admin/work-schedules/{id}/versions/{version_id}` | `GET` | Manager+ | Path `id`, `version_id` | `200 WorkScheduleVersionResponse { ..., schedule_type, flex_policy }` | `400` invalid id, `403` forbidden, `404` not found | 曜日・勤務区間・予定休憩・flex_policy(該当時)を含む版詳細を取得する |
| `/api/admin/work-schedules/{id}/versions/{version_id}` | `PUT` | System Admin | Body `ReplaceWorkScheduleVersionRequest { revision, ..., schedule_type（必須。省略不可。既存flex版のsilent downgrade防止のため`days`同様に明示必須）, flex_policy? }` | `200 WorkScheduleVersionResponse` | `409 REVISION_CONFLICT` / `PUBLISHED_VERSION_IMMUTABLE`, `422` interval不正・`schedule_type`未送信 | 楽観ロック付きでdraft版全体を置換する |
| `/api/admin/work-schedules/{id}/versions/{version_id}` | `DELETE` | System Admin | Path `id`, `version_id` | `204 No Content` | `409 PUBLISHED_VERSION_IMMUTABLE`, `404` not found | draft版を削除する |
| `/api/admin/work-schedules/{id}/versions/{version_id}/publish` | `POST` | System Admin | Path `id`, `version_id` | `200 WorkScheduleVersionResponse` | `409 EFFECTIVE_PERIOD_OVERLAP` / `PUBLISHED_VERSION_IMMUTABLE`, `404` not found | draft版を公開し、以後変更不能にする |
| `/api/admin/work-schedule-assignments` | `GET` | Manager+ | Query `work_schedule_id?`, `department_id?`, `user_id?`, `page?`, `per_page?` | `200 WorkScheduleAssignmentListResponse` | `400` invalid filter, `403` forbidden, `500` lookup failure | 期間付き割当一覧を取得する |
| `/api/admin/work-schedule-assignments` | `POST` | System Admin | Body `WorkScheduleAssignmentRequest` | `201 WorkScheduleAssignmentResponse` + `Location` | `400` invalid target/range, `409 EFFECTIVE_PERIOD_OVERLAP` / `WORK_SCHEDULE_RETIRED` | 全社・部署・従業員への割当を作成する |
| `/api/admin/work-schedule-assignments/bulk` | `POST` | System Admin | Body `BulkWorkScheduleAssignmentRequest { work_schedule_id, targets[], valid_from, valid_until? }` | `200 BulkWorkScheduleAssignmentResponse { created, failed }` | `400` invalid range/empty targets; per-target conflicts in `failed` | 複数targetへ同じ勤務体系割当を一括作成し、target単位の成功/失敗を返す |
| `/api/admin/work-schedule-assignments/{id}` | `DELETE` | System Admin | Path `id` | `204 No Content` | `400` invalid id, `403` forbidden, `404` not found | 割当を削除する |
| `/api/admin/work-schedule-projections/generate` | `POST` | System Admin | Body `GenerateWorkScheduleProjectionsRequest { user_ids[], from, to }` | `200 GenerateWorkScheduleProjectionsResponse` | `400 INVALID_WORK_SCHEDULE`, per-day errors in response | 指定ユーザー・範囲の未来resolved workdayを生成する。未設定日はnot_configuredとして返す |
| `/api/admin/work-schedule-anomalies` | `GET` | Manager+ | Query `user_id?`, `from`, `to` | `200 WorkScheduleAnomalyListResponse` | `400 INVALID_WORK_SCHEDULE`, `403` scope外 | 未設定、予定外勤務、打刻漏れのanomalyを範囲検出する |
| `/api/admin/users/{user_id}/work-schedule-calendar` | `GET` | Scoped Manager+ | Path `user_id`; Query `from`, `to` | `200 WorkScheduleCalendarResponse` | `400 INVALID_WORK_SCHEDULE`, `403` scope外 | resolved workday（`schedule_type`/`core_time_windows`含む）、attendance、anomalyを日別カレンダーとして返す |
| `/api/admin/work-schedule-closures/monthly` | `POST` | System Admin | Body `CloseWorkScheduleMonthRequest { year, month, user_ids?, reason? }` | `200 CloseWorkScheduleMonthResponse { locked_count }` | `400 INVALID_WORK_SCHEDULE`, `500` lock failure | 対象月のresolved workdayをlockし、以後のprojection/override変更をDB triggerで拒否する |
| `/api/work-schedules/me` | `GET` | User | Query `from`, `to` (必須, 最大366日) | `200 ResolvedWorkdayListResponse` | `400 INVALID_WORK_SCHEDULE` (範囲不正), `500` lookup failure | 認証ユーザー本人の解決済み勤務日を範囲取得する。`schedule_type`（`flex`時のみ勤務日の`core_time_windows`を含む）を返す。`expected_work_minutes`はflexでは区間（flex band）の最大稼働可能幅を表し、契約所定時間ではない |
| `/api/admin/users/{user_id}/resolved-workdays` | `GET` | Scoped Manager+ | Path `user_id`; Query `from`, `to` (必須) | `200 ResolvedWorkdayListResponse` | `400 INVALID_WORK_SCHEDULE`, `403` 部署スコープ外, `500` lookup failure | 配下ユーザーの解決済み勤務日を範囲取得する。System Admin は全ユーザー、Manager は配下のみ。`schedule_type`/`core_time_windows`は`/api/work-schedules/me`と同じ意味論 |
| `/api/admin/users/{user_id}/workday-overrides/{date}` | `PUT` | Scoped Manager+ | Path `user_id`, `date` (YYYY-MM-DD); Body `SetWorkdayOverrideRequest { kind, work_schedule_id?, reason }` | `200 WorkdayOverrideResponse` | `400 INVALID_WORK_SCHEDULE` (kind/schedule不整合・reason不正), `403` 部署スコープ外, `409 RESOLVED_WORKDAY_LOCKED` | 日別例外をupsertする。`non_working_day` は schedule なし、`use_schedule` は schedule 必須 |
| `/api/admin/users/{user_id}/workday-overrides/{date}` | `DELETE` | Scoped Manager+ | Path `user_id`, `date` (YYYY-MM-DD) | `204 No Content` | `400 INVALID_WORK_SCHEDULE`, `403` 部署スコープ外, `404` 例外なし, `409 RESOLVED_WORKDAY_LOCKED` | 未ロックの日別例外を削除する |

## Admin / System Admin User & Attendance Operations

| Endpoint | Method | Auth | Parameters | Success Response | Primary Errors | Summary |
| --- | --- | --- | --- | --- | --- | --- |
| `/api/admin/users` | `GET` / `POST` | `GET`: Admin, `POST`: System Admin | `GET`: なし; `POST`: Body `CreateUser { username, password, full_name, email, role, is_system_admin, department_id? }` | `GET 200 [UserResponse]` + header `X-PII-Masked`; `POST 200 UserResponse` | `403` forbidden, `400` validation / duplicate username / password policy, `500` encrypt/hash/create failure | ユーザー一覧取得 / 新規作成 |
| `/api/admin/users/{id}` | `PUT` / `DELETE` | System Admin | `PUT`: Path `id` + Body `UpdateUser { full_name?, email?, role?, is_system_admin? }`; `DELETE`: Path `id` + Query `hard?` | `PUT 200 UserResponse`; `DELETE 200 {"message":"User archived"|"User permanently deleted","user_id":id,"deletion_type":"soft"|"hard"}` | `400` invalid id, email conflict, self delete, `403` forbidden, `404` not found, `500` update/delete failure | ユーザー更新 / soft delete / hard delete |
| `/api/admin/users/{id}/reset-mfa` | `POST` | System Admin | Path `id` | `200 {"message":"MFA reset and active sessions revoked","user_id":id}` | `400` invalid id, `403` forbidden, `404` user not found, `500` reset failure | 指定ユーザーの MFA を reset し active session / access token / refresh token を失効させる |
| `/api/admin/users/{id}/unlock` | `POST` | System Admin | Path `id` | `200 {"message":"User unlocked","user_id":id}` | `400` invalid id, `403` forbidden, `404` user not found, `500` unlock failure | lockout 中ユーザーを解除する |
| `/api/admin/archived-users` | `GET` | System Admin | なし | `200 [ArchivedUserResponse]` | `403` forbidden, `500` lookup failure | archived user 一覧取得 |
| `/api/admin/archived-users/{id}` | `DELETE` | System Admin | Path `id` | `200 {"message":"Archived user permanently deleted","user_id":id}` | `403` forbidden, `404` archived user not found, `500` delete failure | archived user を完全削除する |
| `/api/admin/archived-users/{id}/restore` | `POST` | System Admin | Path `id` | `200 {"message":"User restored","user_id":id}` | `403` forbidden, `404` archived user not found, `400` username/email conflict, `500` restore failure | archived user を復元する |
| `/api/admin/users/{id}/sessions` | `GET` | Admin | Path `id` | `200 [AdminSessionResponse]` | `400` invalid user id, `403` forbidden, `500` lookup failure | 指定ユーザーの active session 一覧取得 |
| `/api/admin/sessions/{id}` | `DELETE` | Admin | Path `id` | `200 {"message":"Session revoked","session_id":id}` | `400` empty id, `403` forbidden, `404` session not found, `500` revoke failure | 任意ユーザー session を revoke する |
| `/api/admin/attendance` | `GET` / `PUT` | System Admin | `GET`: Query `PaginationQuery { limit, offset }`; `PUT`: Body `AdminAttendanceUpsert { user_id, date, clock_in_time, clock_out_time?, breaks? }` | `GET 200 PaginatedResponse<AttendanceResponse>`; `PUT 200 AttendanceResponse` | `400` invalid pagination/date/time/user_id, `403` forbidden, `500` lookup/upsert failure | 全従業員勤怠の一覧取得 / 指定日の勤怠を作成・置換 |
| `/api/admin/breaks/active` | `GET` | System Admin | なし | `200 [ActiveBreakResponse]` | `403` forbidden, `500` lookup failure | 稼働中の休憩一覧取得 |
| `/api/admin/breaks/{id}/force-end` | `PUT` | System Admin | Path `id` | `200 BreakRecordResponse` | `400` invalid id or already ended, `403` forbidden, `404` break not found, `500` update failure | 管理者が休憩を強制終了する |
| `/api/admin/bulk-import/departments` | `POST` | System Admin | Body `BulkImportRequest { csv_data: string }` | `200 BulkImportResponse { imported, failed, errors: [{row, message}] }` | `400` invalid headers/CSV, `403` forbidden, `500` db failure | 部署 CSV 一括インポート（全-or-なし） |
| `/api/admin/bulk-import/users` | `POST` | System Admin | Body `BulkImportRequest { csv_data: string }` | `200 BulkImportResponse { imported, failed, errors: [{row, message}] }` | `400` invalid headers/CSV, `403` forbidden, `500` db failure | ユーザー CSV 一括インポート（全-or-なし） |

## Notes For Future Changes

- 新しい endpoint を追加したら、この文書へ「path / method / auth / params / response / errors / summary」を追加すること。
- 既存 endpoint を削除・rename したら、この文書の該当行を必ず更新または削除すること。
- request / response 型だけ変えた場合も更新対象です。frontend に見える contract が 1 つでも変われば更新が必要です。
- OpenAPI (`backend/src/docs.rs`) も使う場合は、この文書と差分が出ないよう同時更新すること。
