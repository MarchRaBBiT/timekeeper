use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
pub struct SettlementBalanceQueryParams {
    pub year: i32,
    pub month: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SettlementBalanceDayResponse {
    pub work_date: NaiveDate,
    pub actual_minutes: i64,
    pub locked: bool,
    pub in_progress: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SettlementBalanceResponse {
    Calculated {
        year: i32,
        month: u32,
        contracted_minutes: i64,
        actual_minutes: i64,
        balance_minutes: i64,
        days: Vec<SettlementBalanceDayResponse>,
    },
    UnresolvedDays,
    NotApplicable,
    VersionMixed,
    NotConfigured,
}
