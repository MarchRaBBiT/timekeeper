use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totp_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_label: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: UserResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
    #[serde(default)]
    pub is_system_admin: bool,
    #[serde(default)]
    pub mfa_enabled: bool,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub locked_until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub failed_login_attempts: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaSetupResponse {
    pub secret: String,
    pub otpauth_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaStatusResponse {
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceResponse {
    pub id: String,
    pub user_id: String,
    pub date: NaiveDate,
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
    pub status: String,
    pub total_work_hours: Option<f64>,
    pub break_records: Vec<BreakRecordResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceStatusResponse {
    pub status: String,
    pub attendance_id: Option<String>,
    pub active_break_id: Option<String>,
    pub clock_in_time: Option<NaiveDateTime>,
    pub clock_out_time: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolidayResponse {
    pub id: String,
    pub holiday_date: NaiveDate,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminHolidayListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<AdminHolidayListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdminHolidayKind {
    Public,
    Weekly,
    Exception,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminHolidayListItem {
    pub id: String,
    pub kind: AdminHolidayKind,
    pub applies_from: NaiveDate,
    pub applies_to: Option<NaiveDate>,
    pub date: Option<NaiveDate>,
    pub weekday: Option<i16>,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub user_id: Option<String>,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_override: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHolidayRequest {
    pub holiday_date: NaiveDate,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyHolidayResponse {
    pub id: String,
    pub weekday: i16,
    pub starts_on: NaiveDate,
    pub ends_on: Option<NaiveDate>,
    pub enforced_from: NaiveDate,
    pub enforced_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWeeklyHolidayRequest {
    pub weekday: u8,
    pub starts_on: NaiveDate,
    #[serde(default)]
    pub ends_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolidayCheckResponse {
    pub is_holiday: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolidayCalendarEntry {
    pub date: NaiveDate,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakRecordResponse {
    pub id: String,
    pub attendance_id: String,
    pub break_start_time: NaiveDateTime,
    pub break_end_time: Option<NaiveDateTime>,
    pub duration_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceSummary {
    pub month: u32,
    pub year: i32,
    pub total_work_hours: f64,
    pub total_work_days: i32,
    pub average_daily_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLeaveRequest {
    pub leave_type: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequestResponse {
    pub id: String,
    pub user_id: String,
    pub leave_type: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub reason: Option<String>,
    pub status: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub rejected_by: Option<String>,
    pub rejected_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub decision_comment: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOvertimeRequest {
    pub date: NaiveDate,
    pub planned_hours: f64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvertimeRequestResponse {
    pub id: String,
    pub user_id: String,
    pub date: NaiveDate,
    pub planned_hours: f64,
    pub reason: Option<String>,
    pub status: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub rejected_by: Option<String>,
    pub rejected_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub decision_comment: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataSubjectRequestType {
    Access,
    Rectify,
    Delete,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDataSubjectRequest {
    pub request_type: DataSubjectRequestType,
    #[serde(default)]
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSubjectRequestResponse {
    pub id: String,
    pub user_id: String,
    pub request_type: DataSubjectRequestType,
    pub status: String,
    pub details: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_by: Option<String>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub decision_comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectRequestListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<DataSubjectRequestResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAttendanceUpsert {
    pub user_id: String,
    pub date: NaiveDate,
    pub clock_in_time: NaiveDateTime,
    pub clock_out_time: Option<NaiveDateTime>,
    pub breaks: Option<Vec<AdminBreakItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminBreakItem {
    pub break_start_time: NaiveDateTime,
    pub break_end_time: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
    pub full_name: String,
    pub email: String,
    pub role: String,
    #[serde(default)]
    pub is_system_admin: bool,
}

use leptos::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiError {
    pub error: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl From<ApiError> for String {
    fn from(error: ApiError) -> Self {
        error.error
    }
}

impl IntoView for ApiError {
    fn into_view(self) -> View {
        self.error.into_view()
    }
}

impl ApiError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "VALIDATION_ERROR".to_string(),
            details: None,
        }
    }

    pub fn unknown(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "UNKNOWN".to_string(),
            details: None,
        }
    }

    pub fn request_failed(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "REQUEST_FAILED".to_string(),
            details: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Option<String>,
    pub actor_type: String,
    pub event_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub result: String,
    pub error_code: Option<String>,
    pub metadata: Option<Value>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<AuditLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedUserResponse {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
    #[serde(default)]
    pub is_system_admin: bool,
    pub archived_at: String,
    pub archived_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn serialize_create_leave_request_snake_case_fields() {
        let req = CreateLeaveRequest {
            leave_type: "annual".into(),
            start_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
            reason: None,
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["leave_type"], serde_json::json!("annual"));
        assert_eq!(v["start_date"], serde_json::json!("2025-01-02"));
        assert_eq!(v["end_date"], serde_json::json!("2025-01-03"));
        assert!(v.get("reason").is_some());
        assert!(v["reason"].is_null());
    }

    #[wasm_bindgen_test]
    fn deserialize_login_response_role_snake_case() {
        let raw = r#"{
            "user": { "id": "u1", "username": "bob", "full_name": "Bob", "role": "admin" }
        }"#;
        let lr: LoginResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(lr.user.role, "admin");
        assert_eq!(lr.user.username, "bob");
    }

    #[wasm_bindgen_test]
    fn serialize_create_weekly_holiday_request_includes_optional_fields() {
        let request = CreateWeeklyHolidayRequest {
            weekday: 2,
            starts_on: NaiveDate::from_ymd_opt(2025, 1, 8).unwrap(),
            ends_on: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["weekday"], serde_json::json!(2));
        assert_eq!(json["starts_on"], serde_json::json!("2025-01-08"));
        assert!(json.get("ends_on").is_some());
        assert!(json["ends_on"].is_null());
    }

    #[wasm_bindgen_test]
    fn deserialize_holiday_calendar_entry() {
        let raw = r#"{"date":"2025-01-01","reason":"public holiday"}"#;
        let entry: HolidayCalendarEntry = serde_json::from_str(raw).unwrap();
        assert_eq!(entry.date, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        assert_eq!(entry.reason, "public holiday");
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use leptos::IntoView;

    #[test]
    fn api_error_helpers_set_expected_codes() {
        let validation = ApiError::validation("invalid payload");
        assert_eq!(validation.code, "VALIDATION_ERROR");
        assert_eq!(validation.error, "invalid payload");
        assert!(validation.details.is_none());

        let unknown = ApiError::unknown("something failed");
        assert_eq!(unknown.code, "UNKNOWN");

        let request_failed = ApiError::request_failed("network error");
        assert_eq!(request_failed.code, "REQUEST_FAILED");
    }

    #[test]
    fn api_error_display_and_string_conversion_match_error_text() {
        let error = ApiError::unknown("boom");
        assert_eq!(format!("{}", error), "boom");

        let raw: String = ApiError::validation("bad input").into();
        assert_eq!(raw, "bad input");
    }

    #[test]
    fn api_error_can_be_converted_to_view() {
        let _: View = ApiError::request_failed("request failed").into_view();
    }

    #[test]
    fn deserialize_admin_holiday_list_item_with_all_optional_fields() {
        let raw = serde_json::json!({
            "id": "holiday-1",
            "kind": "public",
            "applies_from": "2026-01-01",
            "applies_to": "2026-01-31",
            "date": "2026-01-11",
            "weekday": 0,
            "starts_on": "2026-01-01",
            "ends_on": "2026-01-31",
            "name": "National Day",
            "description": "Official holiday",
            "user_id": null,
            "reason": "public",
            "created_by": "admin-1",
            "created_at": "2026-01-01T00:00:00Z",
            "is_override": false
        });
        let item: AdminHolidayListItem = serde_json::from_value(raw).unwrap();
        assert_eq!(item.id, "holiday-1");
        assert_eq!(item.kind, AdminHolidayKind::Public);
        assert_eq!(item.weekday, Some(0));
        assert_eq!(item.is_override, Some(false));
    }

    #[test]
    fn serialize_and_deserialize_subject_request_types() {
        let payload = CreateDataSubjectRequest {
            request_type: DataSubjectRequestType::Delete,
            details: Some("erase all records".into()),
        };
        let value = serde_json::to_value(&payload).unwrap();
        assert_eq!(value["request_type"], serde_json::json!("delete"));
        assert_eq!(value["details"], serde_json::json!("erase all records"));

        let item: DataSubjectRequestResponse = serde_json::from_value(serde_json::json!({
            "id": "sr-1",
            "user_id": "u1",
            "request_type": "access",
            "status": "pending",
            "details": null,
            "approved_by": null,
            "approved_at": null,
            "rejected_by": null,
            "rejected_at": null,
            "cancelled_at": null,
            "decision_comment": null,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }))
        .unwrap();
        assert_eq!(item.request_type, DataSubjectRequestType::Access);
    }

    #[test]
    fn deserialize_attendance_status_and_break_record() {
        let status: AttendanceStatusResponse = serde_json::from_value(serde_json::json!({
            "status": "clocked_in",
            "attendance_id": "att-1",
            "active_break_id": "break-1",
            "clock_in_time": "2026-01-10T09:00:00",
            "clock_out_time": null
        }))
        .unwrap();
        assert_eq!(status.status, "clocked_in");
        assert_eq!(status.attendance_id.as_deref(), Some("att-1"));

        let break_record: BreakRecordResponse = serde_json::from_value(serde_json::json!({
            "id": "break-1",
            "attendance_id": "att-1",
            "break_start_time": "2026-01-10T12:00:00",
            "break_end_time": "2026-01-10T12:30:00",
            "duration_minutes": 30
        }))
        .unwrap();
        assert_eq!(break_record.duration_minutes, Some(30));
    }
}
