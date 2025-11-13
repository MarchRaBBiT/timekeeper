use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Holiday {
    pub id: String,
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
            id: Uuid::new_v4().to_string(),
            holiday_date,
            name,
            description,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHolidayPayload {
    pub holiday_date: NaiveDate,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolidayResponse {
    pub id: String,
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
