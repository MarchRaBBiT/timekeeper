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
    #[serde(default)]
    pub schedule_type: WorkScheduleType,
    #[serde(default)]
    pub flex_policy: Option<FlexPolicyInput>,
    pub days: Vec<WeekdayRuleInput>,
}

/// `schedule_type` は意図的にデフォルトを持たない。この endpoint はバージョンの完全な置換
/// (PUT semantics) であり、省略時にFixedへ暗黙変換すると既存のFlexバージョンを
/// silentにFixedへdowngradeさせてしまうため、常に明示指定を必須とする。
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
    pub schedule_type: WorkScheduleType,
    #[serde(default)]
    pub flex_policy: Option<FlexPolicyInput>,
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
    pub schedule_type: WorkScheduleType,
    pub flex_policy: Option<FlexPolicyResponse>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BulkWorkScheduleAssignmentRequest {
    pub work_schedule_id: String,
    pub targets: Vec<AssignmentTarget>,
    pub valid_from: NaiveDate,
    #[serde(default)]
    pub valid_until: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BulkWorkScheduleAssignmentFailure {
    pub target: AssignmentTarget,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BulkWorkScheduleAssignmentResponse {
    pub created: Vec<WorkScheduleAssignmentResponse>,
    pub failed: Vec<BulkWorkScheduleAssignmentFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedDayKind {
    ScheduledWorkday,
    ScheduledNonWorkingDay,
    PublicHoliday,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkScheduleSource {
    Override,
    User,
    Department,
    Organization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkdayOverrideKind {
    NonWorkingDay,
    UseSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResolvedWorkIntervalResponse {
    pub start_time: NaiveTime,
    pub start_day_offset: u8,
    pub end_time: NaiveTime,
    pub end_day_offset: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResolvedBreakResponse {
    pub start_time: NaiveTime,
    pub start_day_offset: u8,
    pub end_time: NaiveTime,
    pub end_day_offset: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResolvedWorkdayResponse {
    pub id: String,
    pub user_id: String,
    pub work_date: NaiveDate,
    pub work_schedule_id: String,
    pub work_schedule_version_id: String,
    pub source: WorkScheduleSource,
    pub day_kind: ResolvedDayKind,
    pub timezone: String,
    pub workday_boundary: NaiveTime,
    pub expected_work_minutes: i32,
    pub work_intervals: Vec<ResolvedWorkIntervalResponse>,
    pub planned_breaks: Vec<ResolvedBreakResponse>,
    pub schedule_type: WorkScheduleType,
    pub core_time_windows: Vec<CoreTimeWindowResponse>,
    pub resolved_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResolvedWorkdayListResponse {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub items: Vec<ResolvedWorkdayResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
pub struct ResolvedWorkdayRangeQuery {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SetWorkdayOverrideRequest {
    pub kind: WorkdayOverrideKind,
    #[serde(default)]
    pub work_schedule_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkdayOverrideResponse {
    pub id: String,
    pub user_id: String,
    pub work_date: NaiveDate,
    pub kind: WorkdayOverrideKind,
    pub work_schedule_id: Option<String>,
    pub reason: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct GenerateWorkScheduleProjectionsRequest {
    pub user_ids: Vec<String>,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleProjectionError {
    pub user_id: String,
    pub work_date: NaiveDate,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct GenerateWorkScheduleProjectionsResponse {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub requested_users: usize,
    pub projected: usize,
    pub already_locked: usize,
    pub not_configured: usize,
    pub errors: Vec<WorkScheduleProjectionError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkScheduleAnomalyKind {
    ScheduleNotConfigured,
    UnscheduledWork,
    MissingClockIn,
    MissingClockOut,
    LeaveConflict,
    UnapprovedOvertime,
    OvertimeExceedsRequest,
    Late,
    EarlyLeave,
    Absent,
    InsufficientBreak,
    InsufficientRest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleAnomalyResponse {
    pub user_id: String,
    pub work_date: NaiveDate,
    pub kind: WorkScheduleAnomalyKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
pub struct WorkScheduleAnomalyListQuery {
    pub user_id: Option<String>,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleAnomalyListResponse {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub items: Vec<WorkScheduleAnomalyResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleCalendarAttendanceResponse {
    pub id: String,
    pub clock_in_time: Option<chrono::NaiveDateTime>,
    pub clock_out_time: Option<chrono::NaiveDateTime>,
    pub is_unscheduled_work: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleCalendarLeaveResponse {
    pub leave_request_id: String,
    pub leave_type: String,
    pub acquisition_unit: String,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub requested_minutes: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleCalendarDayResponse {
    pub work_date: NaiveDate,
    pub resolved_workday: Option<ResolvedWorkdayResponse>,
    pub attendance: Option<WorkScheduleCalendarAttendanceResponse>,
    pub anomalies: Vec<WorkScheduleAnomalyResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leave: Option<WorkScheduleCalendarLeaveResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WorkScheduleCalendarResponse {
    pub user_id: String,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub days: Vec<WorkScheduleCalendarDayResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CloseWorkScheduleMonthRequest {
    pub year: i32,
    pub month: u32,
    #[serde(default)]
    pub user_ids: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CloseWorkScheduleMonthResponse {
    pub year: i32,
    pub month: u32,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub locked_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OvertimeMonitorStatus {
    Ok,
    Warning,
    Exceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
pub struct OvertimeMonitorQuery {
    pub year: i32,
    pub month: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct OvertimeMonitorUserResponse {
    pub user_id: String,
    pub month_statutory_excess_minutes: i64,
    pub fiscal_year_statutory_excess_minutes: i64,
    pub rolling_average_statutory_excess_minutes: i64,
    pub monthly_status: OvertimeMonitorStatus,
    pub yearly_status: OvertimeMonitorStatus,
    pub rolling_average_status: OvertimeMonitorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct OvertimeMonitorResponse {
    pub year: i32,
    pub month: u32,
    pub fiscal_year_start_month: u32,
    pub items: Vec<OvertimeMonitorUserResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct OvertimeMonitorSettingsRequest {
    pub valid_from: NaiveDate,
    pub fiscal_year_start_month: u32,
    pub monthly_limit_minutes: i64,
    pub yearly_limit_minutes: i64,
    pub rolling_average_limit_minutes: i64,
    pub single_month_absolute_limit_minutes: i64,
    pub warning_ratio_percent: i32,
    #[serde(default)]
    pub overtime_request_tolerance_minutes: i64,
    #[serde(default)]
    pub minimum_rest_minutes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct OvertimeMonitorSettingsResponse {
    pub id: String,
    pub valid_from: NaiveDate,
    pub fiscal_year_start_month: u32,
    pub monthly_limit_minutes: i64,
    pub yearly_limit_minutes: i64,
    pub rolling_average_limit_minutes: i64,
    pub single_month_absolute_limit_minutes: i64,
    pub warning_ratio_percent: i32,
    pub overtime_request_tolerance_minutes: i64,
    pub minimum_rest_minutes: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MonthlyClosingStatus {
    Open,
    SelfConfirmed,
    Approved,
    Closed,
    Reopened,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MonthlyClosingTransitionRequest {
    pub year: i32,
    pub month: u32,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MonthlyClosingWorkflowResponse {
    pub id: String,
    pub user_id: String,
    pub year: i32,
    pub month: u32,
    pub status: MonthlyClosingStatus,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkScheduleType {
    #[default]
    Fixed,
    Flex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SettlementPeriodUnit {
    Monthly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
pub struct SettlementPeriodInput {
    pub unit: SettlementPeriodUnit,
    #[validate(range(min = 1))]
    pub contracted_minutes_per_period: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SettlementPeriodResponse {
    pub unit: SettlementPeriodUnit,
    pub contracted_minutes_per_period: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CoreTimeWindowInput {
    pub weekday: u8,
    pub start_time: NaiveTime,
    pub start_day_offset: u8,
    pub end_time: NaiveTime,
    pub end_day_offset: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CoreTimeWindowResponse {
    pub weekday: i16,
    pub start_time: NaiveTime,
    pub start_day_offset: i16,
    pub end_time: NaiveTime,
    pub end_day_offset: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FlexPolicyInput {
    pub settlement_period: SettlementPeriodInput,
    #[serde(default)]
    pub core_time_windows: Vec<CoreTimeWindowInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FlexPolicyResponse {
    pub settlement_period: SettlementPeriodResponse,
    pub core_time_windows: Vec<CoreTimeWindowResponse>,
}
