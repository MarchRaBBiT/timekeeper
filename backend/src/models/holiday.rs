use crate::types::{HolidayId, UserId, WeeklyHolidayId};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
pub use timekeeper_contract::holidays::{
    AdminHolidayKind, AdminHolidayListItem, CreateHolidayRequest as CreateHolidayPayload,
    CreateWeeklyHolidayRequest as CreateWeeklyHolidayPayload, GoogleHolidayCandidate,
    HolidayResponse, WeeklyHolidayResponse,
};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Holiday {
    pub id: HolidayId,
    pub holiday_date: NaiveDate,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Holiday {
    pub fn new(holiday_date: NaiveDate, name: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: HolidayId::new(),
            holiday_date,
            name,
            description,
            created_at: now,
            updated_at: now,
        }
    }
}

impl From<Holiday> for HolidayResponse {
    fn from(value: Holiday) -> Self {
        Self {
            id: value.id.to_string(),
            holiday_date: value.holiday_date,
            name: value.name,
            description: value.description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WeeklyHoliday {
    pub id: WeeklyHolidayId,
    pub weekday: i16,
    pub starts_on: NaiveDate,
    pub ends_on: Option<NaiveDate>,
    pub enforced_from: NaiveDate,
    pub enforced_to: Option<NaiveDate>,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WeeklyHoliday {
    pub fn new(
        weekday: u8,
        starts_on: NaiveDate,
        ends_on: Option<NaiveDate>,
        created_by: UserId,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: WeeklyHolidayId::new(),
            weekday: weekday as i16,
            starts_on,
            ends_on,
            enforced_from: starts_on,
            enforced_to: ends_on,
            created_by,
            created_at: now,
            updated_at: now,
        }
    }
}

impl From<WeeklyHoliday> for WeeklyHolidayResponse {
    fn from(value: WeeklyHoliday) -> Self {
        Self {
            id: value.id.to_string(),
            weekday: value.weekday,
            starts_on: value.starts_on,
            ends_on: value.ends_on,
            enforced_from: value.enforced_from,
            enforced_to: value.enforced_to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::str::FromStr;
    use validator::Validate;

    #[test]
    fn admin_holiday_kind_as_str_and_from_str_roundtrip() {
        for (kind, expected) in [
            (AdminHolidayKind::Public, "public"),
            (AdminHolidayKind::Weekly, "weekly"),
            (AdminHolidayKind::Exception, "exception"),
        ] {
            assert_eq!(kind.as_str(), expected);
            assert_eq!(AdminHolidayKind::from_str(expected), Ok(kind));
            assert_eq!(
                AdminHolidayKind::from_str(&expected.to_uppercase()),
                Ok(kind)
            );
        }
        assert!(AdminHolidayKind::from_str("unknown").is_err());
    }

    #[test]
    fn holiday_new_sets_fields_and_timestamps() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 4).expect("valid date");
        let holiday = Holiday::new(date, "Foundation Day".to_string(), Some("desc".to_string()));

        assert_eq!(holiday.holiday_date, date);
        assert_eq!(holiday.name, "Foundation Day");
        assert_eq!(holiday.description.as_deref(), Some("desc"));
        assert_eq!(holiday.created_at, holiday.updated_at);
    }

    #[test]
    fn weekly_holiday_new_sets_enforced_window_from_inputs() {
        let starts_on = NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date");
        let ends_on = NaiveDate::from_ymd_opt(2026, 12, 31);
        let created_by = UserId::new();
        let weekly = WeeklyHoliday::new(1, starts_on, ends_on, created_by);

        assert_eq!(weekly.weekday, 1);
        assert_eq!(weekly.starts_on, starts_on);
        assert_eq!(weekly.ends_on, ends_on);
        assert_eq!(weekly.enforced_from, starts_on);
        assert_eq!(weekly.enforced_to, ends_on);
        assert_eq!(weekly.created_by, created_by);
        assert_eq!(weekly.created_at, weekly.updated_at);
    }

    #[test]
    fn holiday_and_weekly_responses_map_from_models() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 3).expect("valid date");
        let holiday = Holiday::new(date, "Constitution".to_string(), None);
        let holiday_id = holiday.id;
        let holiday_response = HolidayResponse::from(holiday);
        assert_eq!(holiday_response.id, holiday_id.to_string());
        assert_eq!(holiday_response.holiday_date, date);
        assert_eq!(holiday_response.name, "Constitution");

        let weekly = WeeklyHoliday::new(6, date, None, UserId::new());
        let weekly_id = weekly.id;
        let weekly_response = WeeklyHolidayResponse::from(weekly);
        assert_eq!(weekly_response.id, weekly_id.to_string());
        assert_eq!(weekly_response.weekday, 6);
        assert_eq!(weekly_response.starts_on, date);
        assert!(weekly_response.ends_on.is_none());
    }

    #[test]
    fn create_weekly_holiday_payload_rejects_invalid_weekday() {
        let payload = CreateWeeklyHolidayPayload {
            weekday: 7,
            starts_on: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
            ends_on: None,
        };

        assert!(payload.validate().is_err());
    }
}
