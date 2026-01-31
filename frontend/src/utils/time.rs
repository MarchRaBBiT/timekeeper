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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_matches_now_date() {
        let today = today_in_app_tz();
        let now = now_in_app_tz();
        assert_eq!(today, now.date_naive());
    }
}
