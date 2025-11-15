use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;

use crate::config;

fn app_time_zone() -> Tz {
    config::current_time_zone()
}

pub fn now_in_app_tz() -> DateTime<Tz> {
    Utc::now().with_timezone(&app_time_zone())
}

pub fn today_in_app_tz() -> NaiveDate {
    now_in_app_tz().date_naive()
}
