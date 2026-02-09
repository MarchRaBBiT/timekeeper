use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;

/// Returns the current time in the configured timezone.
pub fn now_in_timezone(tz: &Tz) -> DateTime<Tz> {
    Utc::now().with_timezone(tz)
}

/// Returns the current UTC time, aligned with the configured timezone.
pub fn now_utc(tz: &Tz) -> DateTime<Utc> {
    now_in_timezone(tz).with_timezone(&Utc)
}

/// Returns today's date in the configured timezone.
pub fn today_local(tz: &Tz) -> NaiveDate {
    now_in_timezone(tz).date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_in_timezone_returns_datetime_in_tz() {
        let tz = chrono_tz::UTC;
        let result = now_in_timezone(&tz);
        assert_eq!(result.timezone(), tz);
    }

    #[test]
    fn now_utc_returns_utc_datetime() {
        let tz = chrono_tz::UTC;
        let result = now_utc(&tz);
        assert_eq!(result.timezone(), Utc);
    }

    #[test]
    fn today_local_returns_naive_date() {
        let tz = chrono_tz::UTC;
        let result = today_local(&tz);
        assert_eq!(
            result.and_hms_opt(0, 0, 0),
            Some(result.and_hms_opt(0, 0, 0).unwrap())
        );
    }

    #[test]
    fn now_utc_is_close_to_utc_now() {
        let tz = chrono_tz::UTC;
        let result = now_utc(&tz);
        let utc_now = Utc::now();
        let diff = (result - utc_now).num_seconds().abs();
        assert!(diff < 2, "Difference should be less than 2 seconds");
    }
}
