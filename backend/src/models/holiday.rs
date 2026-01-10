use crate::types::{HolidayId, UserId, WeeklyHolidayId};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateHolidayPayload {
    pub holiday_date: NaiveDate,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HolidayResponse {
    pub id: HolidayId,
    pub holiday_date: NaiveDate,
    pub name: String,
    pub description: Option<String>,
}

impl From<Holiday> for HolidayResponse {
    fn from(value: Holiday) -> Self {
        Self {
            id: value.id,
            holiday_date: value.holiday_date,
            name: value.name,
            description: value.description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWeeklyHolidayPayload {
    pub weekday: u8,
    pub starts_on: NaiveDate,
    #[serde(default)]
    pub ends_on: Option<NaiveDate>,
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WeeklyHolidayResponse {
    pub id: WeeklyHolidayId,
    pub weekday: u8,
    pub starts_on: NaiveDate,
    pub ends_on: Option<NaiveDate>,
    pub enforced_from: NaiveDate,
    pub enforced_to: Option<NaiveDate>,
}

impl From<WeeklyHoliday> for WeeklyHolidayResponse {
    fn from(value: WeeklyHoliday) -> Self {
        Self {
            id: value.id,
            weekday: value.weekday as u8,
            starts_on: value.starts_on,
            ends_on: value.ends_on,
            enforced_from: value.enforced_from,
            enforced_to: value.enforced_to,
        }
    }
}
