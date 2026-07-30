use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, IntoParams)]
pub struct PayrollExportQuery {
    pub year: i32,
    pub month: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PayrollExportedUser {
    pub user_id: String,
    pub revision: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PayrollExportFailedUser {
    pub user_id: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PayrollExportResponse {
    pub year: i32,
    pub month: u32,
    pub exported: Vec<PayrollExportedUser>,
    pub failed: Vec<PayrollExportFailedUser>,
    pub csv: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayrollSnapshot {
    pub employee_id: String,
    pub year: i32,
    pub month: i32,
    pub revision: i32,
    pub worked_minutes: i64,
    pub scheduled_minutes: i64,
    pub statutory_within_minutes: i64,
    pub statutory_excess_minutes: i64,
    pub legal_holiday_minutes: i64,
    pub night_minutes: i64,
    pub absent_days: i32,
    pub paid_leave_days: i32,
    pub paid_leave_half_days: i32,
    pub paid_leave_minutes: i64,
    pub holiday_work_minutes: i64,
    pub substitute_holiday_days: i32,
    pub compensatory_leave_minutes: i64,
}
