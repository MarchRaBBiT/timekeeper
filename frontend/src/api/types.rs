use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use timekeeper_contract::attendance::{
    ActiveBreakResponse, AdminAttendanceUpsert, AdminBreakItem, AttendanceResponse,
    AttendanceStatusResponse, AttendanceSummary, BreakRecordResponse, CorrectionBreakItem,
    CreateAttendanceCorrectionRequest, UpdateAttendanceCorrectionRequest,
};
pub use timekeeper_contract::auth::{
    MessageResponse, RequestPasswordResetRequest, ResetPasswordRequest,
};
pub use timekeeper_contract::holidays::{
    AdminHolidayKind, AdminHolidayListItem, AdminHolidayListResponse, CreateHolidayRequest,
    CreateWeeklyHolidayRequest, HolidayCalendarEntry, HolidayCheckResponse, HolidayResponse,
    WeeklyHolidayResponse,
};
pub use timekeeper_contract::organization::{
    AssignManagerRequest, CreateDepartmentRequest, DepartmentResponse, UpdateDepartmentRequest,
};
pub use timekeeper_contract::requests::{
    CreateLeaveRequest, CreateOvertimeRequest, LeaveRequestResponse, OvertimeRequestResponse,
    UpdateLeaveRequest, UpdateOvertimeRequest,
};
pub use timekeeper_contract::subject_requests::{
    CreateDataSubjectRequest, DataSubjectRequestResponse, DataSubjectRequestType,
    SubjectRequestListResponse,
};

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
    #[serde(default)]
    pub password_expiry_warning_days: Option<i64>,
    #[serde(default)]
    pub department_id: Option<String>,
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
pub struct SessionResponse {
    pub id: String,
    pub device_label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSessionResponse {
    pub id: String,
    pub user_id: String,
    pub device_label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentManagerEntry {
    pub department_id: String,
    pub user_id: String,
    pub assigned_at: DateTime<Utc>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department_id: Option<String>,
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
pub struct PiiProtectedResponse<T> {
    pub data: T,
    pub pii_masked: bool,
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
pub struct BulkImportRequest {
    pub csv_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRowError {
    pub row: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportResponse {
    pub imported: usize,
    pub failed: usize,
    pub errors: Vec<ImportRowError>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn serialize_create_leave_request_snake_case_fields() {
        let req = CreateLeaveRequest {
            leave_type: "annual".into(),
            start_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
            acquisition_unit: timekeeper_contract::requests::LeaveAcquisitionUnit::Day,
            start_time: None,
            end_time: None,
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
    fn serialize_create_user_omits_department_id_when_none() {
        let req = CreateUser {
            username: "bob".into(),
            password: "secret".into(),
            full_name: "Bob".into(),
            email: "bob@example.com".into(),
            role: "member".into(),
            is_system_admin: false,
            department_id: None,
        };
        let v = serde_json::to_value(&req).unwrap();
        assert!(
            v.get("department_id").is_none(),
            "department_id should be omitted when None"
        );
        assert_eq!(v["username"], serde_json::json!("bob"));
    }

    #[test]
    fn serialize_create_user_includes_department_id_when_some() {
        let req = CreateUser {
            username: "carol".into(),
            password: "secret".into(),
            full_name: "Carol".into(),
            email: "carol@example.com".into(),
            role: "member".into(),
            is_system_admin: false,
            department_id: Some("dept-42".into()),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(
            v["department_id"],
            serde_json::json!("dept-42"),
            "department_id should be present with correct key when Some"
        );
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
