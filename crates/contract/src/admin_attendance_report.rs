use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::work_schedules::OvertimeMonitorStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
pub struct AdminAttendanceReportQuery {
    pub year: i32,
    pub month: u32,
    #[serde(default)]
    pub department_id: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

const fn default_page() -> i64 {
    1
}
const fn default_per_page() -> i64 {
    25
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ReportClassification {
    Calculated {
        actual_minutes: i64,
        scheduled_minutes: i64,
        statutory_within_minutes: i64,
        statutory_excess_minutes: i64,
        legal_holiday_minutes: i64,
        night_minutes: i64,
    },
    UnresolvedDays,
    WorkRuleNotConfigured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AdminAttendanceReportItem {
    pub user_id: String,
    pub user_name: String,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub classification: ReportClassification,
    pub late_count: i64,
    pub early_leave_count: i64,
    pub absent_count: i64,
    pub anomaly_count: i64,
    pub overtime_status: OvertimeMonitorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AdminAttendanceReportResponse {
    pub year: i32,
    pub month: u32,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<AdminAttendanceReportItem>,
}
