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

pub fn format_in_app_tz(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&app_time_zone())
        .format("%Y/%m/%d %H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TimeZoneStatus;

    #[test]
    fn today_matches_now_date() {
        let today = today_in_app_tz();
        let now = now_in_app_tz();
        assert_eq!(today, now.date_naive());
    }

    #[test]
    fn format_in_app_tz_uses_configured_timezone() {
        let _guard = crate::config::acquire_test_serial_lock();
        crate::config::overwrite_time_zone_status_for_test(TimeZoneStatus {
            time_zone: Some("Asia/Tokyo".into()),
            is_fallback: false,
            last_error: None,
            loading: false,
        });
        let dt = chrono::DateTime::parse_from_rfc3339("2026-01-16T12:34:56Z")
            .expect("valid datetime")
            .with_timezone(&chrono::Utc);
        assert_eq!(format_in_app_tz(dt), "2026/01/16 21:34:56");
    }
}
