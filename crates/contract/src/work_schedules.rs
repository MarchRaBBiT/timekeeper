use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkScheduleStatus {
    Active,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkScheduleVersionStatus {
    Draft,
    Published,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PublicHolidayPolicy {
    NonWorking,
    FollowWeeklyPattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DayKind {
    WorkingDay,
    NonWorkingDay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateWorkScheduleRequest {
    #[validate(length(min = 1, max = 50))]
    pub code: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[serde(default)]
    #[validate(length(max = 1000))]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateWorkScheduleRequest {
    #[serde(default)]
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    #[serde(default)]
    #[validate(length(max = 1000))]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleResponse {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: WorkScheduleStatus,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<WorkScheduleResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
pub struct WorkScheduleListQuery {
    pub status: Option<WorkScheduleStatus>,
    pub q: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlannedWorkIntervalInput {
    pub start_time: NaiveTime,
    pub start_day_offset: u8,
    pub end_time: NaiveTime,
    pub end_day_offset: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlannedBreakInput {
    pub start_time: NaiveTime,
    pub start_day_offset: u8,
    pub end_time: NaiveTime,
    pub end_day_offset: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WeekdayRuleInput {
    pub weekday: u8,
    pub day_kind: DayKind,
    #[serde(default)]
    pub work_intervals: Vec<PlannedWorkIntervalInput>,
    #[serde(default)]
    pub planned_breaks: Vec<PlannedBreakInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkScheduleVersionRequest {
    pub effective_from: NaiveDate,
    #[serde(default)]
    pub effective_until: Option<NaiveDate>,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub public_holiday_policy: PublicHolidayPolicy,
    pub late_grace_minutes: i32,
    pub early_leave_grace_minutes: i32,
    pub days: Vec<WeekdayRuleInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReplaceWorkScheduleVersionRequest {
    pub revision: i32,
    pub effective_from: NaiveDate,
    #[serde(default)]
    pub effective_until: Option<NaiveDate>,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub public_holiday_policy: PublicHolidayPolicy,
    pub late_grace_minutes: i32,
    pub early_leave_grace_minutes: i32,
    pub days: Vec<WeekdayRuleInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlannedWorkIntervalResponse {
    pub sequence: i32,
    pub start_time: NaiveTime,
    pub start_day_offset: i16,
    pub end_time: NaiveTime,
    pub end_day_offset: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlannedBreakResponse {
    pub sequence: i32,
    pub start_time: NaiveTime,
    pub start_day_offset: i16,
    pub end_time: NaiveTime,
    pub end_day_offset: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WeekdayRuleResponse {
    pub weekday: i16,
    pub day_kind: DayKind,
    pub expected_work_minutes: i32,
    pub work_intervals: Vec<PlannedWorkIntervalResponse>,
    pub planned_breaks: Vec<PlannedBreakResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleVersionSummary {
    pub id: String,
    pub version_number: i32,
    pub status: WorkScheduleVersionStatus,
    pub effective_from: NaiveDate,
    pub effective_until: Option<NaiveDate>,
    pub revision: i32,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleVersionResponse {
    pub id: String,
    pub work_schedule_id: String,
    pub version_number: i32,
    pub status: WorkScheduleVersionStatus,
    pub effective_from: NaiveDate,
    pub effective_until: Option<NaiveDate>,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub public_holiday_policy: PublicHolidayPolicy,
    pub late_grace_minutes: i32,
    pub early_leave_grace_minutes: i32,
    pub revision: i32,
    pub published_by: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub days: Vec<WeekdayRuleResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleDetailResponse {
    pub schedule: WorkScheduleResponse,
    pub versions: Vec<WorkScheduleVersionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssignmentTarget {
    Organization,
    Department { department_id: String },
    User { user_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleAssignmentRequest {
    pub work_schedule_id: String,
    pub target: AssignmentTarget,
    pub valid_from: NaiveDate,
    #[serde(default)]
    pub valid_until: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleAssignmentResponse {
    pub id: String,
    pub work_schedule_id: String,
    pub target: AssignmentTarget,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
pub struct WorkScheduleAssignmentListQuery {
    pub work_schedule_id: Option<String>,
    pub department_id: Option<String>,
    pub user_id: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleAssignmentListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<WorkScheduleAssignmentResponse>,
}
