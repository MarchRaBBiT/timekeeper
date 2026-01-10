use crate::types::{HolidayExceptionId, UserId};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct HolidayException {
    pub id: HolidayExceptionId,
    pub user_id: UserId,
    pub exception_date: NaiveDate,
    #[serde(default)]
    #[sqlx(rename = "override")]
    pub is_holiday_override: bool,
    pub reason: Option<String>,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl HolidayException {
    pub fn new(
        user_id: UserId,
        exception_date: NaiveDate,
        reason: Option<String>,
        created_by: UserId,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: HolidayExceptionId::new(),
            user_id,
            exception_date,
            is_holiday_override: false,
            reason,
            created_by,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_workday(&self) -> bool {
        !self.is_holiday_override
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateHolidayExceptionPayload {
    pub exception_date: NaiveDate,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HolidayExceptionResponse {
    pub id: HolidayExceptionId,
    pub exception_date: NaiveDate,
    pub is_workday: bool,
    pub reason: Option<String>,
}

impl From<HolidayException> for HolidayExceptionResponse {
    fn from(value: HolidayException) -> Self {
        let is_workday = value.is_workday();
        Self {
            id: value.id,
            exception_date: value.exception_date,
            is_workday,
            reason: value.reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_workday_override() {
        let user_id = UserId::new();
        let admin_id = UserId::new();
        let exception = HolidayException::new(
            user_id,
            NaiveDate::from_ymd_opt(2024, 12, 24).unwrap(),
            Some("オフサイト参加".to_string()),
            admin_id,
        );

        assert_eq!(exception.user_id, user_id);
        assert_eq!(
            exception.exception_date,
            NaiveDate::from_ymd_opt(2024, 12, 24).unwrap()
        );
        assert!(!exception.is_holiday_override);
        assert!(exception.is_workday());
        assert_eq!(exception.reason.as_deref(), Some("オフサイト参加"));
        assert_eq!(exception.created_by, admin_id);
    }
}
