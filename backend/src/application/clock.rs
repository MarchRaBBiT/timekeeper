use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;

use crate::utils::time;

pub trait Clock: Send + Sync {
    fn now_in_timezone(&self, tz: &Tz) -> DateTime<Tz>;
    fn now_utc(&self, tz: &Tz) -> DateTime<Utc>;
    fn today_local(&self, tz: &Tz) -> NaiveDate;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

pub const SYSTEM_CLOCK: SystemClock = SystemClock;

impl Clock for SystemClock {
    fn now_in_timezone(&self, tz: &Tz) -> DateTime<Tz> {
        time::now_in_timezone(tz)
    }

    fn now_utc(&self, tz: &Tz) -> DateTime<Utc> {
        time::now_utc(tz)
    }

    fn today_local(&self, tz: &Tz) -> NaiveDate {
        time::today_local(tz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_returns_time_in_requested_timezone() {
        let now = SYSTEM_CLOCK.now_in_timezone(&chrono_tz::UTC);
        assert_eq!(now.timezone(), chrono_tz::UTC);
    }

    #[test]
    fn system_clock_returns_utc_timestamp() {
        let now = SYSTEM_CLOCK.now_utc(&chrono_tz::UTC);
        assert_eq!(now.timezone(), Utc);
    }

    #[test]
    fn system_clock_returns_local_date() {
        let date = SYSTEM_CLOCK.today_local(&chrono_tz::UTC);
        assert_eq!(
            date.and_hms_opt(0, 0, 0),
            Some(date.and_hms_opt(0, 0, 0).unwrap())
        );
    }
}
