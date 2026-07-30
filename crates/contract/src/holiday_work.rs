use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HolidayWorkBenefit {
    Substitution,
    Compensatory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HolidayWorkRequestStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
pub struct SubmitHolidayWorkRequest {
    pub work_date: NaiveDate,
    pub benefit: HolidayWorkBenefit,
    pub substitute_date: Option<NaiveDate>,
    #[validate(range(min = 1, max = 1440))]
    pub compensatory_minutes: Option<i32>,
    #[validate(length(min = 1, max = 500))]
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema)]
pub struct HolidayWorkDecision {
    #[validate(length(max = 500))]
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
pub struct HolidayWorkListQuery {
    pub status: Option<HolidayWorkRequestStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HolidayWorkRequestResponse {
    pub id: String,
    pub user_id: String,
    pub work_date: NaiveDate,
    pub benefit: HolidayWorkBenefit,
    pub substitute_date: Option<NaiveDate>,
    pub compensatory_minutes: Option<i32>,
    pub status: HolidayWorkRequestStatus,
    pub reason: String,
    pub decision_comment: Option<String>,
    pub decided_by: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
