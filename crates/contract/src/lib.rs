pub mod attendance {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct ClockInRequest {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub date: Option<NaiveDate>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct ClockOutRequest {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub date: Option<NaiveDate>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct BreakStartRequest {
        pub attendance_id: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct BreakEndRequest {
        pub break_record_id: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    pub struct AttendanceStatusResponse {
        pub status: String,
        pub attendance_id: Option<String>,
        pub active_break_id: Option<String>,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    pub struct BreakRecordResponse {
        pub id: String,
        pub attendance_id: String,
        pub break_start_time: NaiveDateTime,
        pub break_end_time: Option<NaiveDateTime>,
        pub duration_minutes: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    pub struct AttendanceSummary {
        pub month: u32,
        pub year: i32,
        pub total_work_hours: f64,
        pub total_work_days: i32,
        pub average_daily_hours: f64,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct AdminAttendanceUpsert {
        pub user_id: String,
        pub date: String,
        pub clock_in_time: String,
        pub clock_out_time: Option<String>,
        pub breaks: Option<Vec<AdminBreakItem>>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct AdminBreakItem {
        pub break_start_time: String,
        pub break_end_time: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct ActiveBreakResponse {
        pub break_id: String,
        pub attendance_id: String,
        pub user_id: String,
        pub username: String,
        pub full_name: Option<String>,
        pub break_start_time: NaiveDateTime,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum AttendanceCorrectionStatus {
        Pending,
        Approved,
        Rejected,
        Cancelled,
        Conflict,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct CorrectionBreakItem {
        pub break_start_time: NaiveDateTime,
        pub break_end_time: Option<NaiveDateTime>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct AttendanceCorrectionSnapshot {
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
        pub breaks: Vec<CorrectionBreakItem>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct CreateAttendanceCorrectionRequest {
        pub date: NaiveDate,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
        pub breaks: Option<Vec<CorrectionBreakItem>>,
        pub reason: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct UpdateAttendanceCorrectionRequest {
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
        pub breaks: Option<Vec<CorrectionBreakItem>>,
        pub reason: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct AttendanceCorrectionDecisionPayload {
        pub comment: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct AttendanceCorrectionResponse {
        pub id: String,
        pub user_id: String,
        pub attendance_id: String,
        pub date: NaiveDate,
        pub status: AttendanceCorrectionStatus,
        pub reason: String,
        pub original_snapshot: AttendanceCorrectionSnapshot,
        pub proposed_values: AttendanceCorrectionSnapshot,
        pub decision_comment: Option<String>,
        pub approved_by: Option<String>,
        pub approved_at: Option<DateTime<Utc>>,
        pub rejected_by: Option<String>,
        pub rejected_at: Option<DateTime<Utc>>,
        pub cancelled_at: Option<DateTime<Utc>>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }
}

pub mod requests {
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct CreateLeaveRequest {
        pub leave_type: String,
        pub start_date: NaiveDate,
        pub end_date: NaiveDate,
        pub reason: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct UpdateLeaveRequest {
        pub leave_type: String,
        pub start_date: NaiveDate,
        pub end_date: NaiveDate,
        pub reason: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    pub struct CreateOvertimeRequest {
        pub date: NaiveDate,
        pub planned_hours: f64,
        pub reason: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    pub struct UpdateOvertimeRequest {
        pub date: NaiveDate,
        pub planned_hours: f64,
        pub reason: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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
}

pub mod subject_requests {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum DataSubjectRequestType {
        Access,
        Rectify,
        Delete,
        Stop,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct CreateDataSubjectRequest {
        pub request_type: DataSubjectRequestType,
        #[serde(default)]
        pub details: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct SubjectRequestListResponse {
        pub page: i64,
        pub per_page: i64,
        pub total: i64,
        pub items: Vec<DataSubjectRequestResponse>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct SubjectRequestDecisionPayload {
        pub comment: String,
    }
}

pub mod organization {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct DepartmentResponse {
        pub id: String,
        pub name: String,
        #[serde(default)]
        pub parent_id: Option<String>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct CreateDepartmentRequest {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub parent_id: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct UpdateDepartmentRequest {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub parent_id: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct AssignManagerRequest {
        pub user_id: String,
    }
}

pub mod holidays {
    use chrono::{DateTime, NaiveDate, Utc};
    use serde::{Deserialize, Serialize};
    use std::str::FromStr;
    use utoipa::ToSchema;
    use validator::Validate;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum AdminHolidayKind {
        Public,
        Weekly,
        Exception,
    }

    impl AdminHolidayKind {
        pub fn as_str(&self) -> &'static str {
            match self {
                AdminHolidayKind::Public => "public",
                AdminHolidayKind::Weekly => "weekly",
                AdminHolidayKind::Exception => "exception",
            }
        }
    }

    impl FromStr for AdminHolidayKind {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.to_ascii_lowercase().as_str() {
                "public" => Ok(AdminHolidayKind::Public),
                "weekly" => Ok(AdminHolidayKind::Weekly),
                "exception" => Ok(AdminHolidayKind::Exception),
                _ => Err(()),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct CreateHolidayRequest {
        pub holiday_date: NaiveDate,
        pub name: String,
        #[serde(default)]
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct GoogleHolidayCandidate {
        pub holiday_date: NaiveDate,
        pub name: String,
        #[serde(default)]
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct HolidayResponse {
        pub id: String,
        pub holiday_date: NaiveDate,
        pub name: String,
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
    pub struct CreateWeeklyHolidayRequest {
        #[validate(range(min = 0, max = 6))]
        pub weekday: u8,
        pub starts_on: NaiveDate,
        #[serde(default)]
        pub ends_on: Option<NaiveDate>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct WeeklyHolidayResponse {
        pub id: String,
        pub weekday: i16,
        pub starts_on: NaiveDate,
        pub ends_on: Option<NaiveDate>,
        pub enforced_from: NaiveDate,
        pub enforced_to: Option<NaiveDate>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct AdminHolidayListResponse {
        pub page: i64,
        pub per_page: i64,
        pub total: i64,
        pub items: Vec<AdminHolidayListItem>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct HolidayCheckResponse {
        pub is_holiday: bool,
        #[serde(default)]
        pub reason: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct HolidayMonthEntry {
        pub date: NaiveDate,
        pub reason: String,
    }

    pub type HolidayCalendarEntry = HolidayMonthEntry;
}

pub mod auth {
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;
    use validator::{Validate, ValidationError};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
    pub struct RequestPasswordResetRequest {
        #[validate(email(message = "Invalid email address"))]
        pub email: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
    pub struct ResetPasswordRequest {
        #[validate(length(min = 32, message = "Invalid reset token"))]
        pub token: String,
        #[validate(custom(function = "validate_password_strength"))]
        pub new_password: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub struct MessageResponse {
        pub message: String,
    }

    fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
        if password.len() < 8 {
            return Err(ValidationError::new("password_too_short"));
        }

        let has_uppercase = password.chars().any(char::is_uppercase);
        let has_lowercase = password.chars().any(char::is_lowercase);
        let has_digit = password.chars().any(|c| c.is_ascii_digit());

        if !has_uppercase {
            return Err(ValidationError::new("password_missing_uppercase"));
        }
        if !has_lowercase {
            return Err(ValidationError::new("password_missing_lowercase"));
        }
        if !has_digit {
            return Err(ValidationError::new("password_missing_digit"));
        }

        Ok(())
    }
}
