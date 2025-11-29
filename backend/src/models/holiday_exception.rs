use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct HolidayException {
    pub id: String,
    pub user_id: String,
    pub exception_date: NaiveDate,
    #[serde(default)]
    #[sqlx(rename = "override")]
    pub is_holiday_override: bool,
    pub reason: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl HolidayException {
    pub fn new(
        user_id: String,
        exception_date: NaiveDate,
        reason: Option<String>,
        created_by: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
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
    pub id: String,
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
        let exception = HolidayException::new(
            "user-1".to_string(),
            NaiveDate::from_ymd_opt(2024, 12, 24).unwrap(),
            Some("オフサイト参加".to_string()),
            "admin-1".to_string(),
        );

        assert_eq!(exception.user_id, "user-1");
        assert_eq!(
            exception.exception_date,
            NaiveDate::from_ymd_opt(2024, 12, 24).unwrap()
        );
        assert!(!exception.is_holiday_override);
        assert!(exception.is_workday());
        assert_eq!(exception.reason.as_deref(), Some("オフサイト参加"));
        assert_eq!(exception.created_by, "admin-1");
    }
}
