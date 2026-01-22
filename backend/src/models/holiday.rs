use crate::types::{HolidayId, UserId, WeeklyHolidayId};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdminHolidayKind {
    Public,
    Weekly,
    Exception,
}

impl AdminHolidayKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdminHolidayKind::Public => "public",
            AdminHolidayKind::Weekly => "weekly",
            AdminHolidayKind::Exception => "exception",
        }
    }
}

impl FromStr for AdminHolidayKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "public" => Ok(AdminHolidayKind::Public),
            "weekly" => Ok(AdminHolidayKind::Weekly),
            "exception" => Ok(AdminHolidayKind::Exception),
            _ => Err(()),
        }
    }
}

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

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminHolidayListItem {
    pub id: String,
    pub kind: AdminHolidayKind,
    pub applies_from: NaiveDate,
    pub applies_to: Option<NaiveDate>,
    pub date: Option<NaiveDate>,
    pub weekday: Option<i16>,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub user_id: Option<String>,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_override: Option<bool>,
}
