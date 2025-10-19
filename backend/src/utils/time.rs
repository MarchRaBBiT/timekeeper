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
